import json
from pathlib import Path
from typing import Set

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

BASE_DIR = Path(__file__).resolve().parent.parent.parent


class Settings(BaseSettings):
    APP_VERSION: str = "1.0.0"
    PROJECT_NAME: str = "HSE AI Router"
    LOG_LEVEL: str = "INFO"

    AI_ROUTER_API_TOKENS: Set[str] = Field(default_factory=set)

    AI_GATEWAY_URL: str = "http://ai-gateway:8000/v1"
    AI_GATEWAY_TOKEN: str

    ROUTER_MODEL_MODE: str = "fast"
    GENERATION_MODEL_MODE: str = "normal"

    model_config = SettingsConfigDict(
        env_file=BASE_DIR / ".env",
        extra="ignore"
    )

    @field_validator("AI_ROUTER_API_TOKENS", mode="before")
    @classmethod
    def parse_tokens(cls, v):
        if isinstance(v, str):
            try:
                return set(json.loads(v))
            except json.JSONDecodeError:
                return {v}
        return v

settings = Settings()