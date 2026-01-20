from typing import List, Optional
from pydantic import BaseModel, EmailStr, Field

class ChatMessage(BaseModel):
    role: str
    content: str

class SourceMaterial(BaseModel):
    title: str
    url: str
    type: str = "link"
    timecode: Optional[int] = None
    snippet: Optional[str] = None


class RouterRequest(BaseModel):
    messages: List[ChatMessage]
    user_email: Optional[EmailStr] = None
    integrate_links_in_text: bool = Field(
        default=True,
        description="Если True - ссылки вставляются в текст [Name](url). Если False - текст чистый."
    )

class RouterResponse(BaseModel):
    content: str
    sources: List[SourceMaterial] = Field(default_factory=list)
    module_used: Optional[str] = None