#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constraint {
    DenyTool(String),
}
