import time
import logging
from contextlib import contextmanager

from app.core.config import settings

logger = logging.getLogger(__name__)

@contextmanager
def timer(name: str):
    if not settings.PROFILING:
        return
    start_time = time.perf_counter()
    try:
        yield
    finally:
        end_time = time.perf_counter()
        elapsed_ms = (end_time - start_time) * 1000
        logger.info(f"[TIMER] {name}: {elapsed_ms:.2f} ms")