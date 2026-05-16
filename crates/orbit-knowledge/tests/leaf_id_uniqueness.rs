#![allow(missing_docs)]
// ORB-00013: Tests use unwrap/expect to keep fixture setup readable.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::{HashMap, HashSet};
use std::path::Path;

use orbit_knowledge::graph::object_store::RefName;
use orbit_knowledge::pipeline::context::BuildConfig;
use tempfile::tempdir;

#[test]
fn fixture_leaf_ids_are_unique_per_extracted_file() {
    let repo_dir = tempdir().expect("repo tempdir");
    let repo = repo_dir.path();

    let fixtures = [
        (
            "src/python_fixture.py",
            r#"
class Alpha:
    def save(self):
        return "alpha"

    class Inner:
        def save(self):
            return "inner"

class Beta:
    def save(self):
        return "beta"
"#,
        ),
        (
            "src/rust_fixture.rs",
            r#"
trait Runner {
    fn run(&self);
}

struct Foo;

impl Foo {
    fn run(&self) {}
}

impl Runner for Foo {
    fn run(&self) {}
}
"#,
        ),
        (
            "src/java_fixture.java",
            r#"
class Client {
    void connect(int port) {}
    void connect(int port, String host) {}
}
"#,
        ),
        (
            "src/typescript_fixture.ts",
            r#"
function pick(value: string): string;
function pick(value: number): number;
function pick(value: string | number) {
    return value;
}

class Store {
    get(value: string): string;
    get(value: number): number;
    get(value: string | number) {
        return String(value);
    }
}
"#,
        ),
        (
            "src/javascript_fixture.js",
            r#"
class Worker {
    save() {}
    save(value) {}
}
function load() {}
function load() {}
"#,
        ),
        (
            "src/go_fixture.go",
            r#"
package main

func Save() {}
func Save() {}
"#,
        ),
        (
            "src/kotlin_fixture.kt",
            r#"
class Client {
    fun connect(port: Int) {}
    fun connect(host: String) {}
}
"#,
        ),
        (
            "src/ruby_fixture.rb",
            r#"
class Invoice
  def total
  end

  def total
  end
end
"#,
        ),
        (
            "src/c_fixture.c",
            r#"
void reset(void) {}
void reset(int value) {}
"#,
        ),
        (
            "src/csharp_fixture.cs",
            r#"
class Client {
    void Connect(int port) {}
    void Connect(string host) {}
}
"#,
        ),
    ];

    for (path, source) in fixtures {
        write_file(repo, path, source);
    }

    let knowledge_root = tempdir().expect("knowledge tempdir");
    let ctx = orbit_knowledge::pipeline::run_build(BuildConfig {
        repo_path: repo.to_path_buf(),
        output_dir: knowledge_root.path().join("knowledge"),
        incremental: false,
        ref_name: Some(RefName::new("main").expect("valid ref")),
    })
    .expect("pipeline build succeeds");

    let mut leaves_by_file: HashMap<&str, Vec<&str>> = HashMap::new();
    for leaf in &ctx.graph.leaves {
        let (file_path, _) = leaf
            .base
            .location
            .split_once('#')
            .expect("leaf location includes qualified-name separator");
        leaves_by_file
            .entry(file_path)
            .or_default()
            .push(leaf.base.id.as_str());
    }

    for (path, _) in fixtures {
        let leaf_ids = leaves_by_file
            .get(path)
            .unwrap_or_else(|| panic!("fixture `{path}` produced no leaves"));
        let unique_ids: HashSet<&str> = leaf_ids.iter().copied().collect();
        assert_eq!(
            unique_ids.len(),
            leaf_ids.len(),
            "duplicate leaf IDs emitted for fixture `{path}`"
        );
    }
}

fn write_file(repo: &Path, rel: &str, content: &str) {
    let path = repo.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture parent");
    }
    std::fs::write(path, content).expect("write fixture file");
}
