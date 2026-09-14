#[derive(Logos)]
#[logos(subpattern not_unicode_letter = r"\s\d")]
#[logos(subpattern invalid_symbols = r"#:=;\(\),\{\}\.\|")]
#[logos(subpattern start_of_symbol = r"[^(?&not_unicode_letter)(?&invalid_symbols)]")]
enum Token {
    #[regex("(?&start_of_symbol)")]
    Symbol,
}
