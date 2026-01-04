#[test]
fn test_derive() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/ok/*.rs");
    t.compile_fail("tests/ui/err/*.rs");
}
