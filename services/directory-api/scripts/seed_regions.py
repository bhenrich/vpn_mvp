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


def auth_url(base_url: str) -> str:
    """Get auth-api URL from directory-api URL."""
    return os.getenv("AUTH_URL", base_url.replace("8081", "8080").replace(":8081", ":8080"))


def obtain_token(base_url: str, email: str, password: str) -> str:
    """Authenticate with auth-api and obtain bearer token."""
    auth_base = auth_url(base_url)
    payload = {"email": email, "password": password}
    try:
        resp = requests.post(f"{auth_base.rstrip('/')}/auth/login", json=payload, timeout=5)
        resp.raise_for_status()
        token = resp.json().get("access_token")
        if not token:
            raise RuntimeError("Auth response missing access_token")
        return token
    except requests.RequestException as e:
        raise RuntimeError(f"Failed to authenticate with auth-api: {e}")


def list_regions(base_url: str, token: str | None = None) -> list[dict[str, Any]]:
    headers = {}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    resp = requests.get(f"{base_url.rstrip('/')}/regions", headers=headers, timeout=5)
    resp.raise_for_status()
    return resp.json() or []


def create_region(base_url: str, code: str, city: str, token: str) -> None:
    payload = {"country_code": code, "city": city, "status": "active"}
    headers = {"Authorization": f"Bearer {token}"}
    resp = requests.post(f"{base_url.rstrip('/')}/regions", json=payload, headers=headers, timeout=5)
    if resp.status_code not in (200, 201, 409):
        raise RuntimeError(f"failed to create region {code}/{city}: {resp.text}")


def main() -> None:
    base_url = directory_url()
    wait_for_directory(base_url)
    
    # Obtain authentication token (use defaults for dev/testing)
    email = os.getenv("SEED_EMAIL", "admin@test.de")
    password = os.getenv("SEED_PASSWORD", "test")
    
    try:
        token = obtain_token(base_url, email, password)
    except Exception as e:
        print(f"Warning: Failed to authenticate ({e}), attempting to list regions without auth...")
        token = None
    
    # List existing regions (GET /regions doesn't require auth)
    existing = list_regions(base_url, token=token)
    existing_pairs = {(r["country_code"].upper(), r["city"]) for r in existing}
    
    # If we don't have a token and there are no existing regions, we can't create new ones
    if not token:
        print("No authentication token available. Regions already exist or authentication failed.")
        print(f"Existing regions: {[f\"{r['country_code']}/{r['city']}\" for r in existing]}")
        return
    
    # Create missing regions
    for region in DEFAULT_REGIONS:
        key = (region["country_code"].upper(), region["city"])
        if key in existing_pairs:
            print(f"Region {region['country_code']}/{region['city']} already exists, skipping")
            continue
        print(f"Creating region {region['country_code']}/{region['city']}...")
        create_region(base_url, region["country_code"], region["city"], token)
        print(f"Successfully created region {region['country_code']}/{region['city']}")


if __name__ == "__main__":
    main()

