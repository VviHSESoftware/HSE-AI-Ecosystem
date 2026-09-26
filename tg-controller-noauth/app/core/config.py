from pathlib import Path
from pydantic_settings import BaseSettings, SettingsConfigDict

BASE_DIR = Path(__file__).resolve().parent.parent.parent


class Settings(BaseSettings):
    APP_VERSION: str = "1.0.0"
    PROJECT_NAME: str = "HSE TG Bot"
    LOG_LEVEL: str = "INFO"
    PROFILING: bool = False

    BOT_TOKEN: str
    PROXY_URL: str | None = None

    START_MESSAGE: str = (
        "Привет, {name}! 🎓\n"
        "Я — AI-ассистент НИУ ВШЭ.\n\n"
        "Меня можно спросить про дедлайны, лекции или просто поболтать со мной."
    )

    ROUTER_URL: str = "http://ai-router:8000/v1"
    ROUTER_SERVICE_TOKEN: str = "router_secret_token"
    KB_USER_NAME: str = "student@hse.ru"

    REDIS_URL: str = "redis://redis:6379/0"

    model_config = SettingsConfigDict(env_file=BASE_DIR / ".env", extra="ignore")


settings = Settings()