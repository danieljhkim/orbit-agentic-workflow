pub mod fs;
pub mod net;
pub mod proc;
pub mod time;

use crate::ToolRegistry;

pub fn register_builtins(registry: &mut ToolRegistry) {
    fs::register(registry);
    proc::register(registry);
    time::register(registry);
    net::register(registry);
}
