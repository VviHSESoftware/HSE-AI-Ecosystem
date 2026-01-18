import logging
from fastapi import APIRouter, Depends, HTTPException
from app.api.v1.schemas import (
    LLMRequest, LLMResponse,
    EmbeddingRequest, EmbeddingResponse,
    VLMRequest
)
from app.services.gateway_service import gateway_service
from app.core.security import verify_api_key

logger = logging.getLogger(__name__)

router = APIRouter()


@router.post("/llm", response_model=LLMResponse, dependencies=[Depends(verify_api_key)])
async def generate_text(request: LLMRequest):
    try:
        payload = request.model_dump(exclude={"mode"})
        raw = await gateway_service.chat_completion(payload, request.mode)

        choice = raw["choices"][0]["message"]
        content = choice.get("content") or ""

        return LLMResponse(
            content=content,
            model=raw["model"],
            usage=raw.get("usage", {})
        )
    except Exception as e:
        logger.error(f"LLM Provider Error: {str(e)}", exc_info=True)
        raise HTTPException(status_code=502, detail=f"LLM Provider Error: {str(e)}")


@router.post("/embeddings", response_model=EmbeddingResponse, dependencies=[Depends(verify_api_key)])
async def create_embeddings(request: EmbeddingRequest):
    try:
        raw = await gateway_service.create_embeddings(request.input)

        return EmbeddingResponse(
            embeddings=[d["embedding"] for d in raw["data"]],
            model=raw["model"],
            usage=raw.get("usage", {})
        )
    except Exception as e:
        logger.error(f"Embedding Provider Error: {str(e)}", exc_info=True)
        raise HTTPException(status_code=502, detail=f"Embedding Provider Error: {str(e)}")


@router.post("/vlm", response_model=LLMResponse, dependencies=[Depends(verify_api_key)])
async def vlm_chat(request: VLMRequest):
    try:
        raw = await gateway_service.vlm_analyze(
            request.text,
            request.image_url,
            request.temperature,
            request.max_tokens
        )

        return LLMResponse(
            content=raw["choices"][0]["message"]["content"],
            model=raw["model"],
            usage=raw.get("usage", {})
        )
    except Exception as e:
        logger.error(f"VLM Provider Error: {str(e)}", exc_info=True)
        raise HTTPException(status_code=502, detail=f"VLM Provider Error: {str(e)}")