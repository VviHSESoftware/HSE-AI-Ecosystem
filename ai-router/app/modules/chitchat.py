from typing import List, Dict, Tuple
from app.api.v1.schemas import SourceMaterial
from app.modules.base import BaseModule
from app.services.gateway_client import gateway_client
from app.core.config import settings


class StudentChitchatModule(BaseModule):
    name = "student_chitchat"
    description = "Ответы на общие вопросы, пояснение сленга, приветствия, болтовня. Стиль: студент-студенту, с юмором, кратко."

    async def process(
            self, messages: List[Dict[str, str]], user_email: str, integrate_links: bool
    ) -> Tuple[str, List[SourceMaterial]]:
        system_prompt = (
            "Ты — опытный студент ВШЭ (Вышки). Твоя задача — отвечать на сообщения собеседника. "
            "Стиль общения: дружелюбный, неформальный, «на ты», с легким юмором. "
            "Используй студенческий сленг (матан, пара, препод, дедлайн, вышка), но в меру."
            "Отвечай кратко (не более 1-2 предложений). Это голосовой чат, поэтому не используй сложные списки, сокращения типа 'т.е.' и смайлики."
            "Пиши по-русски, без ошибок, правильно согласуй падежи, числа и рода"
        )

        messages = [{"role": "system", "content": system_prompt}] + messages

        text = await gateway_client.chat_completion(
            messages=messages,
            mode=settings.GENERATION_MODEL_MODE,
            temperature=0.7
        )

        return text, []