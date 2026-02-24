# Command Center App

Web inspector for game content and sprite data.

Run from repo root:

```bash
npm run dev
```

Then open:

```text
http://localhost:5173/command-center
```

Data is read live from repo files under `data/`.

## Cloudflare Worker Hosting

Build static assets for Worker deployment:

```bash
npm run build:command-center:worker
```

Run locally with Wrangler:

```bash
npm run dev:command-center:worker
```

Deploy to Cloudflare Workers:

```bash
npm run deploy:command-center:worker
```

Worker config lives in `apps/command-center/wrangler.toml`.

CI deploy (`.github/workflows/deploy-command-center-worker.yml`) expects these
repository secrets (either naming style works):

- `CF_API_TOKEN` or `CLOUDFLARE_API_TOKEN`
- `CF_ACCOUNT_ID` or `CLOUDFLARE_ACCOUNT_ID`

The API token must be valid for the target account and include Worker deploy
permissions.
