from __future__ import annotations

import logging

import click

from orbit_map.pipeline.config import PipelineConfig
from orbit_map.pipeline.engine import run_build

from .common import BUILD_TARGET_CHOICES, build_component_names, resolve_paths, target_label

logger = logging.getLogger(__name__)


def register(cli: click.Group) -> None:
    cli.add_command(build)
    cli.add_command(update)


@click.command()
@click.argument("target", type=click.Choice(BUILD_TARGET_CHOICES))
@click.option("--repo", default=".", help="Repository root path.")
@click.option("--output", default=".orbit/knowledge", help="Output directory.")
def build(target: str, repo: str, output: str) -> None:
    """Build a knowledge artifact."""
    repo_path, output_dir = resolve_paths(repo, output)
    component_names = build_component_names(target, output_dir)
    label = target_label(target)
    logger.info("Starting %s build for %s", label, repo_path)
    run_build(
        repo_path,
        output_dir,
        incremental=False,
        config=PipelineConfig.from_component_names(component_names),
    )
    click.echo(f"{label.title()} artifacts written to {output_dir}")


@click.command()
@click.argument("target", type=click.Choice(BUILD_TARGET_CHOICES))
@click.option("--repo", default=".", help="Repository root path.")
@click.option("--output", default=".orbit/knowledge", help="Output directory.")
def update(target: str, repo: str, output: str) -> None:
    """Update a knowledge artifact."""
    repo_path, output_dir = resolve_paths(repo, output)
    component_names = build_component_names(target, output_dir)
    label = target_label(target)
    logger.info("Starting %s update for %s", label, repo_path)
    run_build(
        repo_path,
        output_dir,
        incremental=True,
        config=PipelineConfig.from_component_names(component_names),
    )
    click.echo(f"{label.title()} artifacts updated at {output_dir}")
