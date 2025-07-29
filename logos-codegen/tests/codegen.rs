use insta::assert_snapshot;
use std::{error::Error, path::PathBuf};
use syn::parse_file;

#[rstest::rstest]
#[case("simple")]
#[case("no_error_lut")]
#[case("skip_callback")]
#[case("skip_callback_failure")]
#[case("error_callback0")]
#[case("error_callback1")]
#[case("error_callback_failure")]
pub fn test_codegen(#[case] fixture: &str) -> Result<(), Box<dyn Error>> {
    let rust_ver = if cfg!(rust_1_82) { "1_82" } else { "pre_1_82" };

    let codegen_alg = if cfg!(feature = "state_machine_codegen") {
        "state_machine"
    } else {
        "tailcall"
    };

    let mut fixture_dir = PathBuf::new();
    fixture_dir.push(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.push("tests");
    fixture_dir.push("data");
    fixture_dir.push("codegen");
    fixture_dir.push(fixture);

    let input = fixture_dir.join("input.rs");
    let input = std::fs::read_to_string(input)?;

    let generated = logos_codegen::generate(input.parse()?).to_string();

    let formatted =
        prettyplease::unparse(&parse_file(&generated).expect("Logos output is unparseable"));

    assert_snapshot!(format!("{fixture}-{codegen_alg}-{rust_ver}"), formatted);

    Ok(())
}
