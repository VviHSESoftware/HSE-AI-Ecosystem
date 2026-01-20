from typing import List, Dict, Tuple

from app.api.v1.schemas import SourceMaterial
from app.modules.base import BaseModule


class RagCourseModule(BaseModule):
    name = "course_rag"
    description = "Вопросы по содержанию курсов, лекций, материалам."

    async def process(
            self, messages: List[Dict[str, str]], user_email: str, integrate_links: bool
    ) -> Tuple[str, List[SourceMaterial]]:
        found_sources = [
            SourceMaterial(
                title="Лекция 3. Теорема Байеса",
                url="https://vk.com/video-123_456?t=930",
                type="video",
                timecode=930
            ),
            SourceMaterial(
                title="Учебник по Матанализу (Глава 5)",
                url="https://lms.hse.ru/mod/resource/view.php?id=999",
                type="file"
            )
        ]

        link_instruction = ""
        if not integrate_links:
            link_instruction = "НЕ вставляй ссылки и URL в текст ответа. Просто ссылайся на названия материалов."
        else:
            link_instruction = "Используй Markdown [Название](url) для ссылок в тексте."


        if integrate_links:
            answer_text = (
                "Преподаватель подробно разбирал это в [Лекции 3](https://vk.com/video-123_456?t=930). "
                "Основная формула приведена в [Учебнике](https://lms.hse.ru/mod/resource/view.php?id=999)."
            )
        else:
            answer_text = (
                "Преподаватель подробно разбирал это в третьей лекции, начиная с пятнадцатой минуты. "
                "Основная формула также есть в пятой главе учебника."
            )

        return answer_text, found_sources