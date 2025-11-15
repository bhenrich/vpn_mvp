#!/usr/bin/env python3
"""
Ensure a set of dev regions exist in directory-api so dev node containers can register automatically.
"""

from __future__ import annotations

import os
import time
from typing import Any

import requests

DEFAULT_REGIONS: list[dict[str, Any]] = [
    {"country_code": "DE", "city": "Berlin"},
    {"country_code": "DE", "city": "Munich"},
]


def directory_url() -> str:
    return os.getenv("DIRECTORY_URL", "http://127.0.0.1:8081")


def wait_for_directory(base_url: str, timeout: float = 30.0) -> None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            resp = requests.get(f"{base_url.rstrip('/')}/healthz", timeout=2)
            if resp.status_code == 200:
                return
        except requests.RequestException:
            time.sleep(1)
    raise RuntimeError("directory-api did not become ready in time")


def list_regions(base_url: str) -> list[dict[str, Any]]:
    resp = requests.get(f"{base_url.rstrip('/')}/regions", timeout=5)
    resp.raise_for_status()
    return resp.json() or []


def create_region(base_url: str, code: str, city: str) -> None:
    payload = {"country_code": code, "city": city, "status": "active"}
    resp = requests.post(f"{base_url.rstrip('/')}/regions", json=payload, timeout=5)
    if resp.status_code not in (200, 201, 409):
        raise RuntimeError(f"failed to create region {code}/{city}: {resp.text}")


def main() -> None:
    base_url = directory_url()
    wait_for_directory(base_url)
    existing = list_regions(base_url)
    existing_pairs = {(r["country_code"].upper(), r["city"]) for r in existing}
    for region in DEFAULT_REGIONS:
        key = (region["country_code"].upper(), region["city"])
        if key in existing_pairs:
            continue
        create_region(base_url, region["country_code"], region["city"])


if __name__ == "__main__":
    main()

