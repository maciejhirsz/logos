use insta::assert_snapshot;
use std::{error::Error, path::PathBuf};

#[rstest::rstest]
#[case("simple")]
#[case("no_error_lut")]
#[case("skip_callback")]
#[case("skip_callback_failure")]
#[case("error_callback0")]
#[case("error_callback1")]
#[case("error_callback_failure")]
pub fn test_codegen(#[case] fixture: &str) -> Result<(), Box<dyn Error>> {
    let mut fixture_dir = PathBuf::new();
    fixture_dir.push(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.push("tests");
    fixture_dir.push("data");
    fixture_dir.push("codegen");
    fixture_dir.push(fixture);

    let input = fixture_dir.join("input.rs");
    let input = std::fs::read_to_string(input)?;

    let generated = logos_codegen::generate(input.parse()?).to_string();

    if cfg!(rust_1_82) {
        assert_snapshot!(format!("{fixture}-1_82"), generated);
    } else {
        assert_snapshot!(format!("{fixture}-pre_1_82"), generated);
    }

    Ok(())
}
