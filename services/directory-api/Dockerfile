FROM python:3.12-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
	PYTHONUNBUFFERED=1

WORKDIR /app

COPY services/directory-api/requirements.txt ./requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Copy shared package to satisfy 'services.common' imports (requires build context at repo root)
COPY services/common /app/services/common

COPY services/directory-api/app ./app

EXPOSE 8081
EXPOSE 50051

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8081", "--no-access-log", "--log-level", "warning"]


