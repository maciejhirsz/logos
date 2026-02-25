#[derive(Logos)]
#[logos(lifetime = 'b, extras = (&'a [String], &'c [bool]))]
enum Token<'a, 'b, 'c, 'd> {
    #[token("a", |lex| lex.extras.0[2].trim())]
    A(&'a str),
    #[token("b")]
    B(&'b str),
    #[token("c", |lex| &lex.extras.1[9])]
    C(&'c bool),
    #[token("d", |_| &0.5)]
    D(&'d f32),
}
