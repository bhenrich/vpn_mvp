from __future__ import annotations

import ipaddress
from typing import Optional

from fastapi import HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.models import ClientDevice

_POOL_PREFIX = ipaddress.IPv4Address("10.66.0.0")
_POOL_NET = ipaddress.IPv4Network("10.66.0.0/24")
_POOL_MIN_HOST = 2
_POOL_MAX_HOST = 254


async def ensure_client_ipv4(db: AsyncSession, device: ClientDevice) -> str:
	"""
	Ensure the device has an assigned IPv4 address within the client pool.
	Allocates a deterministic-but-collision-checked /32 if missing.
	"""
	if device.client_ip_v4:
		return device.client_ip_v4
	ip = await _allocate_ipv4(db, device)
	device.client_ip_v4 = ip
	return ip


async def _allocate_ipv4(db: AsyncSession, device: ClientDevice) -> str:
	# Ensure the device has a primary key assigned before using it for deterministic allocation.
	# For newly created devices, the ID may not be populated until after the first flush.
	if device.id is None:
		await db.flush()
		await db.refresh(device)

	candidates = _POOL_MAX_HOST - _POOL_MIN_HOST + 1
	base = device.id.int % candidates
	for offset in range(candidates):
		host = _POOL_MIN_HOST + ((base + offset) % candidates)
		addr = ipaddress.IPv4Address(int(_POOL_PREFIX) + host)
		candidate = str(addr)
		existing = await db.execute(
			select(ClientDevice).where(ClientDevice.client_ip_v4 == candidate, ClientDevice.id != device.id)
		)
		if existing.scalar_one_or_none() is None:
			return candidate
	raise HTTPException(status_code=status.HTTP_503_SERVICE_UNAVAILABLE, detail="Client IPv4 pool exhausted")



