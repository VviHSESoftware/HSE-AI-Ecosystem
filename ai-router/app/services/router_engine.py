import logging
import json
from typing import List, Dict, Any

from app.core.config import settings
from app.core.timer import timer
from app.services.gateway_client import gateway_client
from app.modules.base import BaseModule
from app.modules.chitchat import StudentChitchatModule
from app.modules.rag_course import RagCourseModule
from app.modules.lna import LnaModule

logger = logging.getLogger(__name__)


class RouterEngine:
    def __init__(self):
        self.modules: Dict[str, BaseModule] = {}
        self._register_defaults()

    def _register_defaults(self):
        self.register_module(StudentChitchatModule())
        #self.register_module(TeacherChitchatModule())
        self.register_module(RagCourseModule())
        self.register_module(LnaModule())

    def register_module(self, module: BaseModule):
        self.modules[module.name] = module

    def _build_classification_prompt(self, user_query: str) -> str:
        tools_desc = "\n".join([f"- {m.name}: {m.description}" for m in self.modules.values()])

        return f"""
You are a semantic router for a university AI assistant.
Analyze the user's input and select the most appropriate tool to handle it.

Available Tools:
{tools_desc}

Input: "{user_query}"

Return ONLY a JSON object with a single key "tool" containing the name of the selected tool.
Example: {{"tool": "student_chitchat"}}
"""

    async def route_and_process(self, messages: List[Dict[str, str]], user_email: str, integrate_links: bool) -> Dict[str, Any]:
        last_user_message = next((m["content"] for m in reversed(messages) if m["role"] == "user"), "")

        if not last_user_message:
            return {"error": "No user message found"}

        prompt = self._build_classification_prompt(last_user_message)

        try:
            with timer("Router Decision making"):
                raw_decision = await gateway_client.chat_completion(
                    messages=[{"role": "user", "content": prompt}],
                    mode=settings.ROUTER_MODEL_MODE,
                    temperature=0.0
                )

            clean_json = raw_decision.replace("```json", "").replace("```", "").strip()
            decision = json.loads(clean_json)
            tool_name = decision.get("tool")

            logger.info(f"Router Decision: {tool_name} for query '{last_user_message[:50]}...'")

        except Exception as e:
            logger.error(f"Routing failed: {e}. Fallback to student_chitchat.")
            tool_name = "student_chitchat"

        module = self.modules.get(tool_name)
        if not module:
            logger.warning(f"Tool {tool_name} not found. Fallback.")
            module = self.modules["student_chitchat"]

        result_text, sources = await module.process(messages, user_email, integrate_links)

        return {
            "response": result_text,
            "sources": sources,
            "module_used": module.name
        }


router_engine = RouterEngine()