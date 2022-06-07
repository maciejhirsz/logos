use assert_cmd::Command;
use assert_fs::{NamedTempFile, assert::PathAssert, fixture::FileWriteStr};

const INPUT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/input.rs");
const OUTPUT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/output.rs");
const FMT_OUTPUT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/fmt_output.rs");

#[test]
fn test_codegen() {
    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    let mut cmd = Command::cargo_bin("logos-cli").unwrap();
    cmd
        .arg(INPUT_FILE)
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .success();

    tempfile.assert(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/output.rs")));
}

#[test]
fn test_codegen_check() {

    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    Command::cargo_bin("logos-cli").unwrap()
        .arg(INPUT_FILE)
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .success();

    tempfile.assert(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/output.rs")));
}

#[test]
fn test_codegen_fail_check() {
    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    tempfile.write_str("some random data");

    Command::cargo_bin("logos-cli").unwrap()
        .arg(INPUT_FILE)
        .arg("--check")
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .failure();
}

#[test]
fn test_codegen_rustfmt() {
    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    let mut cmd = Command::cargo_bin("logos-cli").unwrap();
    cmd
        .arg(INPUT_FILE)
        .arg("--format")
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .success();

    tempfile.assert(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/fmt_output.rs")));
}