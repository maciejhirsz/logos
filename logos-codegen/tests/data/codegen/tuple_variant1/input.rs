#[derive(Logos)]
enum Token<'src> {
    #[regex(r"\d+")]
    Integer(&'src str),
}
