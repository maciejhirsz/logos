#[derive(Logos)]
#[logos(extras = Vec<&'static str>)]
#[logos(error(&'static str, callback = |lex| { lex.extras.push("a"); "a" }))]
enum TokenA {}
