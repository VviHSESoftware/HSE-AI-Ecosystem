from typing import List, Dict, Any, Optional, Union
from pydantic import BaseModel, Field
from app.core.config import LLMMode


class ChatMessage(BaseModel):
    role: str
    content: Union[str, List[Dict[str, Any]]]


class LLMRequest(BaseModel):
    messages: List[ChatMessage]
    mode: LLMMode = Field(default=LLMMode.NORMAL, description="Execution mode: fast, normal, precise")
    temperature: Optional[float] = 0.7
    max_tokens: Optional[int] = None
    stream: bool = False


class LLMResponse(BaseModel):
    content: Optional[str] = ""
    model: str
    usage: Dict[str, Any] = Field(default_factory=dict)


class EmbeddingRequest(BaseModel):
    input: Union[str, List[str]]


class EmbeddingResponse(BaseModel):
    embeddings: List[List[float]]
    model: str
    usage: Dict[str, Any] = Field(default_factory=dict)


class VLMRequest(BaseModel):
    text: str
    image_url: str
    temperature: Optional[float] = 0.1
    max_tokens: Optional[int] = 4096