import os

workers_per_core = 1
cores = os.cpu_count() or 1
default_web_concurrency = workers_per_core * cores
web_concurrency = os.getenv("WEB_CONCURRENCY", None)
workers = int(web_concurrency) if web_concurrency else default_web_concurrency
bind = os.getenv("BIND", "0.0.0.0:8000")
keepalive = 120
errorlog = "-"
accesslog = "-"
loglevel = os.getenv("LOG_LEVEL", "info").lower()
worker_class = "uvicorn.workers.UvicornWorker"