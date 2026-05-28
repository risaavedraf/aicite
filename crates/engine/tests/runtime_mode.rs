//! Runtime-mode enforcement integration tests.
//!
//! Verifies that `check_ingest_allowed` correctly gates ingest
//! by runtime mode.

use config::RuntimeMode;
use engine::runtime_guard::check_ingest_allowed;

#[test]
fn local_private_demo_allows_ingest() {
    let result = check_ingest_allowed(&RuntimeMode::LocalPrivateDemo);
    assert!(result.is_ok(), "LocalPrivateDemo should allow ingest");
}

#[test]
fn public_packaged_demo_forbids_ingest() {
    let result = check_ingest_allowed(&RuntimeMode::PublicPackagedDemo);
    assert!(result.is_err(), "PublicPackagedDemo should forbid ingest");

    let err = result.unwrap_err();
    assert_eq!(err.code(), "runtime_mode_forbidden");
}

#[test]
fn production_forbids_ingest() {
    let result = check_ingest_allowed(&RuntimeMode::Production);
    assert!(result.is_err(), "Production should forbid ingest");

    let err = result.unwrap_err();
    assert_eq!(err.code(), "runtime_mode_forbidden");
}
