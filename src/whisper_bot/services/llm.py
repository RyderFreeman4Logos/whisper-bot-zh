import os
import time

import structlog
from litellm import acompletion

from whisper_bot.config import Settings

logger = structlog.get_logger(__name__)


class LLMService:
    def __init__(self, settings: Settings):
        # Support multiple models separated by comma for fallback strategy
        raw_model_config = settings.LLM_MODEL or ""
        self.models = [m.strip() for m in raw_model_config.split(",") if m.strip()]

        self.system_prompt = settings.LLM_SYSTEM_PROMPT
        self.temperature = settings.LLM_TEMPERATURE
        self.top_p = settings.LLM_TOP_P
        self.max_tokens = settings.LLM_MAX_TOKENS

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

    def _map_env_var(self, value: str | None, target_env: str):
        if value:
            os.environ[target_env] = value

    async def refine_text(self, text: str) -> tuple[str, float]:
        """
        Refine the transcribed text using the configured LLM chain.
        Returns: (refined_text, duration_in_seconds)
        """
        if not self.models:
            return text, 0.0

        start_time = time.time()
        last_error = None

        user_content = (
            "润色下面这段转写。严禁添加评论/建议/总结/补充，"
            "严禁改写原意，严禁替说话人补全没说完的话。直接输出润色后的文本：\n\n"
            f"{text}"
        )

        call_kwargs: dict[str, object] = {
            "temperature": self.temperature,
            "num_retries": 3,  # Automatic exponential backoff for temporary errors
        }
        if self.top_p is not None:
            call_kwargs["top_p"] = self.top_p
        if self.max_tokens is not None:
            call_kwargs["max_tokens"] = self.max_tokens

        for model in self.models:
            logger.info(f"Sending text to LLM ({model}) for refinement...")
            try:
                response = await acompletion(
                    model=model,
                    messages=[
                        {"role": "system", "content": self.system_prompt},
                        {"role": "user", "content": user_content},
                    ],
                    **call_kwargs,
                )

                refined_content = response.choices[0].message.content
                self._last_successful_model = model

                duration = time.time() - start_time
                logger.info(f"LLM refinement completed using {model} in {duration:.2f}s.")
                return refined_content, duration

            except Exception as e:
                logger.warning(f"LLM refinement failed with {model}: {e}. Trying next model...")
                last_error = e
                continue

        duration = time.time() - start_time
        logger.error("All LLM models failed.")
        return f"(LLM处理失败: 所有模型均不可用。最后错误: {last_error!s})", duration

    @property
    def is_enabled(self) -> bool:
        return bool(self.models)

    @property
    def model(self) -> str:
        """Returns the primary model or the one that last succeeded."""
        return getattr(self, "_last_successful_model", self.models[0] if self.models else "Unknown")
