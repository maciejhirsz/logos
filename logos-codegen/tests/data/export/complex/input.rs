#[derive(Logos, Debug)]
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
