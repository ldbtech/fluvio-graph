FROM python:3.13-slim

WORKDIR /app

# psycopg2 build deps for the postgresql connector
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc libpq-dev curl && \
    rm -rf /var/lib/apt/lists/*

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY src/ ./src/

EXPOSE 3006

HEALTHCHECK --interval=5s --timeout=3s --retries=10 \
  CMD curl -sf http://localhost:3006/health || exit 1

CMD ["python", "-m", "uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "3006"]
