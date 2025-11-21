#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-./pki}"
umask 077

if [[ ! -f "${ROOT_DIR}/root/rootCA.crt" || ! -f "${ROOT_DIR}/root/rootCA.key" ]]; then
  echo "Root CA not found. Run init_ca.sh first." >&2
  exit 1
fi

TS="$(date +%Y%m%d%H%M%S)"
NEW_DIR="${ROOT_DIR}/intermediate_${TS}"
mkdir -p "${NEW_DIR}"

cat >"${ROOT_DIR}/openssl_intermediate.cnf" <<'EOF'
[ req ]
distinguished_name = dn
prompt = no

[ dn ]
CN = VPN Intermediate CA
O = VPN
C = US

[ v3_intermediate ]
basicConstraints = critical, CA:true, pathlen:0
keyUsage = critical, keyCertSign, cRLSign
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer
EOF

openssl genrsa -out "${NEW_DIR}/intermediate.key" 4096
openssl req -new -key "${NEW_DIR}/intermediate.key" \
	-config "${ROOT_DIR}/openssl_intermediate.cnf" \
	-out "${NEW_DIR}/intermediate.csr"

openssl x509 -req -in "${NEW_DIR}/intermediate.csr" -CA "${ROOT_DIR}/root/rootCA.crt" \
	-CAkey "${ROOT_DIR}/root/rootCA.key" -CAcreateserial -out "${NEW_DIR}/intermediate.crt" \
	-days 180 -sha256 -extfile "${ROOT_DIR}/openssl_intermediate.cnf" -extensions v3_intermediate

cat "${NEW_DIR}/intermediate.crt" "${ROOT_DIR}/root/rootCA.crt" > "${NEW_DIR}/fullchain.crt"

rm -rf "${ROOT_DIR}/current"
mkdir -p "${ROOT_DIR}/current"
cp "${NEW_DIR}/intermediate.key" "${ROOT_DIR}/current/"
cp "${NEW_DIR}/fullchain.crt" "${ROOT_DIR}/current/"

echo "Rotated Intermediate CA. Current chain/key updated from ${NEW_DIR}."


