from typing import Optional, List, Dict, Any
from pydantic import BaseModel

# --- Request ---

class AliceUser(BaseModel):
    user_id: str
    access_token: Optional[str] = None

class AliceSession(BaseModel):
    new: bool
    message_id: int
    session_id: str
    skill_id: str
    user: Optional[AliceUser] = None

class AliceRequestPayload(BaseModel):
    command: str
    original_utterance: str
    type: str

class AliceRequest(BaseModel):
    meta: Dict[str, Any]
    session: AliceSession
    request: AliceRequestPayload
    version: str

# --- Response ---

class AliceButton(BaseModel):
    title: str
    url: Optional[str] = None
    payload: Optional[Dict] = None
    hide: bool = True

class AliceResponsePayload(BaseModel):
    text: str
    tts: Optional[str] = None
    buttons: List[AliceButton] = []
    end_session: bool = False
    directives: Optional[Dict] = None

class AliceResponse(BaseModel):
    response: AliceResponsePayload
    version: str = "1.0"