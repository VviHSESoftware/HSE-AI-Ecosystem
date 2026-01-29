import logging
from contextlib import asynccontextmanager
from fastapi import FastAPI
from starlette.middleware.sessions import SessionMiddleware
from prometheus_fastapi_instrumentator import Instrumentator

from app.core.config import settings
from app.core.logging_config import setup_logging
from app.api.v1.endpoints import webhook, auth

logger = logging.getLogger(__name__)

@asynccontextmanager
async def lifespan(app: FastAPI):
    setup_logging()
    logger.info(f"Starting {settings.PROJECT_NAME} v{settings.APP_VERSION}")
    yield
    logger.info("Shutting down...")

app = FastAPI(
    title=settings.PROJECT_NAME,
    version=settings.APP_VERSION,
    lifespan=lifespan
)

# Routes
app.include_router(webhook.router, prefix="/vk", tags=["VK Webhook"])
app.include_router(auth.router, prefix="/auth", tags=["Auth"])

# Middleware
app.add_middleware(SessionMiddleware, secret_key=settings.SESSION_SECRET)

@app.get("/health", tags=["System"])
async def health():
    return {"status": "ok", "mode": "callback_api"}

# Metrics
instrumentator = Instrumentator(
    should_group_status_codes=False,
    should_ignore_untemplated=True,
    excluded_handlers=["/metrics", "/docs", "/openapi.json", "/health"]
).instrument(app).expose(app)

if __name__ == "__main__":
    import uvicorn
    uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=True)