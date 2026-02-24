#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <version>"
  echo "Example (beta):  $0 0.2.0-beta.1"
  echo "Example (stable): $0 0.2.0"
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WRAPPER_DIR="$ROOT/npm/permanent-underclass"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
SOURCE_SFX_DIR="$ROOT/data/sfx"
SOURCE_DATA_DIR="$ROOT/data"
DEFAULT_PLATFORMS=("darwin-arm64" "darwin-x64" "linux-x64" "linux-arm64")
PLATFORM_KEYS=()
SELECTED_PLATFORM_KEYS=()
PLATFORM_PKG_NAMES=()
PLATFORM_DIRS=()
PLATFORM_TARGETS=()
STAGED_BIN_PATHS=()

platform_target_triple() {
  case "$1" in
    darwin-arm64) echo "aarch64-apple-darwin" ;;
    darwin-x64) echo "x86_64-apple-darwin" ;;
    linux-x64) echo "x86_64-unknown-linux-gnu" ;;
    linux-arm64) echo "aarch64-unknown-linux-gnu" ;;
    *)
      echo "Unsupported platform key: $1" >&2
      exit 1
      ;;
  esac
}

platform_package_name() {
  echo "permanent-underclass-$1"
}

resolve_platforms() {
  local raw="${PU_NPM_PLATFORMS:-}"
  if [[ -z "$raw" ]]; then
    PLATFORM_KEYS=("${DEFAULT_PLATFORMS[@]}")
    return
  fi

  IFS=',' read -r -a PLATFORM_KEYS <<< "$raw"
  if [[ ${#PLATFORM_KEYS[@]} -eq 0 ]]; then
    echo "PU_NPM_PLATFORMS is set but empty." >&2
    exit 1
  fi
}

resolve_platforms

for key in "${PLATFORM_KEYS[@]}"; do
  key="$(echo "$key" | xargs)"
  if [[ -z "$key" ]]; then
    continue
  fi
  pkg_name="$(platform_package_name "$key")"
  pkg_dir="$ROOT/npm/$pkg_name"
  target_triple="$(platform_target_triple "$key")"

  if [[ ! -d "$pkg_dir" ]]; then
    echo "Missing platform package directory: $pkg_dir" >&2
    exit 1
  fi

  SELECTED_PLATFORM_KEYS+=("$key")
  PLATFORM_PKG_NAMES+=("$pkg_name")
  PLATFORM_DIRS+=("$pkg_dir")
  PLATFORM_TARGETS+=("$target_triple")
done

if [[ ${#PLATFORM_PKG_NAMES[@]} -eq 0 ]]; then
  echo "No platform packages selected." >&2
  exit 1
fi

echo "Preparing npm release assets for version: $VERSION"
echo "Platforms: ${SELECTED_PLATFORM_KEYS[*]}"

node -e '
const fs = require("node:fs");
const pkgPath = process.argv[1];
const version = process.argv[2];
const packageNames = process.argv[3].split(",").map((s) => s.trim()).filter(Boolean);
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
pkg.version = version;
pkg.optionalDependencies = {};
for (const name of packageNames) {
  pkg.optionalDependencies[name] = version;
}
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
' "$WRAPPER_DIR/package.json" "$VERSION" "$(IFS=,; echo "${PLATFORM_PKG_NAMES[*]}")"

for pkg_dir in "${PLATFORM_DIRS[@]}"; do
  node -e '
const fs = require("node:fs");
const pkgPath = process.argv[1];
const version = process.argv[2];
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
pkg.version = version;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
' "$pkg_dir/package.json" "$VERSION"
done

if [[ "${PU_NPM_SKIP_BUILD:-0}" == "1" ]]; then
  echo "PU_NPM_SKIP_BUILD=1 set. Skipping binary build/staging and npm pack dry-runs."
  exit 0
fi

if [[ ! -d "$SOURCE_SFX_DIR" ]]; then
  echo "Missing source SFX directory: $SOURCE_SFX_DIR" >&2
  exit 1
fi
if [[ ! -d "$SOURCE_DATA_DIR" ]]; then
  echo "Missing source data directory: $SOURCE_DATA_DIR" >&2
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required to verify/build cross targets." >&2
  exit 1
fi

for i in "${!PLATFORM_PKG_NAMES[@]}"; do
  key="${SELECTED_PLATFORM_KEYS[$i]}"
  pkg_dir="${PLATFORM_DIRS[$i]}"
  target_triple="${PLATFORM_TARGETS[$i]}"
  staged_bin="$pkg_dir/bin/permanent-underclass"
  staged_data_dir="$pkg_dir/data"
  staged_sfx_dir="$staged_data_dir/sfx"

  if ! rustup target list --installed | grep -qx "$target_triple"; then
    echo "Missing rust target: $target_triple" >&2
    echo "Install it with: rustup target add $target_triple" >&2
    exit 1
  fi

  echo "Building release binary for $key ($target_triple)..."
  cargo build --release --target "$target_triple" --bin permanent-underclass --manifest-path "$ROOT/Cargo.toml"

  release_bin="$TARGET_DIR/$target_triple/release/permanent-underclass"
  if [[ ! -f "$release_bin" ]]; then
    echo "Expected release binary not found: $release_bin" >&2
    exit 1
  fi

  mkdir -p "$(dirname "$staged_bin")"
  cp "$release_bin" "$staged_bin"
  chmod 755 "$staged_bin"
  STAGED_BIN_PATHS+=("$staged_bin")

  rm -rf "$staged_data_dir"
  mkdir -p "$staged_sfx_dir"
  cp "$SOURCE_SFX_DIR"/*.mp3 "$staged_sfx_dir"/

  while IFS= read -r relpath; do
    relpath="${relpath#./}"
    dest="$staged_data_dir/$relpath"
    mkdir -p "$(dirname "$dest")"
    cp "$SOURCE_DATA_DIR/$relpath" "$dest"
  done < <(cd "$SOURCE_DATA_DIR" && find . -type f -name '*.jsonl' -print)
done

echo "Running npm pack dry-runs..."
for pkg_dir in "${PLATFORM_DIRS[@]}"; do
  npm pack --dry-run "$pkg_dir"
done
npm pack --dry-run "$WRAPPER_DIR"

rm -f "$ROOT"/*.tgz "$WRAPPER_DIR"/*.tgz 2>/dev/null || true
for pkg_dir in "${PLATFORM_DIRS[@]}"; do
  rm -f "$pkg_dir"/*.tgz 2>/dev/null || true
done

echo "Done. Prepared version: $VERSION"
for i in "${!SELECTED_PLATFORM_KEYS[@]}"; do
  echo "[$i] ${SELECTED_PLATFORM_KEYS[$i]} => ${STAGED_BIN_PATHS[$i]}"
done
