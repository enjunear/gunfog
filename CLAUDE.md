# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

`gunfog` is a CLI that scores prose with the Gunning fog index and reports hotspots so a coding agent can revise its own writing. 
Token-efficient output is the prime directive.

Check what stage the repo is at before assuming anything runs: `ls Cargo.toml src 2>/dev/null`. 
No manifest means the repo is still planning documents, and the "commands" below describe what CI will run, not what works today.

## Where the decisions live

- `CONTEXT.md` is the glossary and the authoritative vocabulary (prose, sentence, word, complex word, placeholder word, contribution, hotspot, target, floor). 
    Read it before writing code or docs that use those terms, and keep the terms out of any other meaning.
- `SPEC.md` states what to build, without rationale. 
    It sits at the repo root; if it is not there, `git log --all --oneline -- SPEC.md` names the branch carrying it and `git show <branch>:SPEC.md` reads it.
- `docs/research/` holds the rationale and the measurements. `ls docs/research` for the current set. 
    Every note opens with a verdict section; read that before the body, and follow the sources at the end rather than trusting a summary.

Precedence when they disagree: the spec says what to build, research says why. 
Update the losing document in the same change instead of leaving both standing.

## Commands

Read the gates from the repo, not from memory. 
`.gitlab-ci.yml` owns the dev-loop commands and `rust-toolchain.toml` owns the compiler version and components. 
`grep -A5 'script:' .gitlab-ci.yml` and `cat rust-toolchain.toml`.

Four jobs, one stage, every push, no cache:

- `cargo fmt --check`. 
    The only one of the four that takes no `--locked`.
- `cargo clippy --all-targets --locked -- -D warnings`. 
    Note `--all-targets` does not lint doc-tests.
- `cargo test --locked`, bare and unfiltered, because that is what compiles and runs the doc-tests clippy skipped. 
    Do not narrow it to `--lib` in CI. 
    Locally, a single test is `cargo test <substring>` and one integration file is `cargo test --test <file>`.
- `cargo build --profile dist --locked` plus a size guard: `test "$(wc -c < target/dist/gunfog)" -le 2097152`.

The image is `rust:<pinned version>-slim`, tagged to the patch version so it stays in lockstep with the `rust-toolchain.toml` channel and `rust-version`. 
Bump all three in one commit; a mismatch costs about 31.5 s per job re-downloading a toolchain.

Two traps measured in `docs/research/rust-project-conventions.md`:

- Never set `RUSTFLAGS=-Dwarnings`. 
    It is part of Cargo's fingerprint and rechecks every dependency (2.55 s against 1.05 s on the spike). 
    The `-- -D warnings` form applies to the final crate only.
- The shipped binary is built with `--profile dist`, not `--release`. 
    If `dist init` re-adds `lto = "thin"` under `[profile.dist]`, it overrides the inherited `lto = true` and costs about 124 KB. 
    That block should contain `inherits = "release"` and nothing else.

## Architecture

Single package, lib plus thin bin. 
`src/lib.rs` exposes segmentation, syllable, scoring and report modules; `src/main.rs` is CLI wiring. 
Tests call the library.

The pipeline is markdown, then prose extraction, sentence segmentation, word tokenisation, syllable counting, complex-word classification, a document score, per-sentence contributions, and the report. 
The parts that are easy to get wrong, each backed by a research note:

- **The formula only ever sees prose.** Headings, code blocks, tables, image alt text, raw HTML and task-list markers are removed before scoring. 
    Inline code spans and bare URLs become a one-word, one-syllable placeholder rather than disappearing, so the sentence keeps its length.
- **Segmentation is the tool.** How a bulleted list is split into sentences moves the grade by about 3.9 on average, eight times the effect of the syllable counter and more than twice the spread between whole documents. 
    A list item's start and end are always boundaries. See `docs/research/fog-vs-flesch-kincaid.md`.
- **Hotspots are leave-one-out contributions**, `Fog(doc) - Fog(doc minus this sentence)`, computed from running totals. 
    Scoring each sentence on its own is close to the wrong signal: it overlapped correct attribution on 14 of 100 sentences and ranked six-word sentences as the worst offenders.
- **Short input gets no score.** Below 100 prose words both formulas are noise, so the run refuses and exits 1 while still naming complex words.
- **Syllables are rules, not a word list.** The 29-rule byte-comparison set in `docs/research/syllable-counting.md` §4. 
    The CMU-derived list is a deliberate 524 KB that is not shipped; adding it later as a first-check layer is purely additive.
- `pulldown-cmark`'s `into_offset_iter()` yields byte offsets; prose extraction carries them through as source spans, and segmentation turns each sentence's span into its line number with one forward newline scan.

Calibration targets Gunning's own worked example and a hand-counted golden corpus, not agreement with textstat or the npm packages, which diverge from each other by up to 4.9 grades.

## Distribution split

Development lives on the self-hosted GitLab (`git remote -v`). 
Releases have to originate from a GitHub mirror, because cargo-dist supports only GitHub Actions and GitHub Releases. 
That constraint reaches into `Cargo.toml`, whose `repository` field must name the mirror, which is then the URL crates.io, Homebrew and npm advertise. 
Write the ADR when implementation starts.

## Working in this repo

- Feature branch and MR, conventional commits. The `/glab` skill covers the CLI mechanics.
- Research notes share a shape: verdict first, then measurements stamped with the machine and date they were taken on, then a source list of URLs. 
    Match it when adding one, and say when a claim rests on a secondary source.
- Older notes call the tool `gfog` or `g-fog`; it was renamed (`docs/research/naming-collisions.md`). 
    Leave the historical text as written rather than rewriting past documents.
