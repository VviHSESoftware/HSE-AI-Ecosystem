from aiogram import Router, types, F
from aiogram.filters import Command
from aiogram.types import InlineKeyboardMarkup, InlineKeyboardButton

from app.core.config import settings
from app.services.history_service import history_service

router = Router()


@router.message(Command("start"))
async def cmd_start(message: types.Message):
    kb = InlineKeyboardMarkup(inline_keyboard=[
        [InlineKeyboardButton(text="🗑 Сбросить контекст", callback_data="clear_context")]
    ])

    await message.answer(settings.START_MESSAGE, reply_markup=kb)


@router.callback_query(F.data == "clear_context")
async def btn_clear(callback: types.CallbackQuery):
    await history_service.clear_history(callback.from_user.id)
    await callback.message.answer("🧹 Контекст диалога очищен. Начинаем с чистого листа.")
    await callback.answer()