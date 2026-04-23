# Prefix lexing

It is sometimes useful to give to the lexer not the full input buffer, but only a prefix of it.
For example, when lexing files that do not fit into memory.

For that Logos exposes the `Lexer::new_prefix` function that is a variant of `Lexer::new` but return `None` if more data should be given to unambgiously recognize the next token.

Usage example
```rust,no_run,no_playground
while !is_eof { // We run while we still have unread data
    let mut lexer = Lexer::new_prefix(buffer);
    while let Some(token) = lexer.next() {
        // Do something with the token
    }
    // We add more data to the buffer
    let extend_buffer_with_more_data(&mut buffer);
}
// We lex the last tokens with the usual lexer because the buffer is now filled to the end
let mut lexer = Lexer::new(buffer);
while let Some(token) = lexer.next() {
    // Do something with the token
}
```

This can be leverage to lex data from a [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) or an [`AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html) instance without buffering everything into memory.
See the repo `json_reader` example for that.
