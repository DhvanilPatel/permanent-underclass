#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const binPath = path.join(__dirname, "bin", "permanent-underclass");

try {
  fs.chmodSync(binPath, 0o755);
} catch (_err) {
  // Best-effort only: some environments may not permit chmod.
}
