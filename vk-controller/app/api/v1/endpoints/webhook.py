import logging
from fastapi import APIRouter, Request, Response
from app.bot import bot
from app.core.config import settings
from app.core.timer import timer
from app.services.auth_service import auth_service
from app.services.history_service import history_service
from app.services.router_client import router_client
from vkbottle import Keyboard, KeyboardButtonColor, Text, OpenLink

router = APIRouter()
logger = logging.getLogger(__name__)


@router.post("/callback")
async def vk_callback_handler(request: Request):
    event = await request.json()

    if event.get("type") == "confirmation":
        return Response(content=settings.VK_CONFIRMATION_CODE)

    await bot.process_event(event)

    return Response(content="ok")

@bot.on.message(text=["/start", "Начать"])
async def start_handler(message):
    vk_id = message.from_id
    token = await auth_service.get_valid_token(vk_id)

    if token:
        user_info = await auth_service.get_user_info(vk_id)
        name = user_info.get('name', 'Студент') if user_info else 'Студент'

        keyboard = (
            Keyboard(one_time=False, inline=True)
            .add(Text("🗑 Сбросить контекст", payload={"cmd": "clear_context"}), color=KeyboardButtonColor.SECONDARY)
            .row()
            .add(Text("👤 Профиль", payload={"cmd": "profile"}), color=KeyboardButtonColor.PRIMARY)
        )

        await message.answer(
            f"Привет, {name}! 👋\nВы уже авторизованы.",
            keyboard=keyboard
        )
    else:
        login_url = f"{settings.BASE_URL}/auth/login?vk_id={vk_id}"

        keyboard = (
            Keyboard(one_time=False, inline=True)
            .add(OpenLink(link=login_url, label="🔑 Войти через Keycloak"))
        )

        await message.answer(
            "Привет! Я — AI-ассистент НИУ ВШЭ.\nДля доступа к функциям нужно авторизоваться.",
            keyboard=keyboard
        )


@bot.on.message(payload={"cmd": "clear_context"})
async def clear_context_handler(message):
    await history_service.clear_history(message.from_id)
    await message.answer("🧹 Контекст диалога очищен.")


@bot.on.message(payload={"cmd": "profile"})
async def profile_handler(message):
    info = await auth_service.get_user_info(message.from_id)
    if info:
        keyboard = (
            Keyboard(one_time=False, inline=True)
            .add(OpenLink(link=f"{settings.BASE_URL}/auth/login?vk_id={message.from_id}",
                          label="🚪 Войти заново/Сменить"))
        )
        await message.answer(
            f"👤 Профиль:\nИмя: {info.get('name')}\nEmail: {info.get('email')}",
            keyboard=keyboard
        )
    else:
        await message.answer("Данные профиля недоступны. Попробуйте войти снова.")


@bot.on.message()
async def chat_handler(message):
    with timer("VK Endpoint"):
        vk_id = message.from_id
        user_text = message.text

        token = await auth_service.get_valid_token(vk_id)
        if not token:
            await message.answer("🔒 Пожалуйста, авторизуйтесь (напишите /start).")
            return

        user_info = await auth_service.get_user_info(vk_id)
        email = user_info.get('email') if user_info else "unknown"

        history = await history_service.get_history(vk_id)
        messages = history + [{"role": "user", "content": user_text}]

        try:
            with timer("VK Router Call"):
                result = await router_client.send_request(messages, email)

            answer_text = result.get("content", "Ошибка генерации")

            sources = result.get("sources", [])
            if sources:
                sources_list = []
                for source in sources:
                    url = source.get("url")

                    if url:
                        sources_list.append(f"• {url}")

                if sources_list:
                    answer_text += "\n\n📚 Источники:\n" + "\n".join(sources_list)

            await message.answer(answer_text, parse_mode="Markdown")

            await history_service.add_message(vk_id, "user", user_text)
            await history_service.add_message(vk_id, "assistant", answer_text)

        except Exception as e:
            logger.error(f"Chat response error: {e}", exc_info=True)
            await message.answer("😔 Произошла ошибка. Попробуйте позже.")