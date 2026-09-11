#[derive(Logos)]
enum Token {
    #[regex("a*")]
    A,
}
