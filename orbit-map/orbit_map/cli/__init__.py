from __future__ import annotations

import click

from orbit_map.logging_utils import configure_logging

from .build import build, update
from .graph import graph
from .knowledge import knowledge


@click.group()
@click.option("--debug", is_flag=True, help="Enable debug logging.")
@click.pass_context
def cli(ctx: click.Context, debug: bool) -> None:
    configure_logging(debug=debug)
    ctx.ensure_object(dict)
    ctx.obj["debug"] = debug


cli.add_command(build)
cli.add_command(update)
cli.add_command(graph)
cli.add_command(knowledge)

__all__ = ["cli"]
