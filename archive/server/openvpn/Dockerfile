FROM debian:stable-slim
RUN apt-get update && apt-get install -y --no-install-recommends openvpn curl iptables && rm -rf /var/lib/apt/lists/*
WORKDIR /etc/openvpn
COPY config/server.conf /etc/openvpn/server.conf
COPY scripts/verify.sh /etc/openvpn/scripts/verify.sh
RUN chmod +x /etc/openvpn/scripts/verify.sh && mkdir -p /var/log/openvpn
# PKI materials should be mounted or baked into the image at /etc/openvpn/pki
# Status file for exporter
RUN mkdir -p /etc/openvpn/status
EXPOSE 1194/udp
CMD ["openvpn", "--config", "/etc/openvpn/server.conf"]



