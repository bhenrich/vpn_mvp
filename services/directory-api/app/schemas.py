from __future__ import annotations

import uuid
from datetime import datetime
from typing import Optional, List

from pydantic import BaseModel, Field, IPvAnyAddress
from typing import Optional, Literal


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
	public_endpoint: Optional[str] = Field(default=None, max_length=255)
	listen_port: Optional[int] = Field(default=None, ge=1, le=65535)
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
	public_endpoint: Optional[str]
	listen_port: Optional[int]
	capabilities: Optional[dict]
	status: str
	agent_version: Optional[str]
	wg_rs_version: Optional[str]
	last_seen_at: Optional[datetime]

	class Config:
		from_attributes = True


class DeviceRegisterRequest(BaseModel):
	device_id: Optional[uuid.UUID] = None
	device_name: Optional[str] = Field(default=None, max_length=128)
	platform: Literal["linux", "windows", "macos"]
	wg_public_key: str = Field(min_length=32, max_length=255)


class DeviceRead(BaseModel):
	id: uuid.UUID
	device_name: str
	platform: str
	wg_public_key: str
	status: str
	client_ip_v4: Optional[IPvAnyAddress]
	client_ip_v6: Optional[IPvAnyAddress]

	class Config:
		from_attributes = True


class MeshClientPeer(BaseModel):
	node_id: uuid.UUID
	public_key: str
	internal_wg_ip: Optional[IPvAnyAddress]
	endpoint_host: str
	endpoint_port: int


class MeshClientConfigRequest(BaseModel):
	region_id: uuid.UUID
	mode: Literal["single", "multi"] = "single"
	device_id: uuid.UUID
	full_tunnel: bool = True
	ipv6: bool = False


class MeshClientConfigResponse(BaseModel):
	session_id: uuid.UUID
	device_id: uuid.UUID
	client_ip_v4: IPvAnyAddress
	client_ip_v6: Optional[IPvAnyAddress]
	dns_servers: list[IPvAnyAddress]
	allowed_ips_v4: list[str]
	allowed_ips_v6: list[str]
	keepalive_seconds: int
	entry: MeshClientPeer
	exit: Optional[MeshClientPeer] = None


class ConnectionSessionRead(BaseModel):
	id: uuid.UUID
	node_id: uuid.UUID
	device_id: uuid.UUID
	status: str
	client_ip_v4: IPvAnyAddress
	client_ip_v6: Optional[IPvAnyAddress]
	allowed_ips_v4: list[str]
	allowed_ips_v6: list[str]
	dns_servers: list[IPvAnyAddress]
	keepalive_seconds: int
	started_at: datetime
	ended_at: Optional[datetime]

	class Config:
		from_attributes = True


