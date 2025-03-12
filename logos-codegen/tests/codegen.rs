use std::{
    error::Error,
    io::{self, Read, Seek},
    path::PathBuf,
};

#[rstest::rstest]
#[case("simple")]
#[case("no_error_lut")]
#[case("skip_callback")]
#[case("skip_callback_failure")]
pub fn test_codegen(#[case] fixture: &str) -> Result<(), Box<dyn Error>> {
    let mut fixture_dir = PathBuf::new();
    fixture_dir.push(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.push("tests");
    fixture_dir.push("data");
    fixture_dir.push(fixture);

    let input = fixture_dir.join("input.rs");
    #[cfg(rust_1_82)]
    fixture_dir.push("output.rs");
    #[cfg(not(rust_1_82))]
    fixture_dir.push("output-pre_1_82.rs");
    let output_file_path = fixture_dir;

    let input = std::fs::read_to_string(input)?;

    let mut output = String::new();

    // we want to create the output file if it doesn't exist
    let mut output_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&output_file_path)?;

    output_file.read_to_string(&mut output)?;

    let generated = logos_codegen::generate(input.parse()?);
    let generated = generated.to_string();

    if std::env::var("BLESS_CODEGEN").is_ok_and(|value| value == "1") {
        use std::io::{Seek as _, Write as _};
        output_file.set_len(0)?;
        output_file.seek(io::SeekFrom::Start(0))?;
        output_file.write_all(generated.as_bytes())?;
        return Ok(());
    }

    assert_eq!(generated, output, "Codegen test failed: `{fixture}`, run tests again with env var `BLESS_CODEGEN=1` to bless these changes");

    Ok(())
}
