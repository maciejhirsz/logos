# Unicode support

By default, logos is unicode aware. It accepts input in the form of a rust
`&str` that is valid UTF-8 and it compiles its regular expressions to match
unicode codepoints. When it returns spans for tokens, these spans are
guaranteed to not split utf-8 codepoints. These behaviors can all be changed,
however.

## Using `&[u8]` input
The easiest thing to change is how logos accepts an input. By adding the
`#[logos(utf8 = false)]` attribute to your token enum, you instruct logos to
accept a byte slice for input instead. This, by itself, doesn't change matching
behavior at all. The regular expressions are all still compiled with unicode
support, `.` matching a single character rather than a byte, etc. If all you
did was add that attribute and you called the lexer with
`Token::lexer(input.as_bytes())`, then you would get the exact same output as
before.

## Matching bytes rather than Unicode codepoints
If you want to ignore unicode altogether and match ascii, raw bytes, or
whatever esoteric character encoding you want, you can compile your regular
expressions with unicode mode off. This can be done by either removing the
unicode flag manually with `(?-u)` in your regular expression, or if you supply
the pattern as a byte string, like `#[regex(b"my.*pattern")]` then logos will
turn off the flag for you. See the [`regex`
docs](https://docs.rs/regex/latest/regex/#grouping-and-flags) for more
information.

Logos will automatically detect if any of your patterns can match a byte
sequence that is invalid utf8. If one exists and you haven't set the lexer to
use `&[u8]` input, it will issue a compile error.
