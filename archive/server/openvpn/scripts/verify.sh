#!/usr/bin/env sh
set -eu

# OpenVPN provides username/password via env when using 'via-env'
USER="${username:-}"
PASS="${password:-}"

AUTH_URL="${AUTH_HOST:-http://auth:8080}/api/v1/vpn/verify"

code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${AUTH_URL}" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"${USER}\",\"password\":\"${PASS}\"}")

if [ "$code" = "200" ]; then
  exit 0
else
  exit 1
fi



