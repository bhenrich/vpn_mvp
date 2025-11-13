from contextlib import asynccontextmanager

from fastapi import FastAPI

from app.bootstrap import bootstrap_test_user
from app.config import settings
from app.routers import auth as auth_routes
from app.routers import devices as device_routes
from app.routers import gdpr as gdpr_routes
from app.routers import users as user_routes
from app.security import key_manager
from services.common.observability import init_observability


@asynccontextmanager
async def lifespan(app: FastAPI):
	"""Lifespan context manager for startup/shutdown"""
	# Startup
	bootstrap_test_user()
	yield
	# Shutdown (nothing to do)


app = FastAPI(title="auth-api", version="0.1.0", lifespan=lifespan)

init_observability(app, service_name="auth-api")

@app.get("/healthz")
def healthz() -> dict[str, str]:
	return {"status": "ok"}


@app.get("/.well-known/jwks.json", include_in_schema=False)
def well_known_jwks() -> dict:
	return key_manager.get_all_public_jwks()


app.include_router(user_routes.router)
app.include_router(auth_routes.router)
app.include_router(device_routes.router)
app.include_router(gdpr_routes.router)


if __name__ == "__main__":
	import uvicorn
	uvicorn.run("app.main:app", host="0.0.0.0", port=settings.PORT, reload=False)


