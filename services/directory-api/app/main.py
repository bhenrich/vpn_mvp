from fastapi import FastAPI

from app.settings import get_settings
from app.db import init_db
from app.redis_client import init_redis, close_redis
from app.grpc_server import start_grpc_server, stop_grpc_server
from app.routers.regions import router as regions_router
from app.routers.nodes import router as nodes_router
from app.routers.mesh import router as mesh_router
from services.common.observability import init_observability

app = FastAPI(title="directory-api", version="0.1.0")
init_observability(app, service_name="directory-api")


@app.on_event("startup")
async def on_startup() -> None:
	# initialize database schema for dev; production uses Alembic
	await init_db()
	# settings are loaded to ensure env correctness at boot
	get_settings()
	# initialize Redis connection
	await init_redis()
	# start gRPC skeleton in background
	await start_grpc_server()


@app.on_event("shutdown")
async def on_shutdown() -> None:
	# gracefully close Redis connections
	await close_redis()
	# stop gRPC
	await stop_grpc_server()


@app.get("/healthz")
def healthz() -> dict[str, str]:
	return {"status": "ok"}


app.include_router(regions_router, prefix="/regions", tags=["regions"])
app.include_router(nodes_router, prefix="/nodes", tags=["nodes"])
app.include_router(mesh_router, prefix="/mesh", tags=["mesh"])


if __name__ == "__main__":
	import uvicorn
	uvicorn.run("app.main:app", host="0.0.0.0", port=8081, reload=False)


