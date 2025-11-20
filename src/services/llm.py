import os
import structlog
from typing import Optional
from litellm import acompletion

from src.config import Settings

logger = structlog.get_logger(__name__)

class LLMService:
    def __init__(self, settings: Settings):
        self.model = settings.LLM_MODEL
        self.system_prompt = settings.LLM_SYSTEM_PROMPT
        
        # Map custom config keys to standard env vars expected by libraries/litellm
        self._map_env_var(settings.ANTHROPIC_API, "ANTHROPIC_API_KEY")
        self._map_env_var(settings.GEMINI_API, "GEMINI_API_KEY")
        self._map_env_var(settings.GROQ_API, "GROQ_API_KEY")
        self._map_env_var(settings.XAI_API, "XAI_API_KEY")
        # Zenmux usually provides an OpenAI-compatible endpoint, 
        # but if litellm supports it directly or via custom provider, we set it here.
        # Assuming generic mapping for now.
        self._map_env_var(settings.ZENMUX_API, "ZENMUX_API_KEY")
        
        if self.model:
            logger.info(f"LLM Service initialized with model: {self.model}")
        else:
            logger.info("LLM Service disabled (LLM_MODEL not set).")

    def _map_env_var(self, value: Optional[str], target_env: str):
        if value:
            os.environ[target_env] = value

    async def refine_text(self, text: str) -> str:
        """
        Refine the transcribed text using the configured LLM.
        """
        if not self.model:
            return text

        logger.info("Sending text to LLM for refinement...")
        try:
            response = await acompletion(
                model=self.model,
                messages=[
                    {"role": "system", "content": self.system_prompt},
                    {"role": "user", "content": text}
                ],
                temperature=0.3, # Low temperature for fidelity
            )
            
            refined_content = response.choices[0].message.content
            logger.info("LLM refinement completed.")
            return refined_content
        except Exception as e:
            logger.error(f"LLM refinement failed: {e}")
            return f"(LLM处理失败: {str(e)})"

    @property
    def is_enabled(self) -> bool:
        return bool(self.model)
