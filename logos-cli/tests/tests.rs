use std::path::Path;

use assert_cmd::Command;
use assert_fs::{assert::PathAssert, fixture::FileWriteStr, NamedTempFile};
use predicates::prelude::*;

const INPUT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/input.rs");
const OUTPUT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/output.rs");
const FMT_OUTPUT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/fmt_output.rs");

#[test]
fn test_codegen() {
    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    let mut cmd = Command::cargo_bin("logos-cli").unwrap();
    cmd.arg(INPUT_FILE)
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .success();

    tempfile.assert(normalize_newlines(OUTPUT_FILE));
}

#[test]
fn test_codegen_check() {
    Command::cargo_bin("logos-cli")
        .unwrap()
        .arg(INPUT_FILE)
        .arg("--check")
        .arg("--output")
        .arg(OUTPUT_FILE)
        .assert()
        .success();
}

#[test]
fn test_codegen_check_format() {
    Command::cargo_bin("logos-cli")
        .unwrap()
        .arg(INPUT_FILE)
        .arg("--format")
        .arg("--check")
        .arg("--output")
        .arg(FMT_OUTPUT_FILE)
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
    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    let mut cmd = Command::cargo_bin("logos-cli").unwrap();
    cmd.arg(INPUT_FILE)
        .arg("--format")
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .success();

    tempfile.assert(normalize_newlines(FMT_OUTPUT_FILE));
}

fn normalize_newlines(s: impl AsRef<Path>) -> impl Predicate<str> {
    predicates::str::diff(fs_err::read_to_string(s).unwrap().replace("\r\n", "\n")).normalize()
}
