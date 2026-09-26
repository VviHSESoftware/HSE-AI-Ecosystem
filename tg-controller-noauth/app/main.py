import asyncio
import logging
from contextlib import asynccontextmanager
from fastapi import FastAPI
from prometheus_fastapi_instrumentator import Instrumentator

from app.core.config import settings
from app.core.logging_config import setup_logging
from app.bot import dp, bot
from app.api.v1.endpoints import chat, common

logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    setup_logging()
    logger.info(f"Starting {settings.PROJECT_NAME} v{settings.APP_VERSION}")
    await bot.delete_webhook(drop_pending_updates=True)

    polling_task = asyncio.create_task(dp.start_polling(bot))

    yield

    logger.info("Shutting down...")
    polling_task.cancel()
    await bot.session.close()


app = FastAPI(
    title=settings.PROJECT_NAME,
    version=settings.APP_VERSION,
    lifespan=lifespan
)

# Routes
dp.include_router(common.router)
dp.include_router(chat.router)


@app.get("/health", tags=["System"])
async def health():
    return {"status": "ok", "bot": "polling"}


# Metrics
instrumentator = Instrumentator(
    should_group_status_codes=False,
    should_ignore_untemplated=True,
    excluded_handlers=["/metrics", "/docs", "/openapi.json", "/health"]
).instrument(app).expose(app)

if __name__ == "__main__":
    import uvicorn
    uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=True)