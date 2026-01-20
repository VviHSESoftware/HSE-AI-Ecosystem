from enum import Enum
from pathlib import Path
from typing import Optional, Set, Union

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

BASE_DIR = Path(__file__).resolve().parent.parent.parent


class LLMMode(str, Enum):
    FAST = "fast"
    NORMAL = "normal"
    PRECISE = "precise"


class Settings(BaseSettings):
    APP_VERSION: str = "1.0.0"
    PROJECT_NAME: str = "HSE AI Gateway"
    LOG_LEVEL: str = "INFO"

    AI_GATEWAY_API_TOKENS: Union[Set[str], str] = Field(default_factory=set)

    # Providers
    GROQ_API_KEY: str
    GROQ_BASE_URL: str = "https://api.groq.com/openai/v1"

    NEBIUS_API_KEY: str
    NEBIUS_BASE_URL: str = "https://api.studio.nebius.ai/v1"

    # Models
    MODEL_LLM_FAST: str = "openai/gpt-oss-120b"
    MODEL_LLM_NORMAL: str = "openai/gpt-oss-120b"
    MODEL_LLM_PRECISE: str = "Qwen/Qwen3-235B-A22B-Thinking-2507"

    MODEL_VLM: str = "Qwen/Qwen2.5-VL-72B-Instruct"
    MODEL_EMBEDDING: str = "Qwen/Qwen3-Embedding-8B"
    MODEL_ASR: str = "whisper-large-v3-turbo"

    # Proxy
    PROXY_URL: Optional[str] = None

    model_config = SettingsConfigDict(env_file=BASE_DIR / ".env", extra="ignore")

    @field_validator("AI_GATEWAY_API_TOKENS", mode="before")
    @classmethod
    def parse_tokens(cls, v):
        print("TOKEN PARSE")
        if isinstance(v, str):
            return set(v.split(","))
        return v

    @field_validator("PROXY_URL")
    @classmethod
    def format_proxy(cls, v: str | None) -> str | None:
        if not v:
            return None
        if v.startswith("http"):
            return v
        # ip:port:user:pass -> http://user:pass@ip:port
        parts = v.split(":")
        if len(parts) == 4:
            ip, port, user, password = parts
            return f"http://{user}:{password}@{ip}:{port}"
        return f"http://{v}"


settings = Settings()