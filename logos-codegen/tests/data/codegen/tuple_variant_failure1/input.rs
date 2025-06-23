#[derive(Logos)]
enum Token<'src> {
    #[regex(r"\d+.\d+")]
    Decimal(&'src str, &'src str),
}
