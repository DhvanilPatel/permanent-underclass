# Permanent Underclass NPM Release SOT

Last updated: 2026-02-24
Status: canonical; replaces older release notes.

## What Is Canonical

1. This file is the only source of truth for npm release flow.
2. npm publish is tag-driven (not every commit).
3. Two channels:
   - `beta` for prereleases (`vX.Y.Z-beta.N`)
   - `latest` for stable (`vX.Y.Z`)
4. Wrapper + selected platform packages must share the same exact version.

## Package Topology

- Wrapper: `permanent-underclass`
- Platform packages:
  - `permanent-underclass-darwin-arm64`
  - `permanent-underclass-darwin-x64`
  - `permanent-underclass-linux-x64`
  - `permanent-underclass-linux-arm64`

Publish order is always platforms first, wrapper last.

## Current CI Behavior (As Configured)

Release workflow: `.github/workflows/release-npm.yml`

1. Triggers on pushed tags matching `v*`.
2. Derives npm version from tag name.
3. Uses dist-tag:
   - prerelease tag -> `beta`
   - stable tag -> `latest`
4. Runs:
   - `scripts/prepare_npm_release.sh <version>`
   - `scripts/publish_npm_release.sh <version> <dist-tag>`
5. Current platform scope in CI is intentionally:
   - `darwin-arm64`
   - `darwin-x64`
   - `linux-x64`
6. Current provenance setting in CI is:
   - `PU_NPM_PROVENANCE=true`
   - Publish runs are expected from a public GitHub repo with npm trusted publishing configured.

## Release Procedure

1. Merge the intended release state to `main`.
2. Create and push a tag:

```bash
# beta
git tag v0.2.0-beta.1
git push origin v0.2.0-beta.1

# stable
git tag v0.2.0
git push origin v0.2.0
```

3. Wait for `Release NPM` workflow to complete.
4. Smoke test from registry:

```bash
# beta
npx --yes permanent-underclass@beta --headless --character junior-dev --seed 1

# stable
npx --yes permanent-underclass@latest --headless --character junior-dev --seed 1
```

## Local Script Contract

Canonical scripts:
- `scripts/prepare_npm_release.sh`
- `scripts/publish_npm_release.sh`

Expected behavior:
1. Validate version consistency across wrapper + selected platform packages.
2. Refuse publish if any target version already exists on npm.
3. Publish platforms first, wrapper last.
4. Support explicit platform subset via `PU_NPM_PLATFORMS`.

## Provenance Requirements

1. Repository visibility must be `public`.
2. npm trusted publisher must be configured for this repository/workflow.
3. Publish from GitHub Actions tag runs (`Release NPM`) so OIDC provenance is emitted.
4. If a publish was done without provenance, publish a new higher version tag (npm versions are immutable).

## Rules

1. Never retag a published version.
2. If a release is wrong, publish a new higher version.
3. Keep release history explicit through tags.
