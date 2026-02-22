pub mod delete;
pub mod list;
pub mod read;
pub mod write;

use crate::ToolRegistry;

pub fn register(registry: &mut ToolRegistry) {
    registry.register(read::FsReadTool);
    registry.register(write::FsWriteTool);
    registry.register(delete::FsDeleteTool);
    registry.register(list::FsListTool);
}
