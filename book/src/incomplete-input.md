# Handling incomplete input

Logos lexers operate on the source slice they receive. If that slice ends while
the next token may still grow, the lexer cannot tell whether the input is truly
finished or whether more bytes will arrive later.

For interactive editors, language servers, and sans-I/O parsers, a practical
pattern is to model known incomplete forms as explicit tokens. The parser can
then decide whether to report an unfinished construct, keep buffering from the
token span, or request more input from the caller.

The example below accepts complete tags like `<ready>`, but emits
`IncompleteTag` for a tag-like prefix that reaches the end of the provided
source:

```rust
{{#include ../../examples/partial_input.rs:all}}
```

In an editor, `IncompleteTag` can become a recoverable diagnostic. In a
streaming parser, the span from `spanned()` identifies the bytes that should be
kept in the buffer before reading more input.

This does not replace a dedicated partial-lexing API. It is a useful approach
when the incomplete shapes are known from the grammar and can be represented as
ordinary Logos token definitions.
