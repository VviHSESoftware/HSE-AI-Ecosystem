import httpx
import logging
from app.core.config import settings

logger = logging.getLogger(__name__)


class AuthService:
    async def get_user_info(self, access_token: str) -> dict | None:
        if not access_token:
            return None

        async with httpx.AsyncClient() as client:
            try:
                resp = await client.get(
                    settings.keycloak_userinfo_url,
                    headers={"Authorization": f"Bearer {access_token}"}
                )
                if resp.status_code == 200:
                    return resp.json()
                else:
                    logger.warning(f"Invalid token from Alice: {resp.status_code}")
                    return None
            except Exception as e:
                logger.error(f"Auth check error: {e}")
                return None


auth_service = AuthService()