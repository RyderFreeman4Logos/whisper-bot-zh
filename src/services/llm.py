import os
import structlog
from typing import Optional
from litellm import acompletion

from src.config import Settings

logger = structlog.get_logger(__name__)

class LLMService:
    def __init__(self, settings: Settings):
        # Support multiple models separated by comma for fallback strategy
        raw_model_config = settings.LLM_MODEL or ""
        self.models = [m.strip() for m in raw_model_config.split(',') if m.strip()]
        
        self.system_prompt = settings.LLM_SYSTEM_PROMPT
        
        # Map custom config keys to standard env vars expected by libraries/litellm
        self._map_env_var(settings.ANTHROPIC_API, "ANTHROPIC_API_KEY")
        self._map_env_var(settings.GEMINI_API, "GEMINI_API_KEY")
        self._map_env_var(settings.GROQ_API, "GROQ_API_KEY")
        self._map_env_var(settings.XAI_API, "XAI_API_KEY")
        # Zenmux usually provides an OpenAI-compatible endpoint
        self._map_env_var(settings.ZENMUX_API, "ZENMUX_API_KEY")
        
        if self.models:
            logger.info(f"LLM Service initialized with models chain: {self.models}")
        else:
            logger.info("LLM Service disabled (LLM_MODEL not set).")

    def _map_env_var(self, value: Optional[str], target_env: str):
        if value:
            os.environ[target_env] = value

    async def refine_text(self, text: str) -> str:
        """
        Refine the transcribed text using the configured LLM chain.
        Iterates through models until one succeeds.
        """
        if not self.models:
            return text

        last_error = None

        for model in self.models:
            logger.info(f"Sending text to LLM ({model}) for refinement...")
            try:
                response = await acompletion(
                    model=model,
                    messages=[
                        {"role": "system", "content": self.system_prompt},
                        {"role": "user", "content": text}
                    ],
                    temperature=0.3, # Low temperature for fidelity
                    num_retries=3    # Automatic exponential backoff for temporary errors
                )
                
                refined_content = response.choices[0].message.content
                # Append model name metadata here? 
                # The handler expects just text, but we want to know WHICH model succeeded.
                # We can return a tuple or just the text. 
                # The handler uses `llm_service.model` to print metadata. 
                # We should update state or return info.
                # Let's store the successful model to be accessed by handler.
                self._last_successful_model = model 
                
                logger.info(f"LLM refinement completed using {model}.")
                return refined_content

            except Exception as e:
                logger.warning(f"LLM refinement failed with {model}: {e}. Trying next model...")
                last_error = e
                continue
        
        logger.error("All LLM models failed.")
        return f"(LLM处理失败: 所有模型均不可用。最后错误: {str(last_error)})"

    @property
    def is_enabled(self) -> bool:
        return bool(self.models)
    
    @property
    def model(self) -> str:
        """Returns the primary model or the one that last succeeded."""
        return getattr(self, '_last_successful_model', self.models[0] if self.models else "Unknown")
