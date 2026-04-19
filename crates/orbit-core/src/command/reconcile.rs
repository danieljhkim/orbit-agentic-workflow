use orbit_common::types::OrbitError;
use orbit_engine::{ReconcileOutcome, reconcile_once};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn reconcile_once(&self, dry_run: bool) -> Result<ReconcileOutcome, OrbitError> {
        reconcile_once(self, dry_run)
    }
}
