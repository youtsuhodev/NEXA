use std::path::Path;
use std::process::Command;

#[test]
fn run_prints_hello() {
    let exe = env!("CARGO_BIN_EXE_nexa");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hello.nxa");
    let out = Command::new(exe)
        .args(["run", fixture.to_str().unwrap()])
        .output()
        .expect("run nexa");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello NEXA\n");
}

#[test]
fn check_succeeds_on_add_fixture() {
    let exe = env!("CARGO_BIN_EXE_nexa");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/add.nxa");
    let out = Command::new(exe)
        .args(["check", fixture.to_str().unwrap()])
        .output()
        .expect("run nexa check");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
