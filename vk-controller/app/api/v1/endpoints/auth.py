import logging
from fastapi import Request, APIRouter
from fastapi.responses import HTMLResponse
from vkbottle import Keyboard, KeyboardButtonColor, Text

from app.bot import bot
from app.core.config import settings
from app.core.keycloak import oauth
from app.services.auth_service import auth_service

router = APIRouter()
logger = logging.getLogger(__name__)

@router.get("/login")
async def login(request: Request, vk_id: int):
    request.session['vk_id'] = vk_id
    redirect_uri = f"{settings.BASE_URL}/auth/callback"
    return await oauth.keycloak.authorize_redirect(request, redirect_uri)

@router.get("/callback")
async def auth_callback(request: Request):
    try:
        token = await oauth.keycloak.authorize_access_token(request)
        user_info = token.get('userinfo')
        vk_id = request.session.get('vk_id')

        if not vk_id:
            return HTMLResponse("<h1>Ошибка: Lost VK ID</h1>", status_code=400)

        token['user_info'] = user_info
        await auth_service.save_tokens(vk_id, token)

        await bot.api.messages.send(
            peer_id=vk_id,
            message=f"✅ Вы успешно вошли как {user_info.get('name')}!",
            random_id=0
        )

        text = (
            f"Привет, {user_info.get('name')}! 🎓\n"
            "Я — AI-ассистент НИУ ВШЭ.\n\n"
            "Меня можно спросить про дедлайны, лекции или просто поболтать со мной."
        )

        keyboard = (
            Keyboard(one_time=False, inline=True)
            .add(Text("🗑 Сбросить контекст", payload={"cmd": "clear_context"}), color=KeyboardButtonColor.SECONDARY)
            .row()
            .add(Text("👤 Профиль", payload={"cmd": "profile"}), color=KeyboardButtonColor.PRIMARY)
        )

        await bot.api.messages.send(
            peer_id=vk_id,
            message=text,
            keyboard=keyboard.get_json(),
            random_id=0
        )

        return HTMLResponse("""
        <div style="text-align:center; font-family:sans-serif; margin-top:50px;">
            <h1>Успешно! 🎉</h1>
            <p>Вы можете закрыть это окно и вернуться в ВКонтакте.</p>
            <script>window.close()</script>
        </div>
        """)

    except Exception as e:
        logger.error(f"Auth callback error: {e}", exc_info=True)
        return HTMLResponse(f"<h1>Auth Error: {e}</h1>", status_code=500)