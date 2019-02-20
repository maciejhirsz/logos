use logos_derive::Logos;

// Nothing is really being tested here, it just has to compile!
#[derive(Logos)]
enum Token {
    #[end]
    End,

    #[error]
    Error,

    #[regex = r"\w"] // TEST ME WITH r"\w\w"!!!
    Label,
}
