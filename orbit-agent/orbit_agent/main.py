from __future__ import annotations

import logging
from pathlib import Path

import click

from orbit_agent.logging_utils import configure_logging
from orbit_agent.pipeline import run_build
from orbit_agent.pipeline.components import DEFAULT_COMPONENT_NAMES
from orbit_agent.pipeline.config import PipelineConfig
from orbit_agent.pipeline.registry import build_default_registry

logger = logging.getLogger(__name__)


@click.group()
@click.option("--debug", is_flag=True, help="Enable debug logging.")
@click.pass_context
def cli(ctx: click.Context, debug: bool) -> None:
    configure_logging(debug=debug)
    ctx.ensure_object(dict)
    ctx.obj["debug"] = debug
    logger.debug("CLI initialized with debug=%s", debug)


@cli.command()
@click.option("--repo", default=".", help="Repository root path.")
@click.option("--output", default=".orbit/knowledge", help="Output directory.")
@click.option(
    "--components",
    default=",".join(DEFAULT_COMPONENT_NAMES),
    help="Comma-separated ordered component names.",
)
def build(repo: str, output: str, components: str) -> None:
    """Scan and build full knowledge base."""
    repo_path, output_dir = _resolve_paths(repo, output)
    logger.info("Starting full knowledge build for %s", repo_path)
    run_build(repo_path, output_dir, incremental=False, config=_parse_pipeline_config(components))
    click.echo(f"Knowledge artifacts written to {output_dir}")


@cli.command()
@click.option("--repo", default=".", help="Repository root path.")
@click.option("--output", default=".orbit/knowledge", help="Output directory.")
@click.option(
    "--components",
    default=",".join(DEFAULT_COMPONENT_NAMES),
    help="Comma-separated ordered component names.",
)
def update(repo: str, output: str, components: str) -> None:
    """Incrementally update knowledge base."""
    repo_path, output_dir = _resolve_paths(repo, output)
    logger.info("Starting incremental knowledge update for %s", repo_path)
    run_build(repo_path, output_dir, incremental=True, config=_parse_pipeline_config(components))
    click.echo(f"Knowledge artifacts updated at {output_dir}")


@cli.command("list-components")
def list_components() -> None:
    """List registered pipeline component names."""
    registry = build_default_registry()
    logger.debug("Listing %d registered components", len(registry.names()))
    for name in registry.names():
        click.echo(name)


def _resolve_paths(repo: str, output: str) -> tuple[Path, Path]:
    repo_path = Path(repo).resolve()
    output_dir = Path(output) if Path(output).is_absolute() else repo_path / output
    logger.debug("Resolved repo_path=%s output_dir=%s", repo_path, output_dir)
    return repo_path, output_dir


def _parse_pipeline_config(components: str) -> PipelineConfig:
    component_names = [name.strip() for name in components.split(",") if name.strip()]
    logger.debug("Parsed component names: %s", component_names)
    return PipelineConfig.from_component_names(component_names)


if __name__ == "__main__":
    cli()
