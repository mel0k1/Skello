# Contributing

## Build and checks

You need Rust stable and Flutter stable. Run before a commit:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd app
flutter analyze
flutter test
```

CI runs the same on Linux and Windows; a red PR does not merge.

## Code rules

- No comments in code: neither line nor doc comments. If a piece of code needs a paragraph to make sense, the code gets rewritten, not annotated
- Constant, command and variable names come from AOSP and reference-project sources as they are
- Interface strings live in app/lib/l10n/*.arb, in Russian and English at the same time; no user-facing strings in code
- Code ported from other projects lands in NOTICE with the source and license

## Commits

- Commit author is mel0k1
- Titles are short and on point: "app i18n", "chipset notes", no filler
- Before a commit, texts (README, docs, ARB) go through an antislop pass: no officialese, no template phrasing, no unearned promises
