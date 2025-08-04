#[derive(Logos)]
#[logos(utf8 = false)]
enum Token {
    #[token("\n")]
    Newline,
    #[regex(".")]
    AnyUnicode,
    #[regex(b".", priority = 0)]
    Any,
}
