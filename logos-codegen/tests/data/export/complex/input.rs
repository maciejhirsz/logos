#[derive(Logos, Debug)]
#[logos(export_dir = "complex_export_tmp")]
enum Complex {
    #[regex("[a-z]")]
    Letter,

    #[token("struct")]
    Struct,

    #[token("str")]
    Str,

    #[regex("str[a-z]+")]
    StrPrefixed,
}
