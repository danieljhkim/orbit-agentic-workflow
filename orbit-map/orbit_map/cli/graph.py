from __future__ import annotations

import click

from .common import (
    LEAF_KIND_CHOICES,
    NODE_TYPE_CHOICES,
    echo_refs,
    load_graph_context_service,
)
from orbit_map.schemas.graph.contexts import NodeType
from orbit_map.schemas.graph.nodes import LeafKind


@click.group("graph")
def graph() -> None:
    """Inspect the persisted code graph."""


@graph.command("context")
@click.argument("node_id")
@click.option("--repo", default=".", help="Repository root path.")
@click.option(
    "--output", default=".orbit/knowledge", help="Knowledge output directory."
)
def graph_context(node_id: str, repo: str, output: str) -> None:
    """Print an agent-facing context for a graph node."""
    service = load_graph_context_service(repo, output)
    node = service.resolve_selector(node_id)
    click.echo(service.get_context(node.id).model_dump_json(indent=2))


@graph.command("lineage")
@click.argument("node_id")
@click.option("--repo", default=".", help="Repository root path.")
@click.option(
    "--output", default=".orbit/knowledge", help="Knowledge output directory."
)
@click.option(
    "--include-self", is_flag=True, help="Include the requested node in the lineage."
)
def graph_lineage(node_id: str, repo: str, output: str, include_self: bool) -> None:
    """Print the lineage for a graph node."""
    service = load_graph_context_service(repo, output)
    node = service.resolve_selector(node_id)
    echo_refs(service.get_lineage(node.id, include_self=include_self))


@graph.command("children")
@click.argument("node_id")
@click.option("--repo", default=".", help="Repository root path.")
@click.option(
    "--output", default=".orbit/knowledge", help="Knowledge output directory."
)
def graph_children(node_id: str, repo: str, output: str) -> None:
    """Print immediate child nodes for a graph node."""
    service = load_graph_context_service(repo, output)
    node = service.resolve_selector(node_id)
    echo_refs(service.get_children(node.id))


@graph.command("search")
@click.argument("query", required=False, default="")
@click.option("--repo", default=".", help="Repository root path.")
@click.option(
    "--output", default=".orbit/knowledge", help="Knowledge output directory."
)
@click.option(
    "--node-type",
    "node_types",
    multiple=True,
    type=click.Choice(NODE_TYPE_CHOICES),
    help="Filter by node type. May be used multiple times.",
)
@click.option(
    "--leaf-kind",
    "leaf_kinds",
    multiple=True,
    type=click.Choice(LEAF_KIND_CHOICES),
    help="Filter leaf nodes by kind. May be used multiple times.",
)
@click.option(
    "--location-prefix", default=None, help="Filter by graph node location prefix."
)
@click.option(
    "--limit", default=20, show_default=True, help="Maximum number of search results."
)
def graph_search(
    query: str,
    repo: str,
    output: str,
    node_types: tuple[NodeType, ...],
    leaf_kinds: tuple[LeafKind, ...],
    location_prefix: str | None,
    limit: int,
) -> None:
    """Search graph nodes and print lightweight references."""
    service = load_graph_context_service(repo, output)
    echo_refs(
        service.search_nodes(
            query=query,
            node_types=node_types or None,
            leaf_kinds=leaf_kinds or None,
            location_prefix=location_prefix,
            limit=limit,
        )
    )
