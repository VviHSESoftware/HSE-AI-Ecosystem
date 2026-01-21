import logging
from fastapi import APIRouter
from app.api.v1.schemas import (
    AliceRequest, AliceResponse, AliceResponsePayload, AliceButton
)
from app.core.timer import timer
from app.services.auth_service import auth_service
from app.services.history_service import history_service
from app.services.router_client import router_client

router = APIRouter()
logger = logging.getLogger(__name__)


@router.post("/webhook", response_model=AliceResponse)
async def handle_alice_webhook(req: AliceRequest):
    with timer("Alice Endpoint"):
        session = req.session
        request_data = req.request

        token = None
        if session.user:
            token = session.user.access_token

        if session.new:
            if not token:
                return _build_auth_response("Привет! Чтобы я мог помочь с учебой, пожалуйста, войдите в систему.")

            user_info = await auth_service.get_user_info(token)
            if not user_info:
                return _build_auth_response("Кажется, сессия истекла. Пожалуйста, войдите снова.")

            name = user_info.get("given_name", "Студент")
            return _build_response(f"Привет, {name}! Я готов к работе. Что вас интересует?")

        if not token:
            return _build_auth_response("Я не могу ответить без авторизации. Нажмите кнопку, чтобы войти.")

        try:
            user_id = session.user.user_id
            user_text = request_data.command

            if not user_text:
                return _build_response("Я вас слушаю.")

            user_info = await auth_service.get_user_info(token)
            if not user_info:
                return _build_auth_response("Требуется повторный вход.")
            email = user_info.get("email", "unknown")

            history = await history_service.get_history(user_id)
            messages = history + [{"role": "user", "content": user_text}]

            router_resp = await router_client.send_request(messages, email)

            answer_text = router_resp.get("content", "Не удалось получить ответ.")
            sources = router_resp.get("sources", [])

            buttons = []
            for src in sources:
                buttons.append(AliceButton(
                    title=f"🔗 {src.get('title')}",
                    url=src.get('url'),
                    hide=True
                ))

            await history_service.add_message(user_id, "user", user_text)
            await history_service.add_message(user_id, "assistant", answer_text)

            return _build_response(answer_text, buttons=buttons)

        except Exception as e:
            logger.error(f"Alice Error: {e}", exc_info=True)
            return _build_response("Произошла ошибка в магии Вышки. Попробуйте позже.")


def _build_response(text: str, buttons: list = None, end_session: bool = False) -> AliceResponse:
    return AliceResponse(
        response=AliceResponsePayload(
            text=text,
            buttons=buttons or [],
            end_session=end_session
        )
    )


def _build_auth_response(text: str) -> AliceResponse:
    return AliceResponse(
        response=AliceResponsePayload(
            text=text,
            directives={"start_account_linking": {}}
        )
    )