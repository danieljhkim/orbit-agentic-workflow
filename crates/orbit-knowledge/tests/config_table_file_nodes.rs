#![allow(missing_docs)]
// ORB-00013: Tests use unwrap/expect to keep fixture setup readable.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use orbit_knowledge::Selector;
use orbit_knowledge::graph::GraphNodeRef;
use orbit_knowledge::graph::object_store::RefName;
use orbit_knowledge::pipeline::context::BuildConfig;
use orbit_knowledge::service::GraphContextService;
use tempfile::tempdir;

#[test]
fn config_and_table_files_are_file_level_graph_nodes() {
    let repo_dir = tempdir().expect("repo tempdir");
    let repo = repo_dir.path();
    write_file(repo, "settings.yaml", "name: orbit\nversion: 1\n");
    write_file(
        repo,
        "package.json",
        "{\"name\":\"orbit\",\"version\":\"1.0.0\"}\n",
    );
    write_file(repo, "Cargo.toml", "[package]\nname = \"orbit\"\n");
    write_file(repo, "people.csv", "id,name\n1,Alice\n");
    write_file(repo, "people.tsv", "id\tname\n1\tAlice\n");

    let knowledge_root = tempdir().expect("knowledge tempdir");
    let ctx = run_build(repo, knowledge_root.path());
    let service = GraphContextService::new(&ctx.graph);

    assert!(ctx.graph.leaves.is_empty());
    for location in [
        "settings.yaml",
        "package.json",
        "Cargo.toml",
        "people.csv",
        "people.tsv",
    ] {
        let file = ctx
            .graph
            .files
            .iter()
            .find(|file| file.base.location == location)
            .unwrap_or_else(|| panic!("missing file node {location}"));
        assert!(
            file.leaf_children.is_empty(),
            "expected no leaves for {location}"
        );
        assert!(
            !file.source.is_empty() || file.source_blob_hash.is_some(),
            "expected file-level source for {location}"
        );
    }

    let search_results = service.search_structured("package.json", Some(&["file"]), None, None, 10);
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].selector, "file:package.json");
    assert_eq!(search_results[0].kind, "file");

    let selector: Selector = "file:package.json".parse().expect("selector parses");
    let node = service
        .resolve_selector(&selector)
        .expect("file selector resolves");
    let GraphNodeRef::File(file) = node else {
        panic!("package.json selector did not resolve to a file node");
    };
    assert!(file.source.contains("\"name\":\"orbit\""));
}

fn run_build(
    repo: &Path,
    output_dir: &Path,
) -> orbit_knowledge::pipeline::context::PipelineContext {
    let config = BuildConfig {
        repo_path: repo.to_path_buf(),
        output_dir: output_dir.to_path_buf(),
        incremental: false,
        ref_name: Some(RefName::new("main").expect("valid ref")),
    };
    orbit_knowledge::pipeline::run_build(config).expect("pipeline runs")
}

fn write_file(repo: &Path, rel: &str, content: &str) {
    let path = repo.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dir");
    }
    std::fs::write(path, content).expect("write fixture file");
}
