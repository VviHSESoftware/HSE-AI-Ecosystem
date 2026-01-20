import json
import redis.asyncio as redis
from typing import List, Dict
from app.core.config import settings


class HistoryService:
    def __init__(self):
        self.redis = redis.from_url(settings.REDIS_URL, decode_responses=True)
        self.limit = 10
        self.ttl = 3600

    async def add_message(self, user_id: str, role: str, content: str):
        key = f"alice_history:{user_id}"
        msg = {"role": role, "content": content}

        async with self.redis.pipeline() as pipe:
            pipe.rpush(key, json.dumps(msg))
            pipe.ltrim(key, -self.limit, -1)
            pipe.expire(key, self.ttl)
            await pipe.execute()

    async def get_history(self, user_id: str) -> List[Dict[str, str]]:
        key = f"alice_history:{user_id}"
        raw_list = await self.redis.lrange(key, 0, -1)
        return [json.loads(x) for x in raw_list]

    async def clear_history(self, user_id: str):
        await self.redis.delete(f"alice_history:{user_id}")


history_service = HistoryService()