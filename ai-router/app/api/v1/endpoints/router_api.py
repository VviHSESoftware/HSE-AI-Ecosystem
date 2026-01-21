from fastapi import APIRouter, HTTPException, Depends
from app.api.v1.schemas import RouterRequest, RouterResponse
from app.core.security import verify_api_key
from app.core.timer import timer
from app.services.router_engine import router_engine

router = APIRouter()


@router.post("/process", response_model=RouterResponse, dependencies=[Depends(verify_api_key)])
async def process_request(request: RouterRequest):
    with timer("Router Endpoint"):
        try:
            msgs = [m.model_dump() for m in request.messages]

            with timer("Router route module"):
                result = await router_engine.route_and_process(
                    messages=msgs,
                    user_email=request.user_email,
                    integrate_links=request.integrate_links_in_text
                )
            return RouterResponse(
                content=result["response"],
                sources=result["sources"],
                module_used=result["module_used"]
            )
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
