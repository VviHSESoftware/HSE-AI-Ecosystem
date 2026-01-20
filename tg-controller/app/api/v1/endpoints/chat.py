from aiogram import Router, types, flags
from app.services.auth_service import auth_service
from app.services.history_service import history_service
from app.services.router_client import router_client

router = Router()


@router.message()
@flags.chat_action("typing")
async def handle_text(message: types.Message):
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
        result = await router_client.send_request(messages, email)

        answer_text = result.get("content", "Ошибка генерации")

        await message.answer(answer_text, parse_mode="HTML")

        await history_service.add_message(user_id, "user", user_text)
        await history_service.add_message(user_id, "assistant", answer_text)

    except Exception:
        await message.answer("😔 Произошла ошибка при обработке запроса. Попробуйте позже.")