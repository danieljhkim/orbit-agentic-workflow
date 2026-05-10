use tree_sitter::{Language as TreeSitterLanguage, Node, Parser};

use super::FileExtractor;
use super::common::{
    ExtractedLeaf, ExtractionResult, compute_source_hash, finalize_unique_qualified_names,
};
use super::language::{FileKind, Language};

pub struct TypeScriptExtractor {
    language: Language,
}

impl TypeScriptExtractor {
    pub fn new(language: Language) -> Self {
        debug_assert!(matches!(language, Language::TypeScript | Language::Tsx));
        Self { language }
    }
}

impl FileExtractor for TypeScriptExtractor {
    fn file_kind(&self) -> FileKind {
        FileKind::Code(self.language)
    }

    fn extract(&self, source: &str) -> ExtractionResult {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_language(self.language))
            .expect("tree-sitter-typescript");

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return ExtractionResult::default(),
        };

        let mut leaves = Vec::new();
        extract_top_level(tree.root_node(), source, &mut leaves);
        finalize_unique_qualified_names(&mut leaves);
        ExtractionResult {
            leaves,
            ..Default::default()
        }
    }
}

fn tree_sitter_language(language: Language) -> TreeSitterLanguage {
    match language {
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        _ => unreachable!("TypeScriptExtractor only supports TypeScript and TSX"),
    }
}

fn get_name(node: Node, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| n.utf8_text(source.as_bytes()).unwrap_or("").to_string())
        .filter(|name| !name.is_empty())
}

fn get_simple_binding_name(node: Node, source: &str) -> Option<String> {
    let name_node = node.child_by_field_name("name")?;
    if name_node.kind() != "identifier" {
        return None;
    }
    let name = name_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .to_string();
    (!name.is_empty()).then_some(name)
}

fn node_source(node: Node, source: &str) -> String {
    source[node.start_byte()..node.end_byte()]
        .trim_end()
        .to_string()
}

fn extract_top_level(node: Node, source: &str, leaves: &mut Vec<ExtractedLeaf>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let declaration = unwrap_declaration(child);
        match declaration.kind() {
            "function_declaration" | "function_signature" | "generator_function_declaration" => {
                extract_function(declaration, source, leaves)
            }
            "class_declaration" | "abstract_class_declaration" => {
                extract_class(declaration, source, leaves)
            }
            "interface_declaration" => {
                extract_named_declaration(declaration, source, leaves, "interface")
            }
            "type_alias_declaration" => {
                extract_named_declaration(declaration, source, leaves, "type_alias")
            }
            "enum_declaration" => extract_named_declaration(declaration, source, leaves, "enum"),
            "lexical_declaration" | "variable_declaration" => {
                extract_binding_declaration(declaration, source, leaves)
            }
            _ => {}
        }
    }
}

fn unwrap_declaration(node: Node) -> Node {
    match node.kind() {
        "export_statement" => node
            .child_by_field_name("declaration")
            .map(unwrap_declaration)
            .unwrap_or(node),
        "ambient_declaration" => first_named_child(node)
            .map(unwrap_declaration)
            .unwrap_or(node),
        _ => node,
    }
}

fn first_named_child(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).next()
}

fn extract_function(node: Node, source: &str, leaves: &mut Vec<ExtractedLeaf>) {
    let Some(name) = get_name(node, source) else {
        return;
    };
    push_leaf(
        node,
        source,
        leaves,
        &name,
        &name,
        "function",
        None,
        Vec::new(),
    );
}

fn extract_named_declaration(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    kind: &str,
) {
    let Some(name) = get_name(node, source) else {
        return;
    };
    push_leaf(node, source, leaves, &name, &name, kind, None, Vec::new());
}

fn extract_class(node: Node, source: &str, leaves: &mut Vec<ExtractedLeaf>) {
    let Some(name) = get_name(node, source) else {
        return;
    };

    let children = extract_class_methods(node, source, leaves, &name);
    push_leaf(node, source, leaves, &name, &name, "class", None, children);
}

fn extract_class_methods(
    class_node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: &str,
) -> Vec<String> {
    let mut children = Vec::new();
    let Some(body) = class_node.child_by_field_name("body") else {
        return children;
    };

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if !matches!(
            child.kind(),
            "method_definition" | "method_signature" | "abstract_method_signature"
        ) {
            continue;
        }
        if let Some(qualified_name) = extract_method(child, source, leaves, parent) {
            children.push(qualified_name);
        }
    }
    children
}

fn extract_method(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    parent: &str,
) -> Option<String> {
    let name = get_name(node, source)?;
    let arity = parameter_arity(node);
    let qualified_name = format!("{parent}::{name}#{arity}");
    push_leaf(
        node,
        source,
        leaves,
        &name,
        &qualified_name,
        "method",
        Some(parent.to_string()),
        Vec::new(),
    );
    Some(qualified_name)
}

fn parameter_arity(node: Node) -> usize {
    let Some(parameters) = node.child_by_field_name("parameters") else {
        return 0;
    };
    let mut cursor = parameters.walk();
    parameters
        .named_children(&mut cursor)
        .filter(|child| child.kind().contains("parameter"))
        .count()
}

fn extract_binding_declaration(node: Node, source: &str, leaves: &mut Vec<ExtractedLeaf>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "variable_declarator" {
            continue;
        }

        let Some(name) = get_simple_binding_name(child, source) else {
            continue;
        };
        let Some(value) = child.child_by_field_name("value") else {
            continue;
        };

        match value.kind() {
            "arrow_function" | "function_expression" | "generator_function" => {
                push_leaf(
                    child,
                    source,
                    leaves,
                    &name,
                    &name,
                    "function",
                    None,
                    Vec::new(),
                );
            }
            "class" => extract_class_binding(child, value, source, leaves, &name),
            _ => {}
        }
    }
}

fn extract_class_binding(
    declarator: Node,
    class_expr: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    binding_name: &str,
) {
    let children = extract_class_methods(class_expr, source, leaves, binding_name);
    push_leaf(
        declarator,
        source,
        leaves,
        binding_name,
        binding_name,
        "class",
        None,
        children,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_leaf(
    node: Node,
    source: &str,
    leaves: &mut Vec<ExtractedLeaf>,
    name: &str,
    qualified_name: &str,
    kind: &str,
    parent_qualified_name: Option<String>,
    children_qualified_names: Vec<String>,
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
        parent_qualified_name,
        children_qualified_names,
        depth: None,
    });
}
