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
    let input: proc_macro2::TokenStream = input.parse().expect("Unable to parse input.rs");

    // Keep generated files outside the source tree and clean up even if an
    // assertion panics. Each test invocation gets its own directory.
    let export_dir = tempfile::tempdir().expect("Unable to create export directory");
    let export_path = export_dir.path().to_str().expect("Non-UTF-8 export path");
    let _ = logos_codegen::generate(quote::quote! {
        #[logos(export_dir = #export_path)]
        #input
    });

    let generated_dot = std::fs::read_to_string(export_dir.path().join(format!("{case}.dot")))
        .expect("Unable to read dot file");
    let generated_mermaid = std::fs::read_to_string(export_dir.path().join(format!("{case}.mmd")))
        .expect("Unable to read mermaid file");

    assert_snapshot!(format!("{case}-dot"), generated_dot);
    assert_snapshot!(format!("{case}-mmd"), generated_mermaid);
}
