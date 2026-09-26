import logging
import sys
from pythonjsonlogger.json import JsonFormatter
from app.core.config import settings


def setup_logging():
    log_level = settings.LOG_LEVEL.upper()
    numeric_level = getattr(logging, log_level, None)
    if not isinstance(numeric_level, int):
        raise ValueError(f"Invalid log level: {log_level}")

    logger = logging.getLogger()
    logger.setLevel(numeric_level)

    for handler in logger.handlers[:]:
        logger.removeHandler(handler)
        handler.close()

    log_handler = logging.StreamHandler(sys.stdout)
    formatter = JsonFormatter(
        "%(asctime)s %(levelname)s %(name)s %(module)s %(funcName)s %(message)s"
    )
    log_handler.setFormatter(formatter)
    logger.addHandler(log_handler)

    logging.getLogger("uvicorn.access").setLevel(logging.WARNING)
    logging.getLogger("uvicorn.error").setLevel(logging.WARNING)
    logging.getLogger("httpx").setLevel(logging.WARNING)