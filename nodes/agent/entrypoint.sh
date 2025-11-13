#!/bin/sh
set -eu
echo "starting unbound DNS (DoT) on :5353"
unbound -c /etc/unbound/unbound.conf &
if [ "${RUN_NFT_SETUP:-0}" = "1" ]; then
	echo "applying nftables rules"
	nft -f /etc/nftables.conf || echo "nftables apply failed"
fi

# Optional: bring up a static WireGuard server for local testing
if [ "${WG_STATIC:-0}" = "1" ]; then
	echo "bringing up static WireGuard interface wg0"
	mkdir -p /wg
	if [ ! -f /wg/server.key ]; then
		wg genkey | tee /wg/server.key >/dev/null
		chmod 600 /wg/server.key
	fi
	SERVER_PRIV="$(cat /wg/server.key)"
	SERVER_PUB="$(echo "$SERVER_PRIV" | wg pubkey)"
	echo "server public key: $SERVER_PUB"

	WG_LISTEN_PORT="${WG_LISTEN_PORT:-51820}"

	# Create interface if not exists
	if ! ip link show wg0 >/dev/null 2>&1; then
		ip link add wg0 type wireguard
	fi
	ip address flush dev wg0 || true
	ip address add 10.66.0.1/24 dev wg0
	wg set wg0 private-key /wg/server.key listen-port "$WG_LISTEN_PORT"
	ip link set up dev wg0

	# Optional peer
	if [ -n "${WG_CLIENT_PUBKEY:-}" ]; then
		wg set wg0 peer "$WG_CLIENT_PUBKEY" allowed-ips 10.66.0.2/32 persistent-keepalive 25
		echo "added static client peer for 10.66.0.2/32"
	fi

	# Basic NAT for egress if nftables rules not applied
	if [ "${RUN_NFT_SETUP:-0}" != "1" ]; then
		EXT_IFACE="${EXT_IFACE:-eth0}"
		echo "enabling simple MASQUERADE on $EXT_IFACE via nftables"
		nft -f - <<'EOF' || true
table ip nat {
	chain postrouting {
		type nat hook postrouting priority srcnat;
		oifname "eth0" masquerade
	}
}
EOF
	fi

	# Keep container running for static mode
	tail -f /dev/null
else
	exec node-agent
fi


