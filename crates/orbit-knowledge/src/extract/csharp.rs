// ORB-00013: Existing expect calls in this module document local invariants; keep the allow scoped while the workspace lint is ratcheted.
#![allow(clippy::expect_used)]

use tree_sitter::{Node, Parser};

use super::FileExtractor;
use super::common::{
    ExtractedLeaf, ExtractionResult, compute_source_hash, finalize_unique_qualified_names,
};
use super::language::{FileKind, Language};

pub struct CSharpExtractor;

impl FileExtractor for CSharpExtractor {
    fn file_kind(&self) -> FileKind {
        FileKind::Code(Language::CSharp)
    }

    fn extract(&self, source: &str) -> ExtractionResult {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
            .expect("tree-sitter-c-sharp");

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return ExtractionResult::default(),
        };

        let mut leaves = Vec::new();
        extract_children(tree.root_node(), source, &mut leaves, None);
        finalize_unique_qualified_names(&mut leaves);
        ExtractionResult {
            leaves,
            ..Default::default()
        }
    }
}

fn get_name(node: Node, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| node_text(n, source))
        .filter(|name| !name.is_empty())
}

fn node_text(node: Node, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn node_source(node: Node, source: &str) -> String {
    source[node.start_byte()..node.end_byte()]
        .trim_end()
        .to_string()
}

fn qualify_name(parent: Option<&str>, name: &str) -> String {
    match parent {
        Some(parent) => format!("{parent}::{name}"),
        None => name.to_string(),
    }
}

fn extract_children(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: Option<&str>,
) -> Vec<String> {
    let mut children = Vec::new();
    let mut scoped_parent = parent.map(str::to_string);
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }

        if child.kind() == "file_scoped_namespace_declaration" {
            if let Some(qualified_name) =
                extract_file_scoped_namespace(child, source, leaves, parent)
            {
                scoped_parent = Some(qualified_name.clone());
                children.push(qualified_name);
            }
            continue;
        }

        children.extend(extract_node(
            child,
            source,
            leaves,
            scoped_parent.as_deref().or(parent),
        ));
    }

    children
}

fn extract_node(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: Option<&str>,
) -> Vec<String> {
    match node.kind() {
        "namespace_declaration" => extract_namespace(node, source, leaves, parent),
        "class_declaration" => extract_type(node, source, leaves, parent, "class"),
        "struct_declaration" => extract_type(node, source, leaves, parent, "struct"),
        "record_declaration" => extract_type(node, source, leaves, parent, "record"),
        "interface_declaration" => extract_type(node, source, leaves, parent, "interface"),
        "enum_declaration" => extract_type(node, source, leaves, parent, "enum"),
        "method_declaration" | "constructor_declaration" => {
            extract_named_member(node, source, leaves, parent, "method")
        }
        "property_declaration" => extract_named_member(node, source, leaves, parent, "property"),
        "field_declaration" => extract_variable_members(node, source, leaves, parent, "field"),
        "event_declaration" => extract_named_member(node, source, leaves, parent, "event"),
        "event_field_declaration" => {
            extract_variable_members(node, source, leaves, parent, "event")
        }
        "delegate_declaration" => extract_named_member(node, source, leaves, parent, "delegate"),
        "compilation_unit" | "declaration_list" | "declaration" | "type_declaration"
        | "preproc_if" | "preproc_elif" | "preproc_else" => {
            extract_children(node, source, leaves, parent)
        }
        _ => Vec::new(),
    }
}

fn extract_namespace(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: Option<&str>,
) -> Vec<String> {
    let Some(name) = get_name(node, source) else {
        return Vec::new();
    };
    let qualified_name = qualify_name(parent, &name);
    let children = node
        .child_by_field_name("body")
        .map(|body| extract_children(body, source, leaves, Some(&qualified_name)))
        .unwrap_or_default();

    push_leaf(
        node,
        source,
        leaves,
        &name,
        &qualified_name,
        "namespace",
        parent,
        children,
    );

    vec![qualified_name]
}

fn extract_file_scoped_namespace(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: Option<&str>,
) -> Option<String> {
    let name = get_name(node, source)?;
    let qualified_name = qualify_name(parent, &name);
    push_leaf(
        node,
        source,
        leaves,
        &name,
        &qualified_name,
        "namespace",
        parent,
        Vec::new(),
    );
    Some(qualified_name)
}

fn extract_type(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: Option<&str>,
    kind: &str,
) -> Vec<String> {
    let Some(name) = get_name(node, source) else {
        return Vec::new();
    };
    let qualified_name = qualify_name(parent, &name);
    let children = node
        .child_by_field_name("body")
        .map(|body| extract_children(body, source, leaves, Some(&qualified_name)))
        .unwrap_or_default();

    push_leaf(
        node,
        source,
        leaves,
        &name,
        &qualified_name,
        kind,
        parent,
        children,
    );

    vec![qualified_name]
}

fn extract_named_member(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: Option<&str>,
    kind: &str,
) -> Vec<String> {
    let Some(name) = get_name(node, source) else {
        return Vec::new();
    };
    let qualified_name = qualify_name(parent, &name);
    push_leaf(
        node,
        source,
        leaves,
        &name,
        &qualified_name,
        kind,
        parent,
        Vec::new(),
    );
    vec![qualified_name]
}

fn extract_variable_members(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: Option<&str>,
    kind: &str,
) -> Vec<String> {
    let mut children = Vec::new();
    for name in variable_names(node, source) {
        let qualified_name = qualify_name(parent, &name);
        push_leaf(
            node,
            source,
            leaves,
            &name,
            &qualified_name,
            kind,
            parent,
            Vec::new(),
        );
        children.push(qualified_name);
    }
    children
}

fn variable_names(node: Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        if child.kind() != "variable_declaration" {
            continue;
        }

        let mut declaration_cursor = child.walk();
        for declarator in child.named_children(&mut declaration_cursor) {
            if declarator.kind() != "variable_declarator" {
                continue;
            }
            if let Some(name) = get_name(declarator, source) {
                names.push(name);
            }
        }
    }

    names
}

#[allow(clippy::too_many_arguments)]
fn push_leaf(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    name: &str,
    qualified_name: &str,
    kind: &str,
    parent: Option<&str>,
    children: Vec<String>,
) {
    let src = node_source(node, source);
    leaves.push(ExtractedLeaf {
        qualified_name: qualified_name.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        source: src.clone(),
        source_hash: compute_source_hash(&src),
        parent_qualified_name: parent.map(str::to_string),
        children_qualified_names: children,
        depth: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        r#"using System;

namespace Orbit.Sample
{
    public delegate void AccountChanged(object sender, EventArgs args);

    public interface IAccountRepository
    {
        event EventHandler Loaded;
        string Name { get; }
        void Save(Account account);
    }

    public enum AccountStatus
    {
        Active,
        Suspended
    }

    public struct AccountKey
    {
        public Guid Value { get; }
    }

    public record AccountSnapshot(string Id, AccountStatus Status);

    public class AccountService : IAccountRepository
    {
        private readonly string _prefix = "acct";
        public event EventHandler Changed;
        public event EventHandler Loaded { add { } remove { } }
        public string Name { get; private set; }

        public Account CreateAccount(string id)
        {
            return new Account(id);
        }
    }
}"#
    }

    fn leaf<'a>(
        leaves: &'a [ExtractedLeaf],
        qualified_name: &str,
        kind: &str,
    ) -> &'a ExtractedLeaf {
        leaves
            .iter()
            .find(|leaf| leaf.qualified_name == qualified_name && leaf.kind == kind)
            .unwrap_or_else(|| panic!("missing {kind} leaf {qualified_name} in {leaves:#?}"))
    }

    #[test]
    fn file_kind_is_csharp() {
        assert_eq!(
            CSharpExtractor.file_kind(),
            FileKind::Code(Language::CSharp)
        );
    }

    #[test]
    fn extracts_required_csharp_symbols() {
        let result = CSharpExtractor.extract(fixture());
        let leaves = result.leaves;

        let namespace = leaf(&leaves, "Orbit.Sample", "namespace");
        assert_eq!(namespace.name, "Orbit.Sample");
        assert_eq!(namespace.start_line, 3);
        assert_eq!(namespace.end_line, 39);
        assert!(
            namespace
                .children_qualified_names
                .contains(&"Orbit.Sample::AccountService".to_string())
        );

        let delegate = leaf(&leaves, "Orbit.Sample::AccountChanged", "delegate");
        assert_eq!(delegate.name, "AccountChanged");
        assert_eq!(delegate.start_line, 5);
        assert!(
            delegate
                .source
                .trim_start()
                .starts_with("public delegate void")
        );

        let interface = leaf(&leaves, "Orbit.Sample::IAccountRepository", "interface");
        assert_eq!(interface.start_line, 7);
        assert_eq!(interface.end_line, 12);

        let interface_event = leaf(&leaves, "Orbit.Sample::IAccountRepository::Loaded", "event");
        assert_eq!(interface_event.start_line, 9);
        assert_eq!(
            interface_event.parent_qualified_name.as_deref(),
            Some("Orbit.Sample::IAccountRepository")
        );

        let interface_property = leaf(
            &leaves,
            "Orbit.Sample::IAccountRepository::Name",
            "property",
        );
        assert_eq!(interface_property.start_line, 10);

        let interface_method = leaf(&leaves, "Orbit.Sample::IAccountRepository::Save", "method");
        assert_eq!(interface_method.start_line, 11);
        assert!(
            interface_method
                .source
                .trim_start()
                .starts_with("void Save")
        );

        let enum_leaf = leaf(&leaves, "Orbit.Sample::AccountStatus", "enum");
        assert_eq!(enum_leaf.start_line, 14);
        assert_eq!(enum_leaf.end_line, 18);

        let struct_leaf = leaf(&leaves, "Orbit.Sample::AccountKey", "struct");
        assert_eq!(struct_leaf.start_line, 20);
        assert_eq!(struct_leaf.end_line, 23);

        let struct_property = leaf(&leaves, "Orbit.Sample::AccountKey::Value", "property");
        assert_eq!(struct_property.start_line, 22);

        let record_leaf = leaf(&leaves, "Orbit.Sample::AccountSnapshot", "record");
        assert_eq!(record_leaf.start_line, 25);
        assert_eq!(record_leaf.end_line, 25);

        let class_leaf = leaf(&leaves, "Orbit.Sample::AccountService", "class");
        assert_eq!(class_leaf.start_line, 27);
        assert_eq!(class_leaf.end_line, 38);
        assert!(
            class_leaf
                .children_qualified_names
                .contains(&"Orbit.Sample::AccountService::CreateAccount".to_string())
        );

        let field = leaf(&leaves, "Orbit.Sample::AccountService::_prefix", "field");
        assert_eq!(field.start_line, 29);
        assert!(
            field
                .source
                .trim_start()
                .starts_with("private readonly string")
        );

        let event_field = leaf(&leaves, "Orbit.Sample::AccountService::Changed", "event");
        assert_eq!(event_field.start_line, 30);

        let event_property = leaf(&leaves, "Orbit.Sample::AccountService::Loaded", "event");
        assert_eq!(event_property.start_line, 31);

        let property = leaf(&leaves, "Orbit.Sample::AccountService::Name", "property");
        assert_eq!(property.start_line, 32);
        assert!(
            property
                .source
                .trim_start()
                .starts_with("public string Name")
        );

        let method = leaf(
            &leaves,
            "Orbit.Sample::AccountService::CreateAccount",
            "method",
        );
        assert_eq!(method.start_line, 34);
        assert_eq!(method.end_line, 37);
        assert!(
            method
                .source
                .trim_start()
                .starts_with("public Account CreateAccount")
        );
    }

    #[test]
    fn file_scoped_namespace_parents_following_declarations() {
        let source = r#"namespace Orbit.FileScoped;

public class FileScopedService
{
    public string Name { get; }
}"#;

        let result = CSharpExtractor.extract(source);
        let leaves = result.leaves;

        let namespace = leaf(&leaves, "Orbit.FileScoped", "namespace");
        assert_eq!(namespace.start_line, 1);
        assert_eq!(namespace.end_line, 1);

        let class = leaf(&leaves, "Orbit.FileScoped::FileScopedService", "class");
        assert_eq!(
            class.parent_qualified_name.as_deref(),
            Some("Orbit.FileScoped")
        );

        let property = leaf(
            &leaves,
            "Orbit.FileScoped::FileScopedService::Name",
            "property",
        );
        assert_eq!(
            property.parent_qualified_name.as_deref(),
            Some("Orbit.FileScoped::FileScopedService")
        );
    }
}
