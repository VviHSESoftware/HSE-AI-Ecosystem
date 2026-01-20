import json
import time
import logging
import httpx
import redis.asyncio as redis
from app.core.config import settings

logger = logging.getLogger(__name__)


class AuthService:
    def __init__(self):
        self.redis = redis.from_url(settings.REDIS_URL, decode_responses=True)

    async def save_tokens(self, tg_user_id: int, token_data: dict):
        expires_in = token_data.get('expires_in', 300)
        expires_at = int(time.time()) + expires_in

        data = {
            "access_token": token_data['access_token'],
            "refresh_token": token_data.get('refresh_token'),
            "expires_at": expires_at,
        }

        if 'user_info' in token_data:
            await self.redis.set(f"tg:{tg_user_id}:info", json.dumps(token_data['user_info']))

        await self.redis.set(f"tg:{tg_user_id}:tokens", json.dumps(data))

    async def get_valid_token(self, tg_user_id: int) -> str | None:
        raw_json = await self.redis.get(f"tg:{tg_user_id}:tokens")
        if not raw_json:
            return None

        data = json.loads(raw_json)

        if data['expires_at'] > time.time() + 30:
            return data['access_token']

        logger.info(f"Token expired for user {tg_user_id}, refreshing...")
        return await self._refresh_token(tg_user_id, data)

    async def _refresh_token(self, tg_user_id: int, old_data: dict) -> str | None:
        refresh_token = old_data.get('refresh_token')
        if not refresh_token:
            return None

        async with httpx.AsyncClient() as client:
            try:
                resp = await client.post(settings.keycloak_token_url, data={
                    'client_id': settings.CLIENT_ID,
                    'client_secret': settings.CLIENT_SECRET,
                    'grant_type': 'refresh_token',
                    'refresh_token': refresh_token
                })

                if resp.status_code == 200:
                    new_tokens = resp.json()
                    if 'refresh_token' not in new_tokens:
                        new_tokens['refresh_token'] = refresh_token

                    await self.save_tokens(tg_user_id, new_tokens)
                    return new_tokens['access_token']
                else:
                    logger.warning(f"Failed to refresh token: {resp.text}")
                    await self.logout(tg_user_id)
                    return None
            except Exception as e:
                logger.error(f"Refresh network error: {e}")
                return None

    async def get_user_info(self, tg_user_id: int) -> dict | None:
        raw = await self.redis.get(f"tg:{tg_user_id}:info")
        return json.loads(raw) if raw else None

    async def logout(self, tg_user_id: int):
        await self.redis.delete(f"tg:{tg_user_id}:tokens")
        await self.redis.delete(f"tg:{tg_user_id}:info")


auth_service = AuthService()