from abc import ABC, abstractmethod
from typing import List, Dict, Tuple

from app.api.v1.schemas import SourceMaterial


class BaseModule(ABC):
    @property
    @abstractmethod
    def name(self) -> str:
        pass

    @property
    @abstractmethod
    def description(self) -> str:
        pass

    @abstractmethod
    async def process(
        self,
        messages: List[Dict[str, str]],
        user_email: str,
        integrate_links: bool
    ) -> Tuple[str, List[SourceMaterial]]:
        pass