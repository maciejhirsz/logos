use std::path::Path;

use assert_cmd::Command;
use assert_fs::{assert::PathAssert, fixture::FileWriteStr, NamedTempFile};
use predicates::prelude::*;

struct Fixture {
    input: &'static str,
    output: &'static str,
    fmt_output: &'static str,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        input: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/simple/input.rs"),
        output: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/simple/output.rs"),
        fmt_output: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/simple/fmt_output.rs"
        ),
    },
    Fixture {
        input: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/no_error_lut/input.rs"
        ),
        output: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/no_error_lut/output.rs"
        ),
        fmt_output: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/no_error_lut/fmt_output.rs"
        ),
    },
];

#[test]
fn test_codegen() {
    for fixture in FIXTURES.iter() {
        let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

        let mut cmd = Command::cargo_bin("logos-cli").unwrap();
        cmd.arg(fixture.input)
            .arg("--output")
            .arg(tempfile.path())
            .assert()
            .success();

        tempfile.assert(normalize_newlines(fixture.output));
    }
}

#[test]
fn test_codegen_check() {
    for fixture in FIXTURES.iter() {
        Command::cargo_bin("logos-cli")
            .unwrap()
            .arg(fixture.input)
            .arg("--check")
            .arg("--output")
            .arg(fixture.output)
            .assert()
            .success();
    }
}

#[test]
fn test_codegen_check_format() {
    for fixture in FIXTURES.iter() {
        Command::cargo_bin("logos-cli")
            .unwrap()
            .arg(fixture.input)
            .arg("--format")
            .arg("--check")
            .arg("--output")
            .arg(fixture.fmt_output)
            .assert()
            .success();
    }
}

#[test]
fn test_codegen_fail_check() {
    let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

    tempfile.write_str("some random data").unwrap();

    Command::cargo_bin("logos-cli")
        .unwrap()
        .arg(FIXTURES[0].input)
        .arg("--check")
        .arg("--output")
        .arg(tempfile.path())
        .assert()
        .failure();
}

#[test]
fn test_codegen_format() {
    for fixture in FIXTURES {
        let tempfile = NamedTempFile::new("output.gen.rs").unwrap();

        let mut cmd = Command::cargo_bin("logos-cli").unwrap();
        cmd.arg(fixture.input)
            .arg("--format")
            .arg("--output")
            .arg(tempfile.path())
            .assert()
            .success();

        tempfile.assert(normalize_newlines(fixture.fmt_output));
    }
}

fn normalize_newlines(s: impl AsRef<Path>) -> impl Predicate<str> {
    predicates::str::diff(fs_err::read_to_string(s).unwrap().replace("\r\n", "\n")).normalize()
}
