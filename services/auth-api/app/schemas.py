from __future__ import annotations

from datetime import datetime
from typing import Optional

from pydantic import BaseModel, EmailStr, Field


class UserRegisterRequest(BaseModel):
	email: EmailStr
	password: str = Field(min_length=8, max_length=256)
	tos_version: Optional[str] = None
	consent: bool = False
	residency: Optional[str] = Field(default=None, pattern="^[A-Z]{2}$", description="ISO 3166-1 alpha-2 country code")


class UserResponse(BaseModel):
	id: int
	email: EmailStr
	created_at: datetime
	tos_version: Optional[str] = None
	# residency is intentionally excluded from response to minimize exposure

	class Config:
		from_attributes = True


class LoginRequest(BaseModel):
	email: EmailStr
	password: str
	totp_code: Optional[str] = None


class TokenResponse(BaseModel):
	access_token: str
	refresh_token: str
	token_type: str = "bearer"
	expires_in: int


class RefreshRequest(BaseModel):
	refresh_token: str


class DeviceRegisterRequest(BaseModel):
	device_public_key_b64: str
	platform: str = Field(pattern="^(windows|macos|linux)$")


class DeviceResponse(BaseModel):
	id: int


class DeviceLoginStartRequest(BaseModel):
	code_challenge: str
	code_challenge_method: str = Field(pattern="^(S256|plain)$")
	user_hint: Optional[EmailStr] = None


class DeviceLoginStartResponse(BaseModel):
	device_code: str
	expires_in: int
	interval: int


class DeviceLoginAuthorizeRequest(BaseModel):
	device_code: str
	email: EmailStr
	password: str
	totp_code: Optional[str] = None


class DeviceLoginPollRequest(BaseModel):
	device_code: str
	code_verifier: str


