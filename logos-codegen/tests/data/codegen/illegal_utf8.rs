#[derive(Logos)]
#[logos(subpattern stuff = b"\\xFF")]
enum Token {
    #[token("a")]
    A,
    #[token(b"b\xFF")]
    B,
    #[token("1")]
    One,
    #[regex("(?-u)2\\xFF")]
    Two,
    #[regex(b"3\\xFF")]
    Three,
}
