# Chaos Experiments (Dev)

Purpose: Exercise failure modes in the dev compose stack to validate alerting, SLOs, and recovery paths.

Requirements:
- Docker with Compose v2 (`docker compose`).
- Python 3.10+.

Usage:
1) From repo root, run:
   - python tools/chaos/chaos.py --help
2) Example: stop one random API every 2 minutes for 30 seconds, 10 times:
   - python tools/chaos/chaos.py --iterations 10 --downtime 30 --interval 120 --services auth-api directory-api admin-api
3) Pause/unpause instead of stop/start:
   - python tools/chaos/chaos.py --mode pause --iterations 5 --downtime 20 --services auth-api directory-api

Notes:
- By default, infra components (prometheus, grafana, jaeger, postgres, redis) are excluded; include them explicitly via --services if you want them targeted.
- This tool is intended for local/dev validation only.


