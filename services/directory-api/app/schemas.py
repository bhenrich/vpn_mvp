from __future__ import annotations

import uuid
from datetime import datetime
from typing import Optional, List

from pydantic import BaseModel, Field, IPvAnyAddress
from typing import Optional, Literal
import uuid


# Regions
class RegionCreate(BaseModel):
	country_code: str = Field(min_length=2, max_length=2, description="ISO 3166-1 alpha-2 code")
	city: str
	status: str = "active"
	latency_score: Optional[float] = None


class RegionRead(RegionCreate):
	id: uuid.UUID

	class Config:
		from_attributes = True


# Mesh / Path selection
class MeshPathRequest(BaseModel):
	region_id: uuid.UUID
	mode: Literal["single", "multi"] = "single"


class NodeRef(BaseModel):
	id: uuid.UUID
	public_key: str
	internal_wg_ip: Optional[IPvAnyAddress] = None
	egress_ips: list[IPvAnyAddress] = []


class MeshPathResponse(BaseModel):
	mode: Literal["single", "multi"]
	entry: NodeRef
	exit: Optional[NodeRef] = None


class RegionUpdate(BaseModel):
	country_code: Optional[str] = Field(default=None, min_length=2, max_length=2)
	city: Optional[str] = None
	status: Optional[str] = None
	latency_score: Optional[float] = None


# Nodes
class NodeRegisterRequest(BaseModel):
	public_key: str = Field(min_length=32, max_length=255)
	region_id: Optional[uuid.UUID] = None
	internal_wg_ip: Optional[IPvAnyAddress] = None
	egress_ips: Optional[List[IPvAnyAddress]] = None
	capabilities: Optional[dict] = None
	agent_version: Optional[str] = None
	wg_rs_version: Optional[str] = None


class NodeHeartbeatRequest(BaseModel):
	public_key: str
	status: Optional[str] = None
	agent_version: Optional[str] = None
	wg_rs_version: Optional[str] = None


class NodeRead(BaseModel):
	id: uuid.UUID
	public_key: str
	region_id: Optional[uuid.UUID]
	internal_wg_ip: Optional[IPvAnyAddress]
	egress_ips: Optional[list[IPvAnyAddress]]
	capabilities: Optional[dict]
	status: str
	agent_version: Optional[str]
	wg_rs_version: Optional[str]
	last_seen_at: Optional[datetime]

	class Config:
		from_attributes = True


