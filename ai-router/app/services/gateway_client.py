import httpx
import logging
from typing import List, Dict
from tenacity import retry, stop_after_attempt, wait_exponential

from app.core.config import settings
from app.core.timer import timer

logger = logging.getLogger(__name__)


class GatewayClient:
    def __init__(self):
        self.base_url = settings.AI_GATEWAY_URL
        self.headers = {
            "Authorization": f"Bearer {settings.AI_GATEWAY_TOKEN}",
            "Content-Type": "application/json"
        }
        self.client = httpx.AsyncClient(timeout=60.0)

    @retry(stop=stop_after_attempt(3), wait=wait_exponential(multiplier=1, min=1, max=10))
    async def chat_completion(self, messages: List[Dict[str, str]], mode: str = "fast",
                              temperature: float = 0.1) -> str:
        payload = {
            "messages": messages,
            "mode": mode,
            "temperature": temperature,
            "stream": False
        }

        try:
            with timer("Router Gateway Call"):
                response = await self.client.post(
                    f"{self.base_url}/llm",
                    json=payload,
                    headers=self.headers
                )
            response.raise_for_status()
            data = response.json()
            return data["content"]
        except httpx.HTTPStatusError as e:
            logger.error(f"Gateway error: {e.response.text}")
            raise
        except Exception as e:
            logger.error(f"Connection to Gateway failed: {str(e)}")
            raise

    async def close(self):
        await self.client.aclose()


gateway_client = GatewayClient()