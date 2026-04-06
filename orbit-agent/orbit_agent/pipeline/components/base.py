from __future__ import annotations

from abc import ABC, abstractmethod

from orbit_agent.pipeline.context import PipelineContext


class BaseComponent(ABC):
    name = "base"

    @abstractmethod
    def execute(self, context: PipelineContext) -> PipelineContext:
        """Run this component against the shared pipeline context."""
