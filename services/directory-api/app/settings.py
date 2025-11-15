from functools import lru_cache
from pydantic_settings import BaseSettings, SettingsConfigDict
from pydantic import Field


class Settings(BaseSettings):
	model_config = SettingsConfigDict(
		env_file=".env",
		env_file_encoding="utf-8",
		extra="ignore",
	)

	port: int = 8081
	grpc_port: int = 50051
	database_url: str = "postgresql+asyncpg://postgres:example@postgres:5432/postgres"
	redis_url: str = "redis://redis:6379/0"

	# Auth
	auth_jwks_url: str = "http://auth-api:8080/.well-known/jwks.json"

	# gRPC TLS / mTLS
	grpc_tls_enabled: bool = False
	grpc_tls_cert_path: str | None = None
	grpc_tls_key_path: str | None = None
	grpc_tls_client_ca_path: str | None = None

@lru_cache(maxsize=1)
def get_settings() -> Settings:
	return Settings()


