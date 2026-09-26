from aiogram import Router, types, F
from aiogram.filters import Command
from aiogram.types import InlineKeyboardMarkup, InlineKeyboardButton
from app.services.auth_service import auth_service
from app.services.history_service import history_service
from app.core.config import settings

router = Router()


@router.message(Command("start"))
async def cmd_start(message: types.Message):
    user_id = message.from_user.id
    token = await auth_service.get_valid_token(user_id)

    if token:
        user_info = await auth_service.get_user_info(user_id)
        name = user_info.get('name', 'Студент') if user_info else 'Студент'

        text = (
            f"Привет, {name}! 🎓\n"
            "Я — AI-ассистент НИУ ВШЭ.\n\n"
            "Меня можно спросить про дедлайны, лекции или просто поболтать со мной."
        )
        kb = InlineKeyboardMarkup(inline_keyboard=[
            [InlineKeyboardButton(text="🗑 Сбросить контекст", callback_data="clear_context")],
            [InlineKeyboardButton(text="👤 Профиль", callback_data="profile")]
        ])
    else:
        login_url = f"{settings.BASE_URL}/auth/login?tg_id={user_id}"
        text = (
            "Привет! Я — ИИ-ассистент ВШЭ.\n"
            "Чтобы я мог помогать с учебой, мне нужно знать, кто ты."
        )
        kb = InlineKeyboardMarkup(inline_keyboard=[
            [InlineKeyboardButton(text="🔑 Войти", url=login_url)]
        ])

    await message.answer(text, reply_markup=kb)


@router.callback_query(F.data == "clear_context")
async def btn_clear(callback: types.CallbackQuery):
    await history_service.clear_history(callback.from_user.id)
    await callback.message.answer("🧹 Контекст диалога очищен. Начинаем с чистого листа.")
    await callback.answer()


@router.callback_query(F.data == "profile")
async def btn_profile(callback: types.CallbackQuery):
    info = await auth_service.get_user_info(callback.from_user.id)
    if info:
        text = (
            f"👤 <b>Профиль</b>\n"
            f"Имя: {info.get('name')}\n"
            f"Email: {info.get('email')}\n"
            f"Login: {info.get('preferred_username')}"
        )
        kb = InlineKeyboardMarkup(inline_keyboard=[
            [InlineKeyboardButton(text="🚪 Выйти", callback_data="logout")]
        ])
        await callback.message.answer(text, reply_markup=kb)
    else:
        await callback.message.answer("Данные профиля недоступны. Попробуйте войти снова.")
    await callback.answer()


@router.callback_query(F.data == "logout")
async def btn_logout(callback: types.CallbackQuery):
    await auth_service.logout(callback.from_user.id)
    await history_service.clear_history(callback.from_user.id)

    login_url = f"{settings.BASE_URL}/auth/login?tg_id={callback.from_user.id}"
    kb = InlineKeyboardMarkup(inline_keyboard=[
        [InlineKeyboardButton(text="🔑 Войти снова", url=login_url)]
    ])
    await callback.message.edit_text("Вы вышли из системы.", reply_markup=kb)