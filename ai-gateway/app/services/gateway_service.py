import logging
import httpx
from typing import Dict, Any, Union

from app.core.config import settings, LLMMode

from prometheus_client import Counter

from app.core.timer import timer

logger = logging.getLogger(__name__)

TOKENS_SPENT = Counter('ai_tokens_total', 'Total AI tokens spent', ['model', 'mode'])


class GatewayService:
    def __init__(self):
        self.proxy_mounts = {
            "http://": httpx.AsyncHTTPTransport(proxy=settings.PROXY_URL),
            "https://": httpx.AsyncHTTPTransport(proxy=settings.PROXY_URL),
        } if settings.PROXY_URL else None

    def _get_client(self, base_url: str, api_key: str) -> httpx.AsyncClient:
        return httpx.AsyncClient(
            base_url=base_url,
            headers={"Authorization": f"Bearer {api_key}"},
            mounts=self.proxy_mounts,
            timeout=120.0
        )

    async def chat_completion(self, payload: Dict[str, Any], mode: LLMMode) -> Dict[str, Any]:
        if mode == LLMMode.PRECISE:
            client = self._get_client(settings.NEBIUS_BASE_URL, settings.NEBIUS_API_KEY)
            model = settings.MODEL_LLM_PRECISE
        elif mode == LLMMode.FAST:
            client = self._get_client(settings.GROQ_BASE_URL, settings.GROQ_API_KEY)
            model = settings.MODEL_LLM_FAST
        else:
            client = self._get_client(settings.GROQ_BASE_URL, settings.GROQ_API_KEY)
            model = settings.MODEL_LLM_NORMAL

        payload["model"] = model

        async with client as c:
            logger.info(f"Routing LLM request to {model} (Mode: {mode})")
            with timer("Gateway LLM Call"):
                response = await c.post("/chat/completions", json=payload)
            response.raise_for_status()

            response_json = response.json()

            usage = response_json.get("usage", {})
            total_tokens = usage.get("total_tokens", 0)
            TOKENS_SPENT.labels(model=model, mode=mode).inc(total_tokens)

            return response_json

    async def vlm_analyze(self, text: str, image_url: str, temp: float, max_tokens: int) -> Dict[str, Any]:
        client = self._get_client(settings.NEBIUS_BASE_URL, settings.NEBIUS_API_KEY)

        payload = {
            "model": settings.MODEL_VLM,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": text},
                        {"type": "image_url", "image_url": {"url": image_url}}
                    ]
                }
            ],
            "temperature": temp,
            "max_tokens": max_tokens,
            "stream": False
        }

        async with client as c:
            logger.info(f"Routing VLM request to {settings.MODEL_VLM}")
            response = await c.post("/chat/completions", json=payload)
            response.raise_for_status()
            response.raise_for_status()

            response_json = response.json()

            usage = response_json.get("usage", {})
            total_tokens = usage.get("total_tokens", 0)
            TOKENS_SPENT.labels(model=settings.MODEL_VLM, mode="vlm").inc(total_tokens)

            return response_json

    async def create_embeddings(self, text: Union[str, list]) -> Dict[str, Any]:
        client = self._get_client(settings.NEBIUS_BASE_URL, settings.NEBIUS_API_KEY)

        async with client as c:
            logger.info(f"Routing Embedding request to {settings.MODEL_EMBEDDING}")
            response = await c.post("/embeddings", json={
                "model": settings.MODEL_EMBEDDING,
                "input": text
            })
            response.raise_for_status()

            response_json = response.json()

            usage = response_json.get("usage", {})
            total_tokens = usage.get("total_tokens", 0)
            TOKENS_SPENT.labels(model=settings.MODEL_EMBEDDING, mode="embeddings").inc(total_tokens)

            return response_json

    async def transcribe_audio(self, file_content: bytes, filename: str) -> Dict[str, Any]:
        client = self._get_client(settings.GROQ_BASE_URL, settings.GROQ_API_KEY)

        files = {"file": (filename, file_content)}
        data = {
            "model": settings.MODEL_ASR,
            "response_format": "verbose_json",
            "language": "ru"
        }

        async with client as c:
            logger.info(f"Routing ASR request to {settings.MODEL_ASR}")
            response = await c.post("/audio/transcriptions", files=files, data=data)
            response.raise_for_status()
            return response.json()


gateway_service = GatewayService()