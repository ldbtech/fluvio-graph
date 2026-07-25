# Build context is the REPO ROOT (see docker-compose.yml), because this service
# now depends on the sibling package packages/fluvio-planner. The image mirrors
# the repo layout under /srv so the editable path in requirements.txt
# (-e ../../packages/fluvio-planner) resolves exactly as it does locally.
FROM python:3.13-slim

# git installs the csp-sdk dependency (a git+https requirement); curl is used by
# the healthcheck. slim images ship neither.
RUN apt-get update && apt-get install -y --no-install-recommends \
    git curl && \
    rm -rf /var/lib/apt/lists/*

# The reusable domain package first (its own layer — changes less often than the
# service shell), then the service requirements (which editable-install it).
WORKDIR /srv
COPY packages/fluvio-planner/ ./packages/fluvio-planner/
COPY services/agent-planner/requirements.txt ./services/agent-planner/requirements.txt

# pip resolves the "-e ../../packages/fluvio-planner" path against the CWD, so
# install from the service directory — the same relative path used locally.
WORKDIR /srv/services/agent-planner
RUN pip install --no-cache-dir -r requirements.txt

# The FastAPI transport shell.
COPY services/agent-planner/app/ ./app/

EXPOSE 3007

HEALTHCHECK --interval=5s --timeout=3s --retries=10 \
  CMD curl -sf http://localhost:3007/health || exit 1

CMD ["python", "-m", "uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "3007"]
