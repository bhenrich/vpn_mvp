from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
	PORT: int = 8080
	DATABASE_URL: str = "postgresql+psycopg://postgres:example@postgres:5432/postgres"
	REDIS_URL: str = "redis://redis:6379/0"

	JWT_ISSUER: str = "auth-api"
	JWT_ACCESS_TTL_MINUTES: int = 15
	JWT_REFRESH_TTL_DAYS: int = 7
	JWKS_ROTATION_HOURS: int = 24

	ARGON2_TIME_COST: int = 3
	ARGON2_MEMORY_COST: int = 64 * 1024
	ARGON2_PARALLELISM: int = 2

	DEVICE_LOGIN_EXPIRES_IN_SECONDS: int = 600
	DEVICE_LOGIN_POLL_INTERVAL_SECONDS: int = 5

	model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", case_sensitive=False)


settings = Settings()


