import logging
from aiogram import Router, types, flags
from aiogram.enums import ParseMode

from app.core.timer import timer
from app.services.auth_service import auth_service
from app.services.history_service import history_service
from app.services.router_client import router_client

router = Router()

logger = logging.getLogger(__name__)

@router.message()
@flags.chat_action("typing")
async def handle_text(message: types.Message):
    with timer("TG Endpoint"):
        user_id = message.from_user.id
        user_text = message.text

        token = await auth_service.get_valid_token(user_id)
        if not token:
            await message.answer("🔒 Пожалуйста, авторизуйтесь через /start, чтобы я мог отвечать.")
            return

        user_info = await auth_service.get_user_info(user_id)
        email = user_info.get('email') if user_info else "unknown"

        history = await history_service.get_history(user_id)

        messages = history + [{"role": "user", "content": user_text}]

        try:
            with timer("TG Router Call"):
                result = await router_client.send_request(messages, email)

            answer_text = result.get("content", "Ошибка генерации")

            await message.answer(answer_text, parse_mode=ParseMode.MARKDOWN)

            await history_service.add_message(user_id, "user", user_text)
            await history_service.add_message(user_id, "assistant", answer_text)

        except Exception as e:
            logger.error(f"Chat response error: {e}", exc_info=True)
            await message.answer("К сожалению, мне не удалось найти ответ на этот вопрос в базе знаний. Попробуйте задать другой вопрос.")