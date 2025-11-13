FROM python:3.12-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
	PYTHONUNBUFFERED=1

WORKDIR /app

ENV PYTHONPATH=/app

COPY services/auth-api/requirements.txt ./requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Copy shared common package used by imports
COPY services/common /app/services/common

# Copy Alembic config and migrations for DB setup
COPY services/auth-api/alembic.ini ./alembic.ini
COPY services/auth-api/migrations ./migrations

COPY services/auth-api/app ./app

EXPOSE 8080

# Run migrations then start the API
CMD ["sh", "-lc", "alembic upgrade head && uvicorn app.main:app --host 0.0.0.0 --port 8080 --no-access-log --log-level warning"]
