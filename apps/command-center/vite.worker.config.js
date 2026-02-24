import path from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  root: __dirname,
  base: '/command-center/',
  build: {
    outDir: path.resolve(__dirname, '../../dist/command-center-worker/command-center'),
    emptyOutDir: true,
  },
});

