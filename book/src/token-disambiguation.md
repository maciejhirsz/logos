# Token disambiguation

When two or more tokens can match a given sequence, **Logos** compute the
priority of each pattern (`#[token]` or `#[regex]`), and use that priority
to decide which pattern should match.

The rule of thumb is:

+ Longer beats shorter.
+ Specific beats generic.

If any two definitions could match the same input, like `fast` and `[a-zA-Z]+`
in the example above, it's the longer and more specific definition of `Token::Fast`
that will be the result.

This is done by comparing numeric priority attached to each definition. Every
consecutive, non-repeating single byte adds 2 to the priority, while every range
or regex class adds 1.
Loops or optional blocks are ignored, while alternations count the shortest alternative:

+ `[a-zA-Z]+` has a priority of 1 (lowest possible), because at minimum it can
  match a single byte to a class.
+ `foobar` has a priority of 12.
+ `(foo|hello)(bar)?` has a priority of 6, `foo` being it's shortest possible match.

```admonish info
When two patterns have the same priority, **Logos** will issue an compilation
error.
To prevent this from happening, you can manually set the priority of a given
pattern with, e.g., `#token("foobar", priority = 20)`.
```
