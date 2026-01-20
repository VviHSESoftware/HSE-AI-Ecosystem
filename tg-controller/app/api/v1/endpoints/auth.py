import logging

from aiogram.types import InlineKeyboardMarkup, InlineKeyboardButton
from fastapi import Request, APIRouter
from fastapi.responses import HTMLResponse

from app.bot import bot
from app.core.config import settings
from app.core.keycloak import oauth
from app.services.auth_service import auth_service

router = APIRouter()

logger = logging.getLogger(__name__)

@router.get("/login")
async def login(request: Request, tg_id: int):
    request.session['tg_id'] = tg_id
    redirect_uri = f"{settings.BASE_URL}/auth/callback"
    return await oauth.keycloak.authorize_redirect(request, redirect_uri)


@router.get("/callback")
async def auth_callback(request: Request):
    try:
        token = await oauth.keycloak.authorize_access_token(request)
        user_info = token.get('userinfo')
        tg_id = request.session.get('tg_id')

        if not tg_id:
            return HTMLResponse("<h1>Ошибка: Lost telegram ID</h1>", status_code=400)

        token['user_info'] = user_info
        await auth_service.save_tokens(tg_id, token)

        await bot.send_message(
            chat_id=tg_id,
            text=f"✅ Вы успешно вошли как <b>{user_info.get('name')}</b>!",
            parse_mode="HTML"
        )

        text = (
            f"Привет, {user_info.get('name')}! 🎓\n"
            "Я — твой AI-ассистент ВШЭ. Я помню контекст нашей беседы.\n\n"
            "Спрашивай про дедлайны, лекции или просто поболтаем."
        )
        kb = InlineKeyboardMarkup(inline_keyboard=[
            [InlineKeyboardButton(text="🗑 Сбросить контекст", callback_data="clear_context")],
            [InlineKeyboardButton(text="👤 Профиль", callback_data="profile")]
        ])

        await bot.send_message(
            chat_id=tg_id,
            text=text,
            parse_mode="HTML",
            reply_markup=kb,
        )

        return HTMLResponse("""
        <div style="text-align:center; font-family:sans-serif; margin-top:50px;">
            <h1>Успешно! 🎉</h1>
            <p>Вы можете закрыть это окно и вернуться в Telegram.</p>
            <script>window.close()</script>
        </div>
        """)

    except Exception as e:
        logger.error(f"Auth callback error: {e}", exc_info=True)
        return HTMLResponse(f"<h1>Auth Error: {e}</h1>", status_code=500)
