from __future__ import annotations

from .base import BaseAgent
from .factory import get_agent

__all__ = [
    "BaseAgent",
    "get_agent",
    "AnthropicAgent",
    "OllamaAgent",
    "OpenAIAgent",
]


def __getattr__(name: str):
    if name == "AnthropicAgent":
        from .anthropic import AnthropicAgent

        return AnthropicAgent
    if name == "OpenAIAgent":
        from .openai import OpenAIAgent

        return OpenAIAgent
    if name == "OllamaAgent":
        from .ollama import OllamaAgent

        return OllamaAgent
    raise AttributeError(name)
