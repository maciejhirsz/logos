#![cfg(feature = "debug")]

use std::path::Path;

use insta::assert_snapshot;

#[rstest::rstest]
#[case("complex")]
fn test_export(#[case] case: &str) {
    let mut input_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/export");
    input_path.push(case);
    input_path.push("input.rs");

    let input = std::fs::read_to_string(input_path).expect("Unable to read input.rs");

    let _ = logos_codegen::generate(input.parse().expect("Unable to parse input.rs"));

    let generated_dot = std::fs::read_to_string(format!("{case}_export_tmp/{case}.dot"))
        .expect("Unable to read dot file");
    let generated_mermaid = std::fs::read_to_string(format!("{case}_export_tmp/{case}.mmd"))
        .expect("Unable to read mermaid file");

    assert_snapshot!(format!("{case}-dot"), generated_dot);
    assert_snapshot!(format!("{case}-mmd"), generated_mermaid);

    // cleanup
    let _ = std::fs::remove_dir_all(format!("{case}_export_tmp"));
}
