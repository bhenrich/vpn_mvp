from fastapi import FastAPI
from sqlalchemy import select

from app.db import get_db_session, init_db
from app.models import Operator
from app.settings import get_settings
from app.routers import operators as operators_router
from app.routers import flags as flags_router
from app.routers import nodes as nodes_router
from services.common.observability import init_observability

app = FastAPI(title="admin-api", version="0.1.0")
init_observability(app, service_name="admin-api")


@app.get("/healthz")
def healthz() -> dict[str, str]:
	return {"status": "ok"}


@app.on_event("startup")
async def _on_startup() -> None:
	await init_db()
	# Bootstrap operator if configured
	settings = get_settings()
	if settings.bootstrap_operator_user_id is not None:
		async for session in get_db_session():
			existing = await session.execute(select(Operator).where(Operator.user_id == settings.bootstrap_operator_user_id))
			if existing.scalar_one_or_none() is None:
				op = Operator(user_id=settings.bootstrap_operator_user_id, role="admin")
				session.add(op)
				await session.commit()
			break


app.include_router(operators_router.router)
app.include_router(flags_router.router)
app.include_router(nodes_router.router)


if __name__ == "__main__":
	import uvicorn
	uvicorn.run("app.main:app", host="0.0.0.0", port=get_settings().port, reload=False)


