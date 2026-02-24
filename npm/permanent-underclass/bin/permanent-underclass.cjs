#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const PLATFORM_MAP = {
  "darwin-arm64": "permanent-underclass-darwin-arm64",
  "darwin-x64": "permanent-underclass-darwin-x64",
  "linux-x64": "permanent-underclass-linux-x64",
  "linux-arm64": "permanent-underclass-linux-arm64"
};

const key = `${process.platform}-${process.arch}`;
const platformPkg = PLATFORM_MAP[key];

if (!platformPkg) {
  console.error(
    [
      "permanent-underclass: unsupported platform for this beta release.",
      `Detected: ${key}`,
      "Supported:",
      "  - darwin-arm64",
      "  - darwin-x64",
      "  - linux-x64",
      "  - linux-arm64",
      "Try again later when more platform packages are published."
    ].join("\n")
  );
  process.exit(1);
}

let pkgJsonPath;
try {
  pkgJsonPath = require.resolve(`${platformPkg}/package.json`);
} catch (_err) {
  console.error(
    [
      `permanent-underclass: missing optional dependency ${platformPkg}.`,
      "If you installed with --omit=optional, reinstall without that flag.",
      "Try:",
      "  npm i -g permanent-underclass@beta",
      "or",
      "  npx --yes permanent-underclass@beta"
    ].join("\n")
  );
  process.exit(1);
}

const pkgRoot = path.dirname(pkgJsonPath);
const binPath = path.join(pkgRoot, "bin", "permanent-underclass");
const sfxRoot = path.join(pkgRoot, "data", "sfx");
const eventsRoot = path.join(pkgRoot, "data", "events");

const AUDIO_ENV_DEFAULTS = [
  ["PU_SELECTION_SFX", "glitch_004.mp3"],
  ["PU_TITLE_TONE", "gamevideo_seamless_looping_tra_elevenlabs.mp3"],
  ["PU_RUNNING_TONE", "gamevideo_seamless_looping_tra_elevenlabs.mp3"],
  ["PU_CHIPTUNE_SCORE", "cutilatk_ringtone_4_387502.mp3"],
  ["PU_TECHNO_SCORE", "lesiakower_8_bit_energetic_drums_chiptune_beat_384593.mp3"]
];

if (!fs.existsSync(binPath)) {
  console.error(`permanent-underclass: binary not found at ${binPath}`);
  process.exit(1);
}

const childEnv = { ...process.env };
for (const [envKey, fileName] of AUDIO_ENV_DEFAULTS) {
  if (childEnv[envKey]) {
    continue;
  }
  const filePath = path.join(sfxRoot, fileName);
  if (fs.existsSync(filePath)) {
    childEnv[envKey] = filePath;
  }
}
if (!childEnv.PU_EVENTS_DIR && fs.existsSync(eventsRoot)) {
  childEnv.PU_EVENTS_DIR = eventsRoot;
}

const result = spawnSync(binPath, process.argv.slice(2), {
  stdio: "inherit",
  env: childEnv
});

if (result.error) {
  console.error(`permanent-underclass: failed to launch binary: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status == null ? 1 : result.status);
