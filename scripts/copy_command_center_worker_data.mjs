import { cpSync, mkdirSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');

const sourceDataDir = path.join(repoRoot, 'data');
const outputRootDir = path.join(repoRoot, 'dist', 'command-center-worker');
const outputDataDir = path.join(outputRootDir, 'data');

mkdirSync(outputRootDir, { recursive: true });
rmSync(outputDataDir, { recursive: true, force: true });
mkdirSync(outputDataDir, { recursive: true });

const copyRelative = (relativePath) => {
  const source = path.join(sourceDataDir, relativePath);
  const target = path.join(outputDataDir, relativePath);
  mkdirSync(path.dirname(target), { recursive: true });
  cpSync(source, target, { recursive: true });
};

// Files loaded by command-center at runtime.
copyRelative('characters.jsonl');
copyRelative('endings.jsonl');
copyRelative('headlines.jsonl');
copyRelative('flags.jsonl');
copyRelative('events');
copyRelative('sim/ending_sweep_latest.jsonl');
copyRelative('sprites/sprite_library.jsonl');
copyRelative('sprites/sprite_library_meta.json');

console.log(`Copied command-center data assets to ${outputDataDir}`);
