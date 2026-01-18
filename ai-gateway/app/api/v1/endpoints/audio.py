import logging
from fastapi import APIRouter, Depends, UploadFile, File, HTTPException
from app.services.gateway_service import gateway_service
from app.core.security import verify_api_key

logger = logging.getLogger(__name__)

router = APIRouter()

@router.post("/asr", dependencies=[Depends(verify_api_key)])
async def transcribe_audio(file: UploadFile = File(...)):
    try:
        content = await file.read()
        return await gateway_service.transcribe_audio(content, file.filename)
    except Exception as e:
        logger.error(f"ASR Provider Error: {str(e)}", exc_info=True)
        raise HTTPException(status_code=502, detail=f"ASR Provider Error: {str(e)}")