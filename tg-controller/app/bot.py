from aiogram import Bot, Dispatcher
from aiogram.enums import ParseMode
from aiogram.client.default import DefaultBotProperties
from app.core.config import settings
from aiogram.client.session.aiohttp import AiohttpSession

session = AiohttpSession(proxy=settings.PROXY_URL)

bot = Bot(
    token=settings.BOT_TOKEN,
    default=DefaultBotProperties(parse_mode=ParseMode.MARKDOWN),
    session=session
)
dp = Dispatcher()