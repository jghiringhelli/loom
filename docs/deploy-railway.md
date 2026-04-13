# Deploying the BIOISO Colony to Railway

## Prerequisites
- [Railway CLI](https://docs.railway.com/develop/cli) installed (`npm i -g @railway/cli`)
- Railway account (Hobby plan at $5/month)
- Claude API key (for Tier 3 Mammal Brain synthesis)

## One-time setup

### 1. Login and create project
```bash
railway login
railway init            # creates a new project
```

### 2. Add a persistent volume
In the Railway dashboard:
- Go to your project → **New** → **Volume**
- Name it `bioiso-data`, mount path `/data`
- This persists the SQLite signal store across deploys

### 3. Set environment variables
In the Railway dashboard → your service → **Variables**:

| Variable | Value | Notes |
|---|---|---|
| `CLAUDE_API_KEY` | `sk-ant-...` | Your Claude API key — Tier 3 brain |
| `DB_PATH` | `/data/bioiso.db` | SQLite on the mounted volume |
| `TICK_MS` | `5000` | Tick every 5 seconds |
| `OLLAMA_BASE_URL` | *(leave empty)* | Skip Tier 2; go straight to Claude |
| `RUST_LOG` | `info` | Log verbosity |

> **Security**: Never commit your `CLAUDE_API_KEY`. Railway secrets are encrypted at rest.

### 4. Deploy
```bash
railway up
```

Railway will:
1. Build the multi-stage Docker image (first build ~3-5 min; subsequent ~1 min via layer cache)
2. Mount the volume at `/data`
3. Run `start-colony.sh` which spawns all 11 domain entities then starts the daemon

### 5. Verify deployment
```bash
railway logs --tail 50
```

You should see:
```
bioiso: initialising colony at /data/bioiso.db
  spawning climate (Climate Change Mitigation)...
  spawning epidemics (Epidemic Response)...
  ...
bioiso: colony initialised — starting evolution daemon (tick=5000ms)
bioiso: starting evolution daemon (store=/data/bioiso.db, tick=5000ms). Press Ctrl-C or send EOF to stop.
```

## Monitoring the colony

### Check entity status
```bash
railway run loom runtime status --db /data/bioiso.db
```

### Tail recent signals
```bash
railway run loom runtime log --db /data/bioiso.db --n 20
```

### Spawn a child entity that inherits from a parent
```bash
railway run loom runtime spawn climate-v2 \
  --db /data/bioiso.db \
  --name "Climate v2" \
  --telos '{"target":"net zero by 2050"}' \
  --inherit climate
```

## Local testing with Docker

```bash
# Build
docker build -t bioiso-colony .

# Run (ephemeral SQLite — data lost on stop)
docker run -e CLAUDE_API_KEY=sk-ant-... -e DB_PATH=:memory: bioiso-colony

# Run with persistent volume
docker run \
  -v $(pwd)/data:/data \
  -e CLAUDE_API_KEY=sk-ant-... \
  -e DB_PATH=/data/bioiso.db \
  -e TICK_MS=5000 \
  bioiso-colony
```

## Cost estimate (Railway Hobby)

| Scenario | vCPU avg | RAM | Monthly |
|---|---|---|---|
| Idle (11 entities, 5s ticks) | 0.05–0.1 | 128 MB | ~$2–4 |
| Active evolution (retro-val running) | 0.25–0.5 | 256 MB | ~$8–12 |

The $5 Hobby plan credit covers the idle baseline. Expect **$5–15/month** for a 24/7 live colony.

## Architecture overview

```
Railway Service (single container)
│
├── start-colony.sh
│   ├── loom runtime spawn [11 entities]    ← idempotent
│   └── loom runtime start --tick-ms 5000   ← blocks forever
│
├── /data/bioiso.db  (mounted volume)
│   ├── entities table
│   ├── signals table
│   ├── drift_scores table
│   ├── telos_bounds table
│   ├── security_events table
│   └── checkpoints table
│
└── External APIs
    ├── Claude API (CLAUDE_API_KEY) — Tier 3 Mammal Brain
    └── Ollama (OLLAMA_BASE_URL, optional) — Tier 2 Ganglion
```
