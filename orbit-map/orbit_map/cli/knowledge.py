from __future__ import annotations

import logging

import click

from orbit_map.service.bootstrap import render_knowledge_bootstrap
from orbit_map.service.lineage_pack import render_lineage_pack

from .common import BOOTSTRAP_FORMAT_CHOICES, PACK_FORMAT_CHOICES, resolve_paths

logger = logging.getLogger(__name__)


@click.group("knowledge")
def knowledge() -> None:
    """Render knowledge-oriented views from persisted artifacts."""


@knowledge.command("bootstrap")
@click.option("--repo", default=".", help="Repository root path.")
@click.option(
    "--output", default=".orbit/knowledge", help="Knowledge output directory."
)
@click.option(
    "--format",
    "output_format",
    type=click.Choice(BOOTSTRAP_FORMAT_CHOICES),
    default="markdown",
    show_default=True,
)
@click.option(
    "--budget",
    type=int,
    default=12000,
    show_default=True,
    help="Approximate markdown character budget for the rendered bootstrap.",
)
@click.option(
    "--include-source",
    is_flag=True,
    help="Include source excerpts for leaf nodes when available.",
)
@click.option(
    "--source-budget",
    type=int,
    default=0,
    show_default=True,
    help="Maximum characters of leaf source excerpts to include.",
)
def knowledge_bootstrap(
    repo: str,
    output: str,
    output_format: str,
    budget: int,
    include_source: bool,
    source_budget: int,
) -> None:
    """Render a deterministic whole-codebase bootstrap."""
    _, output_dir = resolve_paths(repo, output)
    logger.info("Rendering knowledge bootstrap from %s", output_dir)
    click.echo(
        render_knowledge_bootstrap(
            output_dir,
            format=output_format,
            budget=budget,
            include_source=include_source,
            source_budget=source_budget,
        )
    )


@knowledge.command("pack")
@click.argument("selectors", nargs=-1)
@click.option("--repo", default=".", help="Repository root path.")
@click.option(
    "--output", default=".orbit/knowledge", help="Knowledge output directory."
)
@click.option(
    "--format",
    "output_format",
    type=click.Choice(PACK_FORMAT_CHOICES),
    default="markdown",
    show_default=True,
)
@click.option(
    "--depth",
    type=int,
    default=2,
    show_default=True,
    help="Maximum ancestor lineage depth to include for each selected node.",
)
@click.option(
    "--siblings",
    type=int,
    default=2,
    show_default=True,
    help="Maximum sibling nodes to include around each focus node.",
)
@click.option(
    "--children",
    type=int,
    default=4,
    show_default=True,
    help="Maximum direct child nodes to include around each focus node.",
)
@click.option(
    "--budget",
    type=int,
    default=12000,
    show_default=True,
    help="Approximate markdown character budget for the rendered pack.",
)
@click.option(
    "--detail",
    is_flag=True,
    help="Render the richer lineage-pack schema instead of the compact default.",
)
@click.option(
    "--include-source",
    is_flag=True,
    help="Include source excerpts for leaf nodes when available.",
)
@click.option(
    "--source-budget",
    type=int,
    default=0,
    show_default=True,
    help="Maximum characters of leaf source excerpts to include.",
)
def knowledge_pack(
    selectors: tuple[str, ...],
    repo: str,
    output: str,
    output_format: str,
    depth: int,
    siblings: int,
    children: int,
    budget: int,
    detail: bool,
    include_source: bool,
    source_budget: int,
) -> None:
    """Render a deterministic lineage-specific knowledge pack."""
    if not selectors:
        raise click.UsageError("At least one context selector is required.")
    _, output_dir = resolve_paths(repo, output)
    logger.info("Rendering lineage pack from %s", output_dir)
    click.echo(
        render_lineage_pack(
            output_dir,
            selectors,
            format=output_format,
            budget=budget,
            depth=depth,
            siblings=siblings,
            children=children,
            detail=detail,
            include_source=include_source,
            source_budget=source_budget,
        )
    )
