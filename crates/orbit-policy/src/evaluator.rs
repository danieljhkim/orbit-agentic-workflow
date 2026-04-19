use orbit_common::types::{FsCheckResult, FsOperation, OrbitError, PolicyDef};

pub(crate) fn evaluate(
    def: &PolicyDef,
    profile: &str,
    operation: FsOperation,
    path: &str,
) -> Result<FsCheckResult, OrbitError> {
    def.check_path(profile, operation, path)
}
