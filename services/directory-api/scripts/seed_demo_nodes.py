#!/usr/bin/env python3
"""
Seed helper that creates two German regions and registers placeholder nodes in each.

Usage:
    python services/directory-api/scripts/seed_demo_nodes.py --token <OPERATOR_BEARER_TOKEN>

Environment variables:
    DIRECTORY_URL   Base URL for directory-api (default: http://127.0.0.1:8081)
    DIRECTORY_TOKEN Bearer token to use if --token is not supplied.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import sys
from typing import Any, Dict, Optional

try:
    import requests
except ImportError:  # pragma: no cover
    print("Error: python 'requests' package is required. Install with `pip install requests` and rerun.")
    raise


NodeSpec = Dict[str, Any]


GERMAN_NODES: list[NodeSpec] = [
    {
        "country_code": "DE",
        "city": "Berlin",
        "internal_wg_ip": "10.70.0.1",
        "public_endpoint": "198.51.100.10",
        "listen_port": 51820,
    },
    {
        "country_code": "DE",
        "city": "Munich",
        "internal_wg_ip": "10.70.0.2",
        "public_endpoint": "198.51.100.11",
        "listen_port": 51821,
    },
]


def request(
    base_url: str,
    method: str,
    path: str,
    token: Optional[str],
    payload: Optional[dict[str, Any]] = None,
) -> tuple[int, Any]:
    url = base_url.rstrip("/") + path
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    try:
        resp = requests.request(method, url, headers=headers, json=payload, timeout=5)
    except requests.RequestException as exc:  # pragma: no cover - connectivity issue
        raise RuntimeError(f"Failed to reach {url}: {exc}") from exc
    if not resp.content:
        body: Any = None
    else:
        try:
            body = resp.json()
        except json.JSONDecodeError:
            body = resp.text
    if resp.status_code >= 400:
        raise RuntimeError(f"{method} {path} failed ({resp.status_code}): {body}")
    return resp.status_code, body


def obtain_token(base_url: str, email: str, password: str) -> str:
    auth_url = os.getenv("AUTH_URL", base_url.replace("8081", "8080"))
    payload = {"email": email, "password": password}
    _, resp = request(auth_url, "POST", "/auth/login", token=None, payload=payload)
    token = resp.get("access_token")
    if not token:
        raise RuntimeError("Auth response missing access_token")
    return token


def ensure_region(base_url: str, token: str, country_code: str, city: str) -> str:
    _, regions = request(base_url, "GET", "/regions/", token=None)
    for region in regions or []:
        if region["country_code"] == country_code and region["city"] == city:
            return region["id"]
    payload = {"country_code": country_code, "city": city, "status": "active"}
    _, data = request(base_url, "POST", "/regions/", token, payload)
    return data["id"]


def deterministic_public_key(label: str) -> str:
    """
    Generate a deterministic, WireGuard-compatible public key string for demo nodes.

    WireGuard keys are 32-byte values encoded as standard base64 with padding,
    resulting in a 44-character string. We therefore must NOT strip the '=' padding,
    otherwise downstream agents will reject the key as invalid.
    """
    digest = hashlib.sha256(label.encode("utf-8")).digest()
    return base64.standard_b64encode(digest[:32]).decode("ascii")


def register_node(base_url: str, token: str, region_id: str, spec: NodeSpec) -> dict[str, Any]:
    label = f"{spec['city'].lower()}-{spec['public_endpoint']}"
    public_key = deterministic_public_key(label)
    payload = {
        "public_key": public_key,
        "region_id": region_id,
        "internal_wg_ip": spec["internal_wg_ip"],
        "egress_ips": [spec["public_endpoint"]],
        "public_endpoint": spec["public_endpoint"],
        "listen_port": spec["listen_port"],
        "agent_version": "seed-script",
        "wg_rs_version": "kernel-wg",
    }
    _, node = request(base_url, "POST", "/nodes/register", token=None, payload=payload)
    heartbeat_payload = {
        "public_key": public_key,
        "status": "online",
        "agent_version": "seed-script",
        "wg_rs_version": "kernel-wg",
    }
    request(base_url, "POST", "/nodes/heartbeat", token=None, payload=heartbeat_payload)
    return node


def main() -> None:
    parser = argparse.ArgumentParser(description="Seed two German nodes into directory-api.")
    parser.add_argument(
        "--base-url",
        default=os.getenv("DIRECTORY_URL", "http://127.0.0.1:8081"),
        help="Base URL for directory-api (default: %(default)s)",
    )
    parser.add_argument(
        "--email",
        default=os.getenv("DIRECTORY_SEED_EMAIL", "admin@test.de"),
        help="Auth email used to obtain token (default: admin@test.de)",
    )
    parser.add_argument(
        "--password",
        default=os.getenv("DIRECTORY_SEED_PASSWORD", "test"),
        help="Auth password used to obtain token (default: test)",
    )
    args = parser.parse_args()

    token = obtain_token(args.base_url, args.email, args.password)

    created_nodes: list[dict[str, Any]] = []
    for spec in GERMAN_NODES:
        region_id = ensure_region(args.base_url, token, spec["country_code"], spec["city"])
        node = register_node(args.base_url, token, region_id, spec)
        created_nodes.append(node)

    print("Seeded nodes:")
    for node in created_nodes:
        print(
            f"- {node['public_key']} in {node['region_id']} "
            f"(endpoint={node.get('public_endpoint')}:{node.get('listen_port')}) status={node.get('status')}"
        )


if __name__ == "__main__":
    main()


