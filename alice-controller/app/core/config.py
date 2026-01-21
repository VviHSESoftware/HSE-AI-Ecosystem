from pathlib import Path
from pydantic_settings import BaseSettings, SettingsConfigDict

BASE_DIR = Path(__file__).resolve().parent.parent.parent


class Settings(BaseSettings):
    APP_VERSION: str = "1.0.0"
    PROJECT_NAME: str = "HSE Alice Controller"
    LOG_LEVEL: str = "INFO"
    PROFILING: bool = False

    # --- Keycloak ---
    KEYCLOAK_SERVER_URL: str
    KEYCLOAK_REALM: str = "master"

    # --- Services ---
    ROUTER_URL: str = "http://ai-router:8000/v1"
    ROUTER_SERVICE_TOKEN: str = "router_secret_token"

    # --- Redis ---
    REDIS_URL: str = "redis://redis:6379/0"

    model_config = SettingsConfigDict(env_file=BASE_DIR / ".env", extra="ignore")

    @property
    def keycloak_userinfo_url(self) -> str:
        return f"{self.KEYCLOAK_SERVER_URL.rstrip('/')}/realms/{self.KEYCLOAK_REALM}/protocol/openid-connect/userinfo"


settings = Settings()