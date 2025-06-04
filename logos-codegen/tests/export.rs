use std::error::Error;
use std::path::PathBuf;

#[cfg(feature = "debug")]
#[rstest::rstest]
#[case("complex")]
pub fn test_export(#[case] fixture: &str) -> Result<(), Box<dyn Error>> {
    let mut fixture_dir = PathBuf::new();
    fixture_dir.push(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.push("tests");
    fixture_dir.push("data");
    fixture_dir.push("export");
    fixture_dir.push(fixture);

    let input = fixture_dir.join("input.rs");
    let output_file_dot = fixture_dir.join("output.dot");
    let output_file_mermaid = fixture_dir.join("output.mmd");

    let input = std::fs::read_to_string(input)?;
    let output_dot = std::fs::read_to_string(&output_file_dot)?;
    let output_mermaid = std::fs::read_to_string(&output_file_mermaid)?;

    let _ = logos_codegen::generate(input.parse()?);

    let generated_dot = std::fs::read_to_string(format!("export_tmp/{}.dot", fixture))?;
    let generated_mermaid = std::fs::read_to_string(format!("export_tmp/{}.mmd", fixture))?;

    if std::env::var("BLESS_EXPORT").is_ok_and(|value| value == "1") {
        std::fs::write(&output_file_dot, &generated_dot)?;
        std::fs::write(&output_file_mermaid, &generated_mermaid)?;

        // cleanup
        let _ = std::fs::remove_dir_all("export_tmp");

        return Ok(());
    }

    assert_eq!(generated_dot, output_dot, "Export test failed: `{fixture}`, run tests again with env var `BLESS_EXPORT=1` to bless these changes");
    assert_eq!(generated_mermaid, output_mermaid, "Export test failed: `{fixture}`, run tests again with env var `BLESS_EXPORT=1` to bless these changes");

    // cleanup
    let _ = std::fs::remove_dir_all("export_tmp");

    Ok(())
}
