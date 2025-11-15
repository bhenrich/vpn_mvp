#!/bin/sh
set -eu
echo "enabling IPv4 forwarding"
sysctl -w net.ipv4.ip_forward=1 >/dev/null 2>&1 || true
echo "starting unbound DNS (DoT) on :53"
unbound -c /etc/unbound/unbound.conf &
if [ "${RUN_NFT_SETUP:-0}" = "1" ]; then
	echo "applying nftables rules"
	nft -f /etc/nftables.conf || echo "nftables apply failed"
fi

# Always apply nftables rules for forwarding and NAT (required for VPN to work)
# This is needed even in dynamic mode when node-agent manages WireGuard
echo "applying nftables rules for VPN forwarding and NAT"
nft -f - <<'NFTEOF' || echo "nftables apply failed"
table inet vpn {
	chain forward {
		type filter hook forward priority 0;
		policy drop;
		iifname "wg0" accept;
		oifname "wg0" accept;
	}
	chain postrouting {
		type nat hook postrouting priority 100;
		oifname != "wg0" masquerade;
	}
}
NFTEOF

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
		echo "enabling forwarding and MASQUERADE between wg0 <$EXT_IFACE> via nftables"
		nft -f - <<EOF || true
table inet filter {
	chain forward {
		type filter hook forward priority 0;
		iifname "wg0" oifname "$EXT_IFACE" accept
		iifname "$EXT_IFACE" oifname "wg0" ct state related,established accept
	}
}
table ip nat {
	chain postrouting {
		type nat hook postrouting priority srcnat;
		oifname "$EXT_IFACE" masquerade
	}
}
EOF
	fi

	# Keep container running for static mode
	tail -f /dev/null
else
	exec node-agent
fi


