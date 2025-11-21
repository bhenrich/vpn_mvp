Local static WireGuard test (no control plane)

1) Start the stack:
   - docker compose up -d node-agent
   - Check logs for: "server public key: <BASE64>"

2) On your laptop (WireGuard app):
   - Generate a keypair (keep the private key on the laptop).
   - Copy the laptop public key.

3) Authorize the laptop:
   - Set the env and restart the node:
     - PowerShell: $env:WG_CLIENT_PUBKEY="<LAPTOP_PUBKEY>"; docker compose up -d node-agent
     - Bash: WG_CLIENT_PUBKEY="<LAPTOP_PUBKEY>" docker compose up -d node-agent

4) Laptop tunnel config:
   [Interface]
   PrivateKey = <LAPTOP_PRIVATE_KEY>
   Address = 10.66.0.2/32
   DNS = 10.66.0.1

   [Peer]
   PublicKey = <SERVER_PUBLIC_KEY_FROM_LOGS>
   AllowedIPs = 0.0.0.0/0, ::/0
   Endpoint = <HOST_LAN_IP>:51820
   PersistentKeepalive = 25

5) On Windows host, ensure UDP 51820 is allowed through the firewall.
6) Activate the tunnel on the laptop and ping 10.66.0.1.


