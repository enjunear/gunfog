# gunfog specification

`gunfog` is a CLI that scores prose with the Gunning fog index and reports hotspots so a coding agent can revise its own writing. Token-efficient output is the prime directive: every printed character must earn its place in an agent's context window.

Terms (prose, sentence, word, complex word, placeholder word, contribution, hotspot, target, floor) are defined in [CONTEXT.md](CONTEXT.md). Rationale and measurements live in `docs/research/`; this file states only what to build.

## CLI contract

Exactly one input per call:

- Inline argument: `gunfog "some text"`
- File: `gunfog --file path.md`
- Stdin, when neither is given: `cat doc.md | gunfog`

Flags:

- `--target N` (default 10): the fog score threshold that sets hotspot flagging and the exit code.
- `--limit N` (default 10): maximum hotspot lines. `--limit 0` means score only: the first output line and nothing else.

Exit codes: 0 when the document scores at or under target; 1 when it scores over target **or** is too short to score (the first output line disambiguates).

Output is compact plain text. No JSON, no colour, no decoration.

Input is treated as Markdown always. There is no plain-text mode; plain text is Markdown with no constructs and passes through unchanged.

## Prose extraction

The fog formula only ever sees prose: the text that remains after the following are removed.

Never scored:

- Headings
- Fenced code blocks
- Tables
- Image alt text
- Raw HTML, block and inline

Scored as prose:

- Paragraphs, blockquote content, footnote definitions
- Link text (the URL contributes nothing)
- Emphasis and strong are transparent: just their text

Placeholders: each inline code span and each bare URL/autolink is replaced by one placeholder word. It counts as exactly one word of one syllable and can never be complex. (Deleting these tokens instead would make sentences look shorter than they read.)

## Sentence segmentation

- Base segmentation is UAX #29 (the `unicode-segmentation` crate).
- A list item's start and end are always sentence boundaries; normal segmentation applies inside the item, so one bullet can hold several sentences. Nested items follow the same rule.
- A soft break becomes a space before segmentation. A hard break and a paragraph end are sentence boundaries, with or without terminal punctuation.
- An abbreviation merge pass joins the one UAX #29 failure class (abbreviation followed by a capitalised word, e.g. `Dr. Smith`). The list is fixed and test-covered: **Mr, Mrs, Ms, Dr, Prof, St, e.g., i.e., cf., vs.** It deliberately excludes `etc.`, which legitimately ends sentences.

## Word tokenisation

- A hyphenated compound is one word, hyphen kept (Kincaid et al. 1975, Appendix B; Gunning is silent — see [docs/research/hyphenated-words.md](docs/research/hyphenated-words.md)).
- Contractions count as one word.
- Numbers and version strings count as one word and are never complex.

## Syllable counting

Rules only, no word list, no regex engine. The counter is the 29-rule byte-comparison set from [docs/research/syllable-counting.md](docs/research/syllable-counting.md) §4: a vowel-letter-group base estimate with silent-`e` and consonant+`le` corrections, adjusted by the 29 tuned rules listed there (22 are plain `contains`/`starts_with`/`ends_with` literals; 7 are short windows with a character-class test). Measured: 96.1% held-out accuracy on the 3+ boundary, 98.2% token-weighted on real agent prose, +38 KB, 0.38 µs/word.

The +524 KB CMU-derived word list is deliberately not shipped. If golden tests later show hotspots misfiring on real text, adding it as a first-check layer is purely additive.

## Complex words

A complex word has 3+ syllables, after Gunning's own exclusions (The Technique of Clear Writing, 1968, pp. 38–39):

- **Proper names**: excused, detected as capitalised words not at sentence start. A name opening a sentence slips through and gets counted; accepted cost.
- **Words made 3-syllable only by `-ed`/`-es`**: excused. `-ing` is deliberately not excused; sources dispute it and Gunning's text says `-ed`/`-es`.
- **Hyphenated words**: complex only if at least one hyphen-separated part is 3+ syllables on its own. No compound dictionary; closed compounds (`bookkeeper`) stay undetected.
- There is no "familiar jargon" rule; Gunning's book has none.

A word excused by an exclusion never appears in a `complex:` list.

## Scoring

Fog = `0.4 × (words / sentences + 100 × complex words / words)`, over the whole document's prose. The headline score prints at one decimal.

Calibration anchor: Gunning's own worked example (this implementation scores it 11.0 against his published 10.9) plus a hand-counted golden corpus, kept as permanent tests. gunfog explicitly does not chase agreement with textstat or the npm packages, which mutually diverge by up to 4.9 grades.

## Hotspots

A sentence's **contribution** is `Fog(doc) − Fog(doc minus this sentence)`: the leave-one-out removal counterfactual, computed O(1) per sentence from document running totals (words, sentences, complex words). Every sentence has a signed contribution. There is no sentence-length floor.

- Hotspots print only when the document scores over target. A passing document gets the score line and nothing else.
- Selection: sentences with positive contribution, in descending contribution, until their cumulative contribution covers the gap (score − target), capped by `--limit`.
- Display: document order.
- Tail, only when `--limit` truncates the selection: `+N more, X.X grades remaining` (the uncovered remainder of the gap, one decimal).
- Known approximation: removal contributions do not sum exactly (each removal shifts the base for the others), so gap coverage is approximate. The limit bounds the diffuse-fog case where every contribution is tiny.

## Short text

The floor is 100 prose words, exact and fixed (no flag): exactly 100 scores, 99 refuses. Under the floor no fog score is printed:

```
no score: 34 words (min 100)
complex: paradigm, initialisation, functionality
```

Line 1 always; line 2 only when complex words exist, deduped, in document order. No hotspot lines. Exit 1. `--limit 0` prints line 1 alone. Zero-prose input takes the same path: `no score: 0 words (min 100)`.

Scores at 100+ words print clean, but note: under ~400 words the fog score is coarse (sampling noise of roughly 1–2 grades).

## Output rendering

Over target, file input:

```
fog: 12.4 (target 10)
+0.82 26w L12 "The leave-one-out contribution of each sentence" complex: contribution, counterfactual
+0.41 19w L30 "Selection proceeds in descending contribution until the" complex: descending, contribution
+3 more, 1.2 grades remaining
```

- Score line: `fog: <score> (target <target>)`, score at one decimal.
- Hotspot line, in document order: contribution at two decimals with sign (it is a delta, not a grade claim), word count as `<N>w`, source line number as `L<N>` for `--file` input only, an excerpt of the sentence's first ~8 words as written in the source (quoted, `…` when truncated), then `complex:` and the sentence's complex words in document order — omitted when the sentence has none. Local per-sentence fog is never printed.
- At or under target: the score line alone.

## Implementation

Rust. Dependencies: `pulldown-cmark`, `unicode-segmentation`, `clap`. (`regex-lite` was in the baseline plan but the syllable rules are byte comparisons; it enters only if a genuine need appears.)

### Crate layout and toolchain

- Single package, lib + thin bin: `src/lib.rs` exposes the segmentation, syllable, scoring, and report modules; `src/main.rs` is CLI wiring only. Integration and golden-corpus tests call the library.
- `#![forbid(unsafe_code)]`. `Cargo.lock` committed. `/target` ignored.
- Edition 2024. Latest-stable policy, no MSRV commitment: `rust-toolchain.toml` pins the exact stable version with `components = ["clippy", "rustfmt"]` (the official Rust docker images ship neither). `rust-version` in Cargo.toml mirrors the pin. Cost: `cargo install gunfog` refuses on anything older; bumping the pin is routine maintenance. Keep the CI image tag and the pin in lockstep (a mismatch costs ~31.5 s per job re-downloading a toolchain).
- Stock rustfmt, no `rustfmt.toml`. Default clippy groups, no `[lints]` table.
- `Cargo.toml` metadata: `repository` points at the GitHub mirror (the URL crates.io, Homebrew, and npm advertise); `description` and `license` are required.

### Profiles

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true

[profile.dist]
inherits = "release"
```

`[profile.dist]` contains `inherits = "release"` and nothing else: cargo-dist builds `--profile dist`, and the `lto = "thin"` line `dist init` writes would override the inherited `lto = true` (measured cost 124,664 bytes). Delete it after any `dist init` regeneration. `panic = "abort"` is safe: cargo ignores it for test targets.

### GitLab CI (dev loop)

Single stage on every push, official `rust:<pinned version>` image, four jobs, no caching:

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings` (does not lint doc-tests, so:)
3. `cargo test` — bare, no target filter, to keep doc-tests running
4. `cargo build --profile dist` then the size guard: `test "$(wc -c < target/dist/gunfog)" -le 2097152`

Release CI is separate: cargo-dist-generated GitHub Actions on the mirror (below).

## Distribution

cargo-dist is the release tool; config lives in `dist-workspace.toml` (`[workspace.metadata.dist]` is deprecated). **GitHub is the release origin**: cargo-dist supports only GitHub Actions and GitHub Releases, so the public mirror at github.com/enjunear is a hard requirement, not a preference. The self-hosted GitLab stays the development home. This split deserves an ADR when implementation starts.

- Trigger: `v`-prefixed plain semver tags pushed to the mirror, starting at v0.1.0. No prerelease or nightly channels.
- Platform matrix: macOS arm64 + x64, Linux x64 + arm64 (musl static), Windows x64 zip. No 32-bit.
- Channels at first release:
  1. Homebrew: formula `gunfog` in the shared tap `enjunear/homebrew-tap`, pushed automatically via a tap PAT.
  2. npm: unscoped package `gunfog`, biome-style platform packages (`os`/`cpu`-gated optionalDependencies carrying the native binary, JS shim `bin`, no postinstall), so `bunx gunfog` works with zero install.
  3. Shell installer script from the release page.
  4. crates.io: cargo-dist has no crates.io publish job (`publish-jobs` supports only homebrew, npm, custom), so `cargo publish` is a manual step with a named owner.
- Deferred: Windows MSI (until a Windows user asks); cargo-binstall metadata (only if the conventional asset names don't already work).
- Canonical invocation: `gunfog` on PATH via brew or the shell installer. `bunx gunfog` is the documented no-install fallback; it pays cold-start resolution, so repeat callers should install.

## Out of scope

- Multiple files per invocation (agents loop)
- JSON output (no machine consumer exists)
- Readability metrics beyond Gunning fog
