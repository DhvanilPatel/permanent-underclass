import path from 'node:path';
import { defineConfig } from 'vite';

const repoRoot = path.resolve(__dirname, '../..');

export default defineConfig({
  root: repoRoot,
  server: {
    open: '/command-center',
    watch: {
      include: ['apps/command-center/**', 'data/**', 'src/engine/**'],
    },
  },
  plugins: [
    {
      name: 'command-center-route',
      configureServer(server) {
        server.middlewares.use((req, _res, next) => {
          if (req.url === '/command-center' || req.url === '/command-center/') {
            req.url = '/apps/command-center/index.html';
          }
          next();
        });
      },
    },
  ],
});
