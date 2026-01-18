import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request, status
from fastapi.responses import ORJSONResponse, JSONResponse
from fastapi.exceptions import RequestValidationError
from prometheus_fastapi_instrumentator import Instrumentator

from app.core.config import settings
from app.core.logging_config import setup_logging
from app.api.v1.endpoints import generation, audio

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
    default_response_class=ORJSONResponse,
    lifespan=lifespan
)

# Routes
app.include_router(generation.router, prefix="/v1", tags=["Generation"])
app.include_router(audio.router, prefix="/v1", tags=["Audio"])

@app.exception_handler(RequestValidationError)
async def validation_exception_handler(request: Request, exc: RequestValidationError):
    logger.warning(f"Validation error: {exc.errors()}")
    return JSONResponse(
        status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
        content={"detail": exc.errors()},
    )

@app.get("/health", tags=["System"])
async def health_check():
    return {"status": "ok", "proxy_enabled": bool(settings.PROXY_URL)}

# Metrics
instrumentator = Instrumentator(
    should_group_status_codes=False,
    should_ignore_untemplated=True,
    excluded_handlers=["/metrics", "/docs", "/openapi.json", "/health"]
).instrument(app).expose(app)

if __name__ == "__main__":
    import uvicorn
    uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=True)