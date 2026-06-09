FROM python:3.13-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl && \
    rm -rf /var/lib/apt/lists/*

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY src/ ./src/

EXPOSE 3008

# Note: the docker Python SDK talks to the host daemon — mount
# /var/run/docker.sock when running this container if sandbox tools are used.
HEALTHCHECK --interval=5s --timeout=3s --retries=10 \
  CMD curl -sf http://localhost:3008/health || exit 1

CMD ["python", "-m", "uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "3008"]
