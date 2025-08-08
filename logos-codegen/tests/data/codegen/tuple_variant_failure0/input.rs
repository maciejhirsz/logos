#[derive(Logos)]
enum Token {
    #[regex(r"\d+")]
    Integer(),
}
