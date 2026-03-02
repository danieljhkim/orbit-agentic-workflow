use std::path::Path;

use orbit_types::OrbitError;

use crate::fs_utils::write_text_with_parent;

const DEFAULT_IDENTITY_FILES: [(&str, &str); 6] = [
    ("linus", include_str!("../../assets/identities/linus.yaml")),
    ("john", include_str!("../../assets/identities/john.yaml")),
    ("kent", include_str!("../../assets/identities/kent.yaml")),
    ("rob", include_str!("../../assets/identities/rob.yaml")),
    ("grace", include_str!("../../assets/identities/grace.yaml")),
    ("steve", include_str!("../../assets/identities/steve.yaml")),
];

pub(crate) fn seed_default_identities(identity_root: &Path) -> Result<usize, OrbitError> {
    let mut created = 0usize;
    for (name, content) in DEFAULT_IDENTITY_FILES {
        let path = identity_root.join(format!("{name}.yaml"));
        if path.exists() {
            continue;
        }
        write_text_with_parent(&path, content)?;
        created += 1;
    }
    Ok(created)
}
