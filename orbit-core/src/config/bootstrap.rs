use std::path::Path;

use orbit_types::OrbitError;

use crate::fs_utils::write_text_with_parent;

const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../../assets/config/default-config.toml");
const DEFAULT_CONFIG_TEMPLATE_REPO: &str =
    include_str!("../../assets/config/default-config-repo.toml");

pub(crate) fn default_config_template_for_root(
    orbit_root: &Path,
    orbit_home: &Path,
) -> &'static str {
    if orbit_root == orbit_home {
        DEFAULT_CONFIG_TEMPLATE
    } else {
        DEFAULT_CONFIG_TEMPLATE_REPO
    }
}

pub(crate) fn seed_default_config(
    config_path: &Path,
    config_template: &str,
) -> Result<bool, OrbitError> {
    if config_path.exists() {
        return Ok(false);
    }
    write_text_with_parent(config_path, config_template)?;
    Ok(true)
}
