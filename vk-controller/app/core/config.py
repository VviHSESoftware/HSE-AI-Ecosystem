from pathlib import Path
from pydantic_settings import BaseSettings, SettingsConfigDict

BASE_DIR = Path(__file__).resolve().parent.parent.parent


class Settings(BaseSettings):
    APP_VERSION: str = "1.0.0"
    PROJECT_NAME: str = "HSE VK Controller"
    LOG_LEVEL: str = "INFO"
    PROFILING: bool = False

    # --- VK Specifics ---
    VK_TOKEN: str
    VK_CONFIRMATION_CODE: str
    VK_SECRET_KEY: str = ""

    # --- Keycloak ---
    KEYCLOAK_SERVER_URL: str
    KEYCLOAK_REALM: str = "master"
    CLIENT_ID: str
    CLIENT_SECRET: str

    # --- Service Config ---
    BASE_URL: str = "https://vkbot.hsecontest.ru" #
    SESSION_SECRET: str = "change_me_in_prod"

    ROUTER_URL: str = "http://ai-router:8000/v1"
    ROUTER_SERVICE_TOKEN: str = "router_secret_token"

    REDIS_URL: str = "redis://redis:6379/0"

    model_config = SettingsConfigDict(env_file=BASE_DIR / ".env", extra="ignore")

    @property
    def keycloak_metadata_url(self) -> str:
        return f"{self.KEYCLOAK_SERVER_URL.rstrip('/')}/realms/{self.KEYCLOAK_REALM}/.well-known/openid-configuration"

    @property
    def keycloak_token_url(self) -> str:
        return f"{self.KEYCLOAK_SERVER_URL.rstrip('/')}/realms/{self.KEYCLOAK_REALM}/protocol/openid-connect/token"


settings = Settings()