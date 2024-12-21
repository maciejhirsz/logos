# Context-dependent lexing

Sometimes, a single lexer is insufficient to properly handle complex grammars. To address this, many lexer generators offer the ability to have separate lexers with their own set of patterns and tokens, allowing you to dynamically switch between them based on the context.

In Logos, context switching is handled using the [`morph`](https://docs.rs/logos/0.11.0-rc2/logos/struct.Lexer.html#method.morph) method of the `logos::Lexer` struct.
This method takes ownership of the current lexer and transforms it into a lexer for a new token type.

It is important to note that:

- Both the original lexer and the new lexer must share the same [`Source`](./attributes/logos.md#custom-source-type) type.
- The [`Extras`](./extras.md) type from the original lexer must be convertible into the `Extras` type of the new lexer.

## Example

The following example demonstrates how to use `morph` to handle a C-style language that also supports python blocks:

```rust
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"\s+")]
enum CToken {
    /* Tokens supporting C syntax */
    // ...
    #[regex(r#"extern\s+"python"\s*\{"#, python_block_callback)]
    PythonBlock(Vec<PythonToken>),
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"\s+")]
enum PythonToken {
    #[token("}")]
    ExitPythonBlock,
    /* Tokens supporting Python syntax */
    // ...
}

fn python_block_callback(lex: &mut Lexer<CToken>) -> Option<Vec<PythonToken>> {
    let mut python_lexer = lex.clone().morph::<PythonToken>();
    let mut tokens = Vec::new();
    while let Some(token) = python_lexer.next() {
        match token {
            Ok(PythonToken::ExitPythonBlock) => break,
            Err(_) => return None,
            Ok(tok) => tokens.push(tok),
        }
    }
    *lex = python_lexer.morph();
    Some(tokens)
}
```

Note that if we want to use `morph` inside a callback we need to be able to clone the original lexer, as `morph` needs to take ownership but the callback receives only a reference to the lexer.

For a more in depth example check out [String interpolation](./examples/string-interpolation.md).
