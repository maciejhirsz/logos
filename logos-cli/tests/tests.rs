use insta::assert_snapshot;
use std::fs::read_to_string;

use assert_cmd::Command;
use assert_fs::{fixture::FileWriteStr, NamedTempFile};

const INPUT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/input.rs");

fn gen_to_file(format: bool) -> NamedTempFile {
    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    let mut cmd = Command::cargo_bin("logos-cli").unwrap();
    cmd.arg(INPUT_FILE).arg("--output").arg(tempfile.path());
    if format {
        cmd.arg("--format");
    }

    cmd.assert().success();

    tempfile
}

fn get_features_label() -> &'static str {
    if cfg!(feature = "state_machine_codegen") {
        "state_machine"
    } else {
        "tailcall"
    }
}

#[test]
fn test_codegen() {
    let tempfile = gen_to_file(false);

    let output = read_to_string(tempfile).expect("Unable to read output file");

    assert_snapshot!(format!("{}-nofmt", get_features_label()), output);
}

#[test]
fn test_codegen_check() {
    let tempfile = gen_to_file(false);

    Command::cargo_bin("logos-cli")
        .unwrap()
        .arg(INPUT_FILE)
        .arg("--check")
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .success();
}

#[test]
fn test_codegen_check_format() {
    let tempfile = gen_to_file(true);

    Command::cargo_bin("logos-cli")
        .unwrap()
        .arg(INPUT_FILE)
        .arg("--format")
        .arg("--check")
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .success();
}

#[test]
fn test_codegen_fail_check() {
    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    tempfile.write_str("some random data").unwrap();

    Command::cargo_bin("logos-cli")
        .unwrap()
        .arg(INPUT_FILE)
        .arg("--check")
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .failure();
}

#[test]
fn test_codegen_format() {
    let tempfile = gen_to_file(true);

    let output = read_to_string(tempfile).expect("Unable to read output file");

    assert_snapshot!(format!("{}-fmt", get_features_label()), output);
}
