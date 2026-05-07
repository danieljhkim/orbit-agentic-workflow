#[test]
fn intentional_ci_failure_for_permissions_validation() {
    assert_eq!(
        std::env::var("ORBIT_CSO_PERMISSION_VALIDATION")
            .ok()
            .as_deref(),
        Some("pass")
    );
}
