from __future__ import annotations

from .architecture import GenerateArchitectureComponent
from .base import BaseComponent
from .graph import (
    BuildGraphDirsComponent,
    BuildGraphFilesComponent,
    BuildGraphLeavesComponent,
    PersistGraphComponent,
)
from .hashes import ComputeHashesComponent, SaveHashCacheComponent, SelectChangedPathsComponent
from .manifest import ManifestComponent
from .scan import ScanRepoComponent
from .summarize import SummarizeFilesComponent

BUILTIN_COMPONENTS = [
    ScanRepoComponent,
    ComputeHashesComponent,
    BuildGraphDirsComponent,
    BuildGraphFilesComponent,
    BuildGraphLeavesComponent,
    PersistGraphComponent,
    ManifestComponent,
    SaveHashCacheComponent,
]

DEFAULT_COMPONENT_NAMES = [component_cls.name for component_cls in BUILTIN_COMPONENTS]

__all__ = [
    "BaseComponent",
    "BuildGraphDirsComponent",
    "BuildGraphFilesComponent",
    "BuildGraphLeavesComponent",
    "BUILTIN_COMPONENTS",
    "ComputeHashesComponent",
    "DEFAULT_COMPONENT_NAMES",
    "GenerateArchitectureComponent",
    "ManifestComponent",
    "PersistGraphComponent",
    "SaveHashCacheComponent",
    "ScanRepoComponent",
    "SelectChangedPathsComponent",
    "SummarizeFilesComponent",
]
