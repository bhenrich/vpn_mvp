from functools import lru_cache
from pydantic_settings import BaseSettings
from pydantic import AnyUrl
from pydantic import field_validator


class Settings(BaseSettings):
	port: int = 8082
	database_url: AnyUrl = "postgresql+asyncpg://postgres:example@postgres:5432/postgres"
	redis_url: AnyUrl = "redis://redis:6379/0"
	auth_jwks_url: AnyUrl = "http://auth-api:8080/.well-known/jwks.json"
	bootstrap_operator_user_id: int | None = None

	@field_validator("bootstrap_operator_user_id", mode="before")
	@classmethod
	def empty_str_to_none(cls, v):
		if isinstance(v, str) and v.strip() == "":
			return None
		return v

	class Config:
		env_file = ".env"
		env_file_encoding = "utf-8"
		case_sensitive = False


@lru_cache(maxsize=1)
def get_settings() -> Settings:
	return Settings()



