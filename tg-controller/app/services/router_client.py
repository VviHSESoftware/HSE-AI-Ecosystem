import httpx
import logging
from typing import List, Dict, Any
from app.core.config import settings
from app.core.timer import timer

logger = logging.getLogger(__name__)

class RouterClient:
    def __init__(self):
        self.base_url = settings.ROUTER_URL
        self.headers = {
            "Authorization": f"Bearer {settings.ROUTER_SERVICE_TOKEN}"
        }

    async def send_request(self, messages: List[Dict], user_email: str) -> Dict[str, Any]:
        payload = {
            "messages": messages,
            "user_email": user_email,
            "integrate_links_in_text": True
        }

        async with httpx.AsyncClient(timeout=60.0) as client:
            try:
                with timer("TG Router Request"):
                    response = await client.post(
                        f"{self.base_url}/process",
                        json=payload,
                        headers=self.headers
                    )
                response.raise_for_status()
                return response.json()
            except httpx.HTTPError as e:
                logger.error(f"Router API Error: {e}")
                raise

router_client = RouterClient()