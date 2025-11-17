# Logos Version Migration Guide

This page contains guidance for migrating between versions of logos that have
major breaking changes.

## Changes in 0.16.0

Logos 0.16.0 was a very large update. As of this writing, the PR changed over
100 files and touches over 1000 lines of code. It fixed a number of long
standing issues related to backtracking and matching state machine soundness.

The update also added some major new features and a handful of breaking changes.

### New Features

- Dot repetitions such as `.*` and `.+` are now supported. Due to the related
  supported pitfalls, they are disallowed by default, but can be used if you pass
  the attribute argument `allow_greedy = true` or if you make them non-greedy.
  For more information, see [Common performance
  pitfalls](./common-regex#common-performance-pitfalls).
- The error token semantics are now precisely defined. See [Error
  semantics](./common-regex#error-semantics).
- The new `state_machine_codegen` feature. If you are experiencing issues with
  stack overflows, enabling this feature will solve them. It is slower than the
  default tailcall codegen, but it will never overflow the stack. See [State
  machine codegen](./state-machine-codegen).

### Breaking Changes

- The `ignore_ascii_case` attribute was removed. You can switch to using the
  `ignore_case` attribute, which also works on non-unicode patterns. If you
  explicitly want to ignore case for ascii characters but not others, you will
  have to do it manually using character classes. See [`#[token]` and
  `#[regex]`](.attributes/token_and_regex).
- The `source` attribute has been removed. You can now use the `utf8` attribute
  to select either `&str` or `&[u8]` as the source type. Custom source types
  are no longer supported. If you need this feature, you can either stay on
  `0.15.1` or contribute an implementation to Logos! For more information on
  `utf8`, see its [`#[logos]`](./attributes/logos.md#custom-source-type).

