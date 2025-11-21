#!/usr/bin/env bash
set -euo pipefail

# Initialize Root and Intermediate CAs using OpenSSL.
# Root is intended to be kept offline. Intermediate will be used for mTLS issuance.

ROOT_DIR="${1:-./pki}"
mkdir -p "${ROOT_DIR}/root" "${ROOT_DIR}/intermediate"

umask 077

cat >"${ROOT_DIR}/openssl_root.cnf" <<'EOF'
[ req ]
distinguished_name = dn
x509_extensions = v3_ca
prompt = no

[ dn ]
CN = VPN Root CA
O = VPN
C = US

[ v3_ca ]
basicConstraints = critical, CA:true
keyUsage = critical, keyCertSign, cRLSign
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer
EOF

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

# Root CA (10 years)
openssl genrsa -out "${ROOT_DIR}/root/rootCA.key" 4096
openssl req -x509 -new -nodes -key "${ROOT_DIR}/root/rootCA.key" -sha256 -days 3650 \
	-config "${ROOT_DIR}/openssl_root.cnf" \
	-out "${ROOT_DIR}/root/rootCA.crt"

# Intermediate CA (180 days)
openssl genrsa -out "${ROOT_DIR}/intermediate/intermediate.key" 4096
openssl req -new -key "${ROOT_DIR}/intermediate/intermediate.key" \
	-config "${ROOT_DIR}/openssl_intermediate.cnf" \
	-out "${ROOT_DIR}/intermediate/intermediate.csr"

openssl x509 -req -in "${ROOT_DIR}/intermediate/intermediate.csr" -CA "${ROOT_DIR}/root/rootCA.crt" \
	-CAkey "${ROOT_DIR}/root/rootCA.key" -CAcreateserial -out "${ROOT_DIR}/intermediate/intermediate.crt" \
	-days 180 -sha256 -extfile "${ROOT_DIR}/openssl_intermediate.cnf" -extensions v3_intermediate

cat "${ROOT_DIR}/intermediate/intermediate.crt" "${ROOT_DIR}/root/rootCA.crt" > "${ROOT_DIR}/intermediate/fullchain.crt"

mkdir -p "${ROOT_DIR}/current"
cp "${ROOT_DIR}/intermediate/intermediate.key" "${ROOT_DIR}/current/"
cp "${ROOT_DIR}/intermediate/fullchain.crt" "${ROOT_DIR}/current/"

echo "Initialized Root and Intermediate CAs under ${ROOT_DIR}. Current chain/key in ${ROOT_DIR}/current."


