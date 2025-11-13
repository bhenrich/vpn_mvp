#!/usr/bin/env python3
import os
from datetime import datetime, timedelta
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID, ExtendedKeyUsageOID


def write_key_and_cert(key, cert, key_path: Path, cert_path: Path) -> None:
	key_bytes = key.private_bytes(
		encoding=serialization.Encoding.PEM,
		format=serialization.PrivateFormat.TraditionalOpenSSL,
		encryption_algorithm=serialization.NoEncryption(),
	)
	cert_bytes = cert.public_bytes(serialization.Encoding.PEM)
	key_path.write_bytes(key_bytes)
	cert_path.write_bytes(cert_bytes)
	print(f"Wrote {key_path} and {cert_path}")


def gen_ca(subject_cn: str, days: int = 3650):
	key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
	subject = issuer = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, subject_cn)])
	now = datetime.utcnow()
	cert = (
		x509.CertificateBuilder()
		.subject_name(subject)
		.issuer_name(issuer)
		.public_key(key.public_key())
		.serial_number(x509.random_serial_number())
		.not_valid_before(now - timedelta(days=1))
		.not_valid_after(now + timedelta(days=days))
		.add_extension(x509.BasicConstraints(ca=True, path_length=None), critical=True)
		.sign(key, hashes.SHA256())
	)
	return key, cert


def gen_cert_signed_by_ca(ca_key, ca_cert, subject_cn: str, san_hosts: list[str], is_client: bool, days: int = 825):
	key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
	subject = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, subject_cn)])
	now = datetime.utcnow()
	san = x509.SubjectAlternativeName(
		[
			x509.DNSName(h) if not h.replace('.', '').isdigit() else x509.IPAddress(ipaddress.ip_address(h))
			for h in san_hosts
			for ipaddress in [__import__("ipaddress")]
		]
	)
	eku = x509.ExtendedKeyUsage(
		[x509.oid.ExtendedKeyUsageOID.CLIENT_AUTH] if is_client else [ExtendedKeyUsageOID.SERVER_AUTH]
	)
	cert = (
		x509.CertificateBuilder()
		.subject_name(subject)
		.issuer_name(ca_cert.subject)
		.public_key(key.public_key())
		.serial_number(x509.random_serial_number())
		.not_valid_before(now - timedelta(days=1))
		.not_valid_after(now + timedelta(days=days))
		.add_extension(san, critical=False)
		.add_extension(eku, critical=False)
		.sign(ca_key, hashes.SHA256())
	)
	return key, cert


def main():
	out_dir = Path(__file__).resolve().parent / "dev"
	out_dir.mkdir(parents=True, exist_ok=True)

	ca_key, ca_cert = gen_ca("VPN-MVP Dev CA")
	write_key_and_cert(ca_key, ca_cert, out_dir / "ca.key", out_dir / "ca.crt")

	dir_key, dir_cert = gen_cert_signed_by_ca(
		ca_key, ca_cert, "directory-api", ["directory-api", "localhost", "127.0.0.1"], is_client=False
	)
	write_key_and_cert(dir_key, dir_cert, out_dir / "directory.key", out_dir / "directory.crt")

	node_key, node_cert = gen_cert_signed_by_ca(
		ca_key, ca_cert, "node-agent", ["node-agent", "localhost", "127.0.0.1"], is_client=True
	)
	write_key_and_cert(node_key, node_cert, out_dir / "node.key", out_dir / "node.crt")

	print("Dev mTLS certs created in", out_dir)


if __name__ == "__main__":
	main()


