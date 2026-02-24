#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
DIST_TAG="${2:-${PU_NPM_DIST_TAG:-latest}}"
if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <version> [dist-tag]"
  echo "Example (beta):  $0 0.2.0-beta.1 beta"
  echo "Example (stable): $0 0.2.0 latest"
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WRAPPER_DIR="$ROOT/npm/permanent-underclass"
DEFAULT_PLATFORMS=("darwin-arm64" "darwin-x64" "linux-x64" "linux-arm64")
PLATFORM_KEYS=()
SELECTED_PLATFORM_KEYS=()
PLATFORM_NAMES=()
PLATFORM_DIRS=()
PLATFORM_PKG_JSON_PATHS=()
WRAPPER_NAME="permanent-underclass"
PUBLISH_PLATFORMS_MODE="${PU_NPM_PUBLISH_PLATFORMS:-1}"
PUBLISH_WRAPPER_MODE="${PU_NPM_PUBLISH_WRAPPER:-1}"

parse_bool() {
  case "$1" in
    1|true|TRUE|yes|YES) echo "1" ;;
    0|false|FALSE|no|NO) echo "0" ;;
    *)
      echo "Invalid boolean value: $1" >&2
      exit 1
      ;;
  esac
}

PUBLISH_PLATFORMS="$(parse_bool "$PUBLISH_PLATFORMS_MODE")"
PUBLISH_WRAPPER="$(parse_bool "$PUBLISH_WRAPPER_MODE")"
if [[ "$PUBLISH_PLATFORMS" != "1" && "$PUBLISH_WRAPPER" != "1" ]]; then
  echo "Nothing to publish: PU_NPM_PUBLISH_PLATFORMS=0 and PU_NPM_PUBLISH_WRAPPER=0" >&2
  exit 1
fi

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
  name="$(platform_package_name "$key")"
  dir="$ROOT/npm/$name"
  if [[ ! -d "$dir" ]]; then
    echo "Missing platform package directory: $dir" >&2
    exit 1
  fi
  SELECTED_PLATFORM_KEYS+=("$key")
  PLATFORM_NAMES+=("$name")
  PLATFORM_DIRS+=("$dir")
  PLATFORM_PKG_JSON_PATHS+=("$dir/package.json")
done

if [[ ${#PLATFORM_NAMES[@]} -eq 0 ]]; then
  echo "No platform packages selected." >&2
  exit 1
fi

# Optional local env support (NPM_TOKEN, NPM_OTP)
if [[ -f "$ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT/.env"
  set +a
fi

NPMRC_PATH=""
cleanup_npmrc() {
  if [[ -n "$NPMRC_PATH" && -f "$NPMRC_PATH" ]]; then
    rm -f "$NPMRC_PATH"
  fi
}
trap cleanup_npmrc EXIT

if [[ -n "${NPM_TOKEN:-}" ]]; then
  NPMRC_PATH="$(mktemp)"
  chmod 600 "$NPMRC_PATH"
  printf "//registry.npmjs.org/:_authToken=%s\n" "$NPM_TOKEN" > "$NPMRC_PATH"
fi

npm_cmd() {
  if [[ -n "$NPMRC_PATH" ]]; then
    npm --userconfig "$NPMRC_PATH" "$@"
  else
    npm "$@"
  fi
}

PROVENANCE_MODE="${PU_NPM_PROVENANCE:-auto}"
PUBLISH_ARGS=(--tag "$DIST_TAG" --access public)
case "$PROVENANCE_MODE" in
  1|true|TRUE|yes|YES)
    PUBLISH_ARGS+=(--provenance)
    ;;
  0|false|FALSE|no|NO)
    PUBLISH_ARGS+=(--provenance=false)
    ;;
  auto)
    if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
      PUBLISH_ARGS+=(--provenance)
    else
      PUBLISH_ARGS+=(--provenance=false)
    fi
    ;;
  *)
    echo "Invalid PU_NPM_PROVENANCE value: $PROVENANCE_MODE" >&2
    exit 1
    ;;
esac

if [[ -n "${NPM_OTP:-}" ]]; then
  PUBLISH_ARGS+=(--otp "$NPM_OTP")
fi

if [[ -z "${GITHUB_ACTIONS:-}" ]]; then
  echo "Checking npm auth..."
  npm_cmd whoami >/dev/null
fi

echo "Release version: $VERSION"
echo "Dist-tag: $DIST_TAG"
echo "Platforms: ${SELECTED_PLATFORM_KEYS[*]}"
echo "Publish platforms: $PUBLISH_PLATFORMS"
echo "Publish wrapper: $PUBLISH_WRAPPER"

echo "Ensuring versions match requested version ($VERSION)..."
node -e '
const fs = require("node:fs");
const [wrapperPath, platformPathsRaw, platformNamesRaw, expected] = process.argv.slice(1);
const platformPaths = platformPathsRaw.split(",").map((s) => s.trim()).filter(Boolean);
const platformNames = platformNamesRaw.split(",").map((s) => s.trim()).filter(Boolean);
if (platformPaths.length !== platformNames.length) {
  throw new Error("platform path/name count mismatch");
}
const wrapper = JSON.parse(fs.readFileSync(wrapperPath, "utf8"));
if (wrapper.version !== expected) {
  throw new Error(`wrapper version mismatch: ${wrapper.version} !== ${expected}`);
}
for (let i = 0; i < platformPaths.length; i += 1) {
  const platform = JSON.parse(fs.readFileSync(platformPaths[i], "utf8"));
  const dep = wrapper.optionalDependencies?.[platformNames[i]];
  if (platform.version !== expected) {
    throw new Error(`platform version mismatch (${platformNames[i]}): ${platform.version} !== ${expected}`);
  }
  if (dep !== expected) {
    throw new Error(`wrapper optionalDependency mismatch (${platformNames[i]}): ${dep} !== ${expected}`);
  }
}
' "$WRAPPER_DIR/package.json" "$(IFS=,; echo "${PLATFORM_PKG_JSON_PATHS[*]}")" "$(IFS=,; echo "${PLATFORM_NAMES[*]}")" "$VERSION"

echo "Checking whether versions already exist on npm..."
if [[ "$PUBLISH_WRAPPER" == "1" ]]; then
  if npm_cmd view "${WRAPPER_NAME}@${VERSION}" version >/dev/null 2>&1; then
    echo "Version already exists: ${WRAPPER_NAME}@${VERSION}" >&2
    exit 1
  fi
fi
if [[ "$PUBLISH_PLATFORMS" == "1" ]]; then
  for name in "${PLATFORM_NAMES[@]}"; do
    if npm_cmd view "${name}@${VERSION}" version >/dev/null 2>&1; then
      echo "Version already exists: ${name}@${VERSION}" >&2
      exit 1
    fi
  done
fi

if [[ "$PUBLISH_PLATFORMS" == "1" ]]; then
  echo "Publishing platform packages first..."
  for i in "${!PLATFORM_NAMES[@]}"; do
    name="${PLATFORM_NAMES[$i]}"
    dir="${PLATFORM_DIRS[$i]}"
    echo "Publishing ${name} (${DIST_TAG})..."
    (cd "$dir" && npm_cmd publish "${PUBLISH_ARGS[@]}")
  done
fi

if [[ "$PUBLISH_WRAPPER" == "1" ]]; then
  echo "Publishing wrapper package..."
  (cd "$WRAPPER_DIR" && npm_cmd publish "${PUBLISH_ARGS[@]}")
fi

echo "Publish complete."
if [[ "$DIST_TAG" == "latest" ]]; then
  echo "Smoke test: npx --yes permanent-underclass --headless --character junior-dev --seed 1"
else
  echo "Smoke test: npx --yes permanent-underclass@${DIST_TAG} --headless --character junior-dev --seed 1"
fi
