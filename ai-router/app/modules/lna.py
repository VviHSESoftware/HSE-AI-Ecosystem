from typing import List, Dict, Tuple
from app.api.v1.schemas import SourceMaterial
from app.modules.base import BaseModule

class LnaModule(BaseModule):
    name = "lna_help"
    description = "Вопросы по нормативным актам ВШЭ (ЛНА), правилам внутреннего распорядка, приказам, скидкам, стипендиям."

    async def process(
            self, messages: List[Dict[str, str]], user_email: str, integrate_links: bool
    ) -> Tuple[str, List[SourceMaterial]]:
        return "Модуль анализа ЛНА пока не реализован. Обратитесь в учебный офис.", []