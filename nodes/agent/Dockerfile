FROM rust:1.84 as builder

WORKDIR /app

# Install protoc for prost-build
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Cache deps first
COPY Cargo.toml build.rs ./
COPY protos ./protos
RUN mkdir -p src && echo "fn main(){}" > src/main.rs && cargo build --release || true

# Build
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
	unbound nftables iproute2 ca-certificates curl wireguard-tools dos2unix && \
	update-ca-certificates && \
	rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/node-agent /usr/local/bin/node-agent
COPY unbound/unbound.conf /etc/unbound/unbound.conf
COPY nft/nftables.conf /etc/nftables.conf
RUN dos2unix /etc/nftables.conf || true
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod 755 /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]


