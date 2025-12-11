# Regular expressions

Maybe the most important feature of **Logos** is its ability to accept
regex patterns in your tokens' definition.

[Regular expressions](https://en.wikipedia.org/wiki/Regular_expression),
or regexes for short, are sequences of characters (or bytes) that define a match
pattern. When constructing lexers, this is especially useful to define tokens
that should match a set of *similar* literals. E.g., a sequence of
3 ASCII uppercase letters and 3 digits could define a license plate,
and could be matched with the following regex: `"[A-Z]{3}[0-9]{3}"`.

For more details about regexes in Rust, refer to the
[regex](https://crates.io/crates/regex) crate.

## Regex Flags

Regular expression flags are a useful way to change the behavior of regular
expressions. For example, enabling the`m` flag (stands for "multiline") causes
the dot `.` to match newlines. The `i` flag causes the pattern to ignore case.
To enable a flag, either toggle it on with `(?<flag>)`, or enable it using a
flag group, like so `(?<flag>:<pattern>)`. For example

- `r"(?m).*"` will match the entire input. Without the `m` flag, it would only match the entire first line.
- `r"(?i:c)afe"` will match both `"cafe"` and `"Cafe"`

For more information about regex flags, see the `regex` crate
[documentation](https://docs.rs/regex/latest/regex/#grouping-and-flags).

## Common performance pitfalls

Because **Logos** aims at generating high-performance code, its matching engine
will never backtrack during a token. However, it is possible that it will have
to re-read bytes that it already read while lexing the previous token. As an
example, consider the regex `"a(.*b)?"`. When trying to parse tokens on a file
full of `a` characters, the engine must read the entire file to see if there is
a `b` character anywhere. Properly tokenizing this pattern creates a surprising
performance of `O(n^2)` where `n` is the size of the file. Indeed, any pattern
that contains an unbounded greedy dot repetition requires reading the entire
file before returning the next token. Since this is almost never the intended
behavior, logos returns a compile time error by default when encountering
patterns containing `.*` and `.+`. If this is truly your intention, you can add
the flag `allow_greedy = true` to your `#[regex]` attribute. But first
consider whether you can instead use a non-greedy repetition, which would also
resolve the performance concern.

For reference, **Logos** parses regexes using the `regex-syntax` and
`regex-automata` crates, and transforms the deterministic finite automata
created by the `regex-automata` crate into rust code that implements the
matching state machine. Every regex is compiled with an implicit `^` anchor at
its start, since that is how a tokenizer works.

Additionally, note that capture groups will be silently changed to *non
capturing*, because **Logos** does not support capturing groups, only the whole
match group returned by `lex.slice()`.

If any of these limitations are problematic, you can move more of your matching
logic into a callback, possibly using the
[`Lexer::bump`](https://docs.rs/logos/latest/logos/struct.Lexer.html#method.bump)
function.

## Error semantics

The matching semantics for returning an error are as follows. An error is
generated when the lexer encounters a byte that doesn't have an transition for
its current state. This means that this adding this byte to the currently read
byte string makes it impossible to match any defined `#[regex]` or
`#[token]`. An error token is then returned with a span up to but not
including the byte that caused the error, unless that would return an empty
span, in which case that byte is included.

This is usually a good heuristic for generating error spans because the first
token that cannot match anything is likely to be the start of another valid
token.

## Limitations

While Logos strives to have a feature complete regex implementation, there are
some limitations. Unicode word boundaries, some lookarounds, and other advanced
features not supported by the DFA matching engine in the `regex` crate are not
possible to match using Logos's generated state machine.

However, attempting to use a missing feature will result in a compile time
error. If your code compiles, the matcher behavior is exactly the same as the
`regex` crate.

## Other issues

**Logos**' support for regexes is feature complete, but errors can still exist.
Some are found at compile time, and others will create wrong matches or panic.

If you ever feel like your patterns do not match the expected source slices,
please check the
[GitHub issues](https://github.com/maciejhirsz/logos/issues?q=is%3Aissue).
If no issue covers your problem, we encourage
you to create a
[new issue](https://github.com/maciejhirsz/logos/issues/new),
and document it as best as you can so that the issue
can be reproduced locally.
