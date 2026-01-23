import asyncio
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

        limits = httpx.Limits(max_keepalive_connections=10, keepalive_expiry=60)

        self.nebius_client = httpx.AsyncClient(
            base_url=settings.NEBIUS_BASE_URL,
            headers={"Authorization": f"Bearer {settings.NEBIUS_API_KEY}"},
            mounts=self.proxy_mounts,
            timeout=120.0,
            limits=limits
        )

        self.groq_client = httpx.AsyncClient(
            base_url=settings.GROQ_BASE_URL,
            headers={"Authorization": f"Bearer {settings.GROQ_API_KEY}"},
            mounts=self.proxy_mounts,
            timeout=120.0,
            limits=limits
        )

        self._warmup_task = None

    async def chat_completion(self, payload: Dict[str, Any], mode: LLMMode) -> Dict[str, Any]:
        if mode == LLMMode.PRECISE:
            client = self.nebius_client
            model = settings.MODEL_LLM_PRECISE
        elif mode == LLMMode.FAST:
            client = self.groq_client
            model = settings.MODEL_LLM_FAST
        else:
            client = self.groq_client
            model = settings.MODEL_LLM_NORMAL

        payload["model"] = model

        logger.info(f"Routing LLM request to {model} (Mode: {mode})")
        with timer("Gateway LLM Call"):
            response = await client.post("/chat/completions", json=payload)
        response.raise_for_status()

        response_json = response.json()

        usage = response_json.get("usage", {})
        total_tokens = usage.get("total_tokens", 0)
        TOKENS_SPENT.labels(model=model, mode=mode).inc(total_tokens)

        return response_json

    async def vlm_analyze(self, text: str, image_url: str, temp: float, max_tokens: int) -> Dict[str, Any]:
        client = self.nebius_client

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

        logger.info(f"Routing VLM request to {settings.MODEL_VLM}")
        response = await client.post("/chat/completions", json=payload)
        response.raise_for_status()
        response.raise_for_status()

        response_json = response.json()

        usage = response_json.get("usage", {})
        total_tokens = usage.get("total_tokens", 0)
        TOKENS_SPENT.labels(model=settings.MODEL_VLM, mode="vlm").inc(total_tokens)

        return response_json

    async def create_embeddings(self, text: Union[str, list]) -> Dict[str, Any]:
        client = self.nebius_client

        logger.info(f"Routing Embedding request to {settings.MODEL_EMBEDDING}")
        response = await client.post("/embeddings", json={
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
        client = self.groq_client

        files = {"file": (filename, file_content)}
        data = {
            "model": settings.MODEL_ASR,
            "response_format": "verbose_json",
            "language": "ru"
        }

        logger.info(f"Routing ASR request to {settings.MODEL_ASR}")
        response = await client.post("/audio/transcriptions", files=files, data=data)
        response.raise_for_status()
        return response.json()

    async def start_warmup(self):
        if self._warmup_task is None:
            self._warmup_task = asyncio.create_task(self._warmup_loop())
            logger.info("Warmup task started")

    async def stop(self):
        if self._warmup_task:
            self._warmup_task.cancel()
            try:
                await self._warmup_task
            except asyncio.CancelledError:
                pass

        await self.groq_client.aclose()
        await self.nebius_client.aclose()
        logger.info("HTTP Clients closed")

    async def _warmup_loop(self):
        while True:
            try:
                await self.groq_client.get("/models")
                await self.nebius_client.get("/models")

            except Exception as e:
                logger.warning(f"Warmup ping failed: {e}")

            await asyncio.sleep(20)
gateway_service = GatewayService()