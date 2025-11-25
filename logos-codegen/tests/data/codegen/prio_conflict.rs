#[derive(Logos)]
enum Token {
    #[token("a")]
    A,
    #[token("a")]
    B,
    #[token("1")]
    One,
    #[regex("1")]
    Two,
    #[regex("1", ignore(case))]
    Three,
}
