from __future__ import annotations

from datetime import datetime
from typing import Literal, Optional

from pydantic import BaseModel, Field


OperatorRole = Literal["admin", "operator", "read_only"]


class OperatorCreateRequest(BaseModel):
	user_id: int = Field(ge=1)
	role: OperatorRole = "operator"


class OperatorResponse(BaseModel):
	id: int
	user_id: int
	role: OperatorRole
	created_at: datetime

	class Config:
		from_attributes = True


class FeatureFlagCreateRequest(BaseModel):
	key: str = Field(min_length=1, max_length=128)
	enabled: bool = False
	description: Optional[str] = Field(default=None, max_length=512)
	rollout_percentage: int = Field(ge=0, le=100, default=100)


class FeatureFlagUpdateRequest(BaseModel):
	enabled: Optional[bool] = None
	description: Optional[str] = Field(default=None, max_length=512)
	rollout_percentage: Optional[int] = Field(default=None, ge=0, le=100)


class FeatureFlagResponse(BaseModel):
	key: str
	enabled: bool
	description: Optional[str] = None
	rollout_percentage: int

	class Config:
		from_attributes = True


class NodeLifecycleRequest(BaseModel):
	reason: Optional[str] = Field(default=None, max_length=512)
	version: Optional[str] = Field(default=None, max_length=64)


class AcceptedResponse(BaseModel):
	accepted: bool = True



