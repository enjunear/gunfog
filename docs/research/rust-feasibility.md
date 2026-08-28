# Rust feasibility for gfog

Research for the `g-fog` planning effort. Date: 2026-08-28.

Question: can Rust build `gfog` well, and does it deliver the ~2 MB single-binary
Homebrew-installable goal?

Companion to `existing-readability-tools.md`, which recommended TypeScript/Bun on cold-start
and building-block grounds. This note revisits that recommendation now that the ~2 MB binary
target is on the table.

Every size and timing number below was measured on this machine: Linux x86_64, 16 cores,
rustc 1.98.0 (2026-08-18), cargo 1.98.0, Bun 1.4.0. The spike projects live in the session
scratchpad, not in this repo. The dependency sets and build profiles are reproduced inline so
the numbers can be re-run.

## Verdict up front

**Rust hits the target with room to spare, and Bun cannot come close.**

A working spike of the actual tool, doing markdown parsing with source offsets, sentence
segmentation, per-sentence fog and complex-word hotspots, weighs **753 KB** stripped and
starts in **1.0 ms**. Adding `clap` and a rules engine brings it to **936 KB**. Even the
heaviest sensible configuration, with a 51,628-word polysyllabic dictionary embedded, lands
around **1.4 MB**. The 2 MB budget is not tight. It is roughly double what the tool needs.

The same program compiled with `bun build --compile --minify` is **82,535,624 bytes (78.7 MiB)**
and starts in **6.5 ms**. That is 88x the size and 6.5x the startup. The 50-100 MB figure in
the brief was accurate, at the top of the range.

Three things changed my read of the earlier recommendation:

1. **The cold-start argument now favours Rust, not TypeScript.** The earlier note compared Bun
   against Python and found Bun 58x faster to start. Against Rust, Bun is the slow one.
2. **The building-block argument mostly dissolves**, exactly as the brief suspected. `gfog`
   defines its own complex-word and segmentation rules regardless, so "npm has more packages"
   buys little. The one place it still bites is syllable counting, where npm's `syllable` is
   genuinely better than anything on crates.io and Rust has to pay for parity.
3. **Rust's segmentation is better out of the box.** UAX #29 handles decimal numbers, version
   strings, URLs and lowercase-following abbreviations correctly. Those were four of the six
   segmentation failures documented in section 4 of the prior note.

The cost of choosing Rust is one bounded piece of work: roughly 400 lines of ported syllable
rules, or a 524 KB embedded word list. Section 4 prices both.

## 1. Building blocks

### 1.1 Markdown parsing: pulldown-cmark

**pulldown-cmark** v0.13.4, published 2026-05-20. MIT. MSRV 1.71.1.
(https://crates.io/api/v1/crates/pulldown-cmark, https://github.com/pulldown-cmark/pulldown-cmark)

Mature and active. Last push 2026-08-17. 141.7M downloads total, 42.3M in the last 90 days.
The project moved out of `raphlinus/pulldown-cmark` into its own GitHub org; the README credits
Martín Pozo, Michael Howell, Roope Salmi and Martin Geisler with driving it since 2023.

It vendors the CommonMark spec it tests against at `third_party/CommonMark/spec.txt`, pinned to
**spec version 0.31.2 (2024-01-28)**, so compliance is tracked against the official suite rather
than asserted.

Source positions work, and they are the reason this crate fits. `Parser::into_offset_iter()`
yields `(Event, Range<usize>)` pairs where the range maps to the corresponding bytes in the
markdown source
(https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Parser.html). Byte offsets, not
line and column, so `gfog` builds its own newline table. That is one pass over the source.

Confirmed working in the spike. Given this input:

~~~markdown
## Configuration

The deployment pipeline requires considerable configuration before the application becomes operational.
Engineers must establish authentication credentials. It works.

Setup steps:

- Install the dependencies
- Configure the environment

The version 1.2.3 release improved performance by 12.5 percent overall. Dr. Smith agreed.

```python
x = compute_aggregate_statistics(dataframe)
```

See https://example.com/docs/v1.2/index.html for details.
~~~

the spike produces:

```
fog 13.8  target 9  47 words  10 sentences
L3 fog 26.2 (11w) The deployment pipeline requires considerable configuration
    complex: deployment, pipeline, considerable, configuration, application, operational
L4 fog 26.0 (5w) Engineers must establish authentication credentials.
    complex: establish, authentication, credentials
L8 fog 14.5 (3w) Install the dependencies
    complex: dependencies
L9 fog 14.5 (3w) Configure the environment
    complex: environment
L11 fog 12.0 (10w) The version 1.2.3 release improved performance by 12.5 perce
    complex: performance, overall
```

The heading is excluded, the fenced code block is excluded, the list items become separate
sentences, and line numbers are correct. That covers four of the five preprocessing
requirements from the prior note's section 6 in about 60 lines.

Dependencies are `bitflags`, `memchr` and `unicase`. Three transitive crates, nothing else.

**Known gap.** Issue #441 (https://github.com/pulldown-cmark/pulldown-cmark/issues/441) reports
that for links and images, `into_offset_iter()` gives the range of the whole tag rather than
separate ranges for the URL, title and link text. Plain `Text` events inside emphasis still
carry their own correct ranges. `gfog` walks `Text` events, so this does not affect it.

**Alternatives.** `comrak` v0.54.0 (2026-07-12, BSD-2-Clause) also reports source positions,
with a `--sourcepos-chars` option for character rather than byte columns, but it builds a full
arena-allocated AST and drags in `clap`, `phf`, `typed-arena`, `jetscii` and others. Its own
docs concede it "is not and will not be the fastest" against non-AST parsers. The `markdown`
crate v1.0.0 (2025-04-23, MIT, https://github.com/wooorm/markdown-rs) is the interesting one:
its mdast nodes carry line, column **and** byte offset natively, which removes the newline-table
step. It is `no_std` + alloc with almost no dependencies. The tradeoff is age; 1.0 shipped 16
months ago against pulldown-cmark's decade. I would start with pulldown-cmark and treat
`markdown` as the fallback if offset handling gets fiddly.

### 1.2 Sentence segmentation: unicode-segmentation

**unicode-segmentation** v1.13.3, published 2026-06-01. Dual MIT/Apache-2.0. MSRV 1.85.0.
Official `unicode-rs` crate, actively maintained, recently updated for Unicode 17.0.0.
(https://github.com/unicode-rs/unicode-segmentation)

It implements UAX #29 sentence boundaries and exposes `unicode_sentences()`,
`split_sentence_bounds()` and `split_sentence_bound_indices()`
(https://docs.rs/unicode-segmentation/latest/unicode_segmentation/trait.UnicodeSegmentation.html).
The third is the one `gfog` wants: it returns byte-offset-tagged sentence substrings, which
chains directly onto pulldown-cmark's offsets to give a word its line number.

**This is much better than the prior note's regex-splitting results.** Measured directly with
`unicode_sentences()`:

| Input | Sentences | Correct? |
| --- | --- | --- |
| `The version 1.2.3 release improved performance by 12.5 percent overall.` | 1 | yes |
| `Roughly 12.5% of tests failed. That is high.` | 2 | yes |
| `Version 2.0 shipped. Version 3.0 did not.` | 2 | yes |
| `See https://example.com/docs/v1.2/index.html for details. Then run it.` | 2 | yes |
| `Use e.g. a linter. It helps.` | 2 | yes |
| `The build failed at 3 p.m. yesterday. Try again.` | 2 | yes |
| `The U.S. team shipped. Others did not.` | 2 | yes |
| `He said "stop." She left.` | 2 | yes |
| `Install the dependencies\nConfigure the environment` | 2 | yes |
| `Dr. Smith agreed. The meeting ran late.` | **3** | **no** |

The prior note found that textstat's regex splitter turned `1.2.3` into three sentences. UAX #29
gets it right, along with `12.5`, URLs with dotted path segments, `e.g.`, `p.m.` and `U.S.`.

The single failure class is an abbreviation followed by a capitalised word. The spec explains
why, and admits it. From https://www.unicode.org/reports/tr29/:

> Plain text provides inadequate information for determining good sentence boundaries. Periods
> can signal the end of a sentence, indicate abbreviations, or be used for decimal points, for
> example.

> As with the other default specifications, implementations are free to override (tailor) the
> results to meet the requirements of different environments or particular languages.

And in the notes after rule SB11:

> These rules permit breaks in strings such as those shown in Figure 4. They cannot detect
> cases such as "...Mr. Jones..."; more sophisticated tailoring would be required to detect
> such cases.

The mechanics are three rules:

- **SB6**, `ATerm × Numeric`: no break between a period and a following digit. This is what
  saves `12.5` and `1.2.3`.
- **SB7**, `(Upper | Lower) ATerm × Upper`: no break when a period sits between two letters,
  which handles `U.S.A`.
- **SB8**: no break if the next actual letter after the period is lowercase. This is why
  `e.g. a linter` and `U.S. team` work, and precisely why `Dr. Smith` does not. `Smith` is
  uppercase, so nothing suppresses the break.

For fog specifically this matters: a spurious break after `Dr.` shortens mean sentence length
and pulls the score down. The fix is a `HashSet<&str>` of titles and abbreviations applied as a
merge pass over the boundaries. Maybe 30 lines and a list.

**Alternatives, all worse.** `srx` v0.1.4 (2023-07-17, dual MIT/Apache-2.0,
https://github.com/bminixhofer/srx) implements the SRX 2.0 standard but ships no rules; you
supply the `.srx` file. Its parent project `nlprule` has had no commits since 2023-05-23. The
`punkt` crate v1.0.5 (published 2019-02-26, last push 2020-01-27,
https://github.com/ferristseng/rust-punkt) would fix abbreviations statistically and ships
pretrained English data, but its README says "I am no longer maintaining this library."
`text-splitter` (MIT, active) delegates to `icu_segmenter` and inherits the same UAX #29
behaviour, and its own README says so: "we rely on unicode method of sentence boundaries, which
in most cases is good enough." `icu_segmenter` (ICU4X, Unicode-3.0 licence) needs compiled break
data through a data provider, which is real machinery and real bytes for the same algorithm.

Use `unicode-segmentation` plus a hand-written abbreviation list.

### 1.3 Syllables and complex words: the weak spot

This is where Rust genuinely loses ground, and it is worth being blunt about it.

I hand-labelled 31 English words with their syllable counts and ran every candidate against
them. Same list, same labels, four implementations:

| Approach | Correct | Notes |
| --- | --- | --- |
| npm `syllable` v5.0.1 (MIT) | **28/31** | Regex rule set plus a problem-word dictionary |
| `syllarust` v0.3.0 (Apache-2.0) | 27/31 | Embeds all 3.6 MB of CMUdict, disqualified on size |
| `hyphenation` v0.8.4, breaks + 1 | 20/31 | Systematically undercounts |
| Hand-rolled vowel-group heuristic | 18/31 | The `gunning-fog` crate's approach |

**The `hyphenation` crate is not a syllable counter, and counting its hyphen points does not
approximate one.** v0.8.4, published 2021-08-19, last push 2024-01-24, dual Apache-2.0/MIT
(https://github.com/tapeinosyne/hyphenation). Nothing in its README or docs claims otherwise;
it implements Knuth-Liang TeX patterns, which mark where a line break looks acceptable, not
where syllables divide. Measured with `embed_en-us`, every single error was an undercount:

```
readability      breaks=2 est=3 truth=5 MISS
area             breaks=0 est=1 truth=3 MISS
idea             breaks=0 est=1 truth=3 MISS
chocolate        breaks=1 est=2 truth=3 MISS
camera           breaks=1 est=2 truth=3 MISS
family           breaks=1 est=2 truth=3 MISS
table            breaks=0 est=1 truth=2 MISS
```

Knuth-Liang patterns refuse breaks that would strand fewer than two characters at the start or
three at the end of a line, and skip ambiguous positions entirely. Vowel hiatus (`a-re-a`,
`i-de-a`) is invisible to them. A one-way undercount is the worst possible bias for fog, because
it drags words below the 3-syllable complexity threshold and makes prose look easier than it is.
Rule this out.

**`syllarust` v0.3.0** (2026-03-31, Apache-2.0, https://github.com/cyalen/syllarust) is
accurate, and disqualified on both size and speed. It embeds the CMU Pronouncing Dictionary via
`include_str!("../cmudict/cmudict.dict")`, a 3,618,488-byte text file, and parses it into a
HashMap at first use. Measured: **5,181,840-byte binary** and **28.8 ms startup**, against
753 KB and 1.0 ms for the spike. It is 2.6x over the size budget on its own and 29x slower to
start.

**The `gunning-fog` crate v0.1.0** (2024-08-07, https://github.com/astuanax/gunning-fog) is a
naive vowel-group counter, the same family the prior note found in every other language:

```rust
fn syllable_count(word: &str) -> usize {
    let vowels = Regex::new(r"[aeiouy]+").unwrap();
    let mut count = vowels.find_iter(word).count();
    if word.ends_with('e') { count -= 1; }
    count.max(1)
}
```

It encodes the complex-word rule as "3+ syllables" with no exclusions, which the prior note
measured as the naive end of the 4.9-grade spread. Read it as a reference, do not depend on it.

**Ruled out:** the `syllable` crate v0.1.0 (2021-03-02, 2,881 total downloads, `repository:
null` on crates.io, so the source cannot be audited); `cmudict` v0.3.2 (last released
2018-09-07, pins `reqwest` 0.8 and the deprecated `failure` and `tempdir`, unlikely to build);
`rust_readability` (GPL-3.0, which would make the whole binary copyleft).

**So what does Rust actually do?** Two options, both measured in section 4. Port the `syllable`
package's rules, or embed a word list. Neither is hard; both cost less than 600 KB.

### 1.4 Argument parsing

| Crate | Version | Licence | Binary cost over the spike |
| --- | --- | --- | --- |
| `lexopt` | 0.3.2 (2026-02-28) | MIT | baseline (753,376 bytes) |
| `clap` (derive) | 4.6.6 (2026-08-06) | MIT/Apache-2.0 | **+182,696 bytes** (936,072) |

`clap` costs 183 KB and brings `anstream`, `anstyle`, `clap_lex`, `strsim` and the derive macro.
At a 2 MB budget that is affordable, and it buys generated `--help`, subcommands and shell
completions. `lexopt` is 400 lines of zero-dependency parsing if the surface stays tiny.
`pico-args` (0.5.0, last released 2022-06-04) and `argh` (0.1.19, Google) are the other light
options; `argh` explicitly optimises for code size.

Given the budget has room, take `clap`. The `--help` output is part of how a coding agent
discovers the tool.

## 2. Binary size and startup, measured

All builds used this profile unless noted:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

The spike is ~220 lines implementing markdown prose extraction with offsets, UAX #29
segmentation, suffix-stripping syllable counting, the canonical complex-word exclusions,
per-sentence fog and hotspot output with line numbers. Not a hello-world.

| Build | Bytes | MB |
| --- | --- | --- |
| Rust hello-world floor | 291,784 | 0.28 |
| **Spike: pulldown-cmark + unicode-segmentation + lexopt** | **753,376** | **0.72** |
| Spike, `opt-level = 3` instead of `"z"` | 828,624 | 0.79 |
| Spike + `regex-lite` | 812,632 | 0.78 |
| Spike + embedded 51,628-word polysyllabic list | 819,448 | 0.78 |
| Spike with `clap` instead of `lexopt` | 936,072 | 0.89 |
| **Recommended: pulldown-cmark + unicode-segmentation + clap + regex-lite** | **936,056** | **0.89** |
| Recommended, `x86_64-unknown-linux-musl` static | 1,024,664 | 0.98 |
| + `hyphenation` with embedded en-US | 1,101,408 | 1.05 |
| Spike + `regex` (reduced features) | 1,435,408 | 1.37 |
| Spike + `regex` (default features) | 1,939,144 | 1.85 |
| `syllarust` (embeds full CMUdict) | 5,181,840 | 4.94 |
| **Bun `--compile --minify`, equivalent program** | **82,535,624** | **78.7** |

Two size traps worth knowing.

**Stripping is most of the win.** The same spike built without `strip`, `lto` or
`panic = "abort"` is 5,157,664 bytes. Stripped by hand it drops to 915,480. Debug symbols are
about 4.2 MB of the 5 MB. Anyone who measures a Rust binary without setting `strip = true` will
conclude the budget is blown when it is not.

**The full `regex` crate is the one thing that can blow the budget.** At default features it
adds 1.19 MB, nearly the entire allowance, because it compiles in the full Unicode character
class tables. `regex-lite` v0.1.9 (MIT/Apache-2.0) adds **59,256 bytes** for the same
backtracking-free API on ASCII-ish patterns. For syllable rules, which are all ASCII letter
classes, `regex-lite` is the right call and the difference is 20x.

### Startup

Measured with `hyperfine -N`, 20 warmup runs, minimum 100 timed runs:

| Binary | Mean | σ |
| --- | --- | --- |
| Rust spike (lexopt) | **1.0 ms** | 0.1 |
| Rust + clap + hyphenation | 1.6 ms | 0.2 |
| Rust + embedded 51,628-word list | 2.5 ms | 0.3 |
| Bun compiled binary | 6.5 ms | 0.6 |
| `bun run gfog.ts` (no compile step) | 6.5 ms | 0.6 |
| Rust + syllarust (CMUdict parse at startup) | 28.8 ms | 0.8 |

Rust is 6.5x faster to start than Bun. Note that `bun build --compile` buys no startup
improvement over `bun run` on a script this small; the 79 MB is paying purely for
self-containment.

Note also what the syllarust row shows: embedding data is free, but *parsing* it at startup is
not. The 51,628-word list costs 1.5 ms because it builds a `Vec<&str>` in a `OnceLock` on first
use. A sorted-blob binary search or an `fst` would cut that to near zero.

### Build times

Relevant because coding agents will iterate on this. The spike has 6 transitive dependencies
total (`bitflags`, `memchr`, `unicase`, `pulldown-cmark`, `unicode-segmentation`, `lexopt`).

| Operation | Time |
| --- | --- |
| Cold `cargo build --release` (LTO, opt-z, from empty target dir) | 4.44 s |
| Cold `cargo build` (debug) | 1.10 s |
| Incremental debug rebuild after touching `main.rs` | 0.11 s |
| Incremental `cargo check` | 0.64 s |

Rust's reputation for slow builds comes from large dependency graphs. This one has six crates.
The edit-compile-test loop is a tenth of a second.

### Cross-compilation

**Linux static builds work locally with zero setup.** `rustup target add
x86_64-unknown-linux-musl` then `cargo build --release --target x86_64-unknown-linux-musl`
produced a 1,024,664-byte binary that `file` reports as `ELF 64-bit LSB pie executable,
static-pie linked, stripped`. No musl toolchain, no container, no C compiler configuration. That
one artifact runs on any glibc or musl Linux.

**macOS cannot be cross-compiled from Linux without the Apple SDK.** Adding
`aarch64-apple-darwin` and building fails at the link step:

```
warning: invoking `"xcrun" "--sdk" "macosx" "--show-sdk-path"` to find MacOSX.sdk failed
error: linking with `cc` failed: cc: error: unrecognized command-line option '-arch'
```

This is a non-issue in practice: GitHub Actions provides `macos-14` arm64 and `macos-13` x64
runners, and a release workflow builds each target natively on its own runner. `cargo-zigbuild`
is the alternative if everything must build on one machine.

Tier 1 targets covering the stated matrix: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`,
`x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`. Nothing exotic is needed.

## 3. Distribution

### 3.1 cargo-dist is alive, and the "Astral took it over" story is backwards

Worth stating clearly because the brief flagged uncertainty here.

The canonical repo is still **https://github.com/axodotdev/cargo-dist**. Not archived, 2.1k
stars, dual MIT/Apache-2.0. Latest release **v0.32.0, 2026-05-22**, with commits continuing at
roughly weekly cadence through today. 298 open issues.
(https://github.com/axodotdev/cargo-dist/releases, https://crates.io/api/v1/crates/cargo-dist)

The Astral fork at https://github.com/astral-sh/cargo-dist **was archived on 2025-12-19**. Its
README describes it as an unofficial fork made to backport fixes for Astral's own projects and
directs users back upstream because axodotdev's project is active again. Per the axodotdev
changelog, v0.29.0 folded the fork's improvements back in. So the fork was absorbed, not the
reverse.

The tool rebranded to `dist` (README: "`dist` (formerly known as `cargo-dist`)") in v0.24.0,
2024-10-28, but the crates.io package is still `cargo-dist`. The `dist` crate name belongs to an
unrelated dormant 2017 crate. Docs moved: `opensource.axo.dev/cargo-dist` no longer resolves,
current docs are at **https://axodotdev.github.io/cargo-dist/**.

Installers supported today, confirmed live in the docs
(https://axodotdev.github.io/cargo-dist/book/installers/index.html): **shell, PowerShell, npm,
Homebrew formula, Windows MSI**. Docker, Flatpak, macOS Cask, PyPI and WinGet are requested but
not implemented.

That is exactly the set `gfog` wants, from one config block. Homebrew tap and npm package
generated together, cross-compiled artifacts and a GitHub Release built by generated CI.

### 3.2 If cargo-dist is not wanted

No single drop-in replacement exists. The pieces:

- **goreleaser** gained a `builder: rust` in v2.5, shelling out to `cargo zigbuild`
  (https://goreleaser.com/customization/builds/rust/). Catch: its `brews` config, which
  generates a real Formula, is **deprecated as of v2.10** in favour of `homebrew_casks`.
  goreleaser's reasoning is that Formulas are "supposed to build from source"
  (https://github.com/goreleaser/goreleaser/discussions/5563). That is a goreleaser opinion, not
  a Homebrew rule. Casks work for CLI binaries but mean `brew install --cask`.
- **taiki-e/upload-rust-binary-action** builds and uploads to GitHub Releases only, no Homebrew.
  Active, v1.30.2. (https://github.com/taiki-e/upload-rust-binary-action)
- **Formula bump actions** exist and are maintained, but only bump an existing formula's version
  and sha256: `mislav/bump-homebrew-formula-action` (last push 2026-08-17) and
  `dawidd6/action-homebrew-bump-formula` (last push 2026-07-02, needs a PAT because the default
  `GITHUB_TOKEN` cannot fork and open a PR).
- **cargo-release** (v1.1.5, 2026-08-11) does version bumping and `cargo publish` only. Different
  problem.

**Homebrew does not require a bottle.** A formula that downloads a prebuilt tarball and installs
the binary is the documented standard pattern (https://docs.brew.sh/Formula-Cookbook):

```ruby
class Foo < Formula
  desc "A sample application"
  homepage "https://example.com"
  url "https://example.com/foo-0.1.tar.gz"
  sha256 "abc123..."
  license "BSD-2-Clause"
  def install
    bin.install "somebinary"
  end
end
```

The tap repo must be named `homebrew-<name>` for `brew tap user/<name>` to work
(https://docs.brew.sh/Taps).

### 3.3 npm distribution, so agents can `bunx gfog`

This is the pattern that keeps the "agents can reach it without installing anything" property
that Bun would have given for free. Verified against live registry JSON, not from memory.

**esbuild** 0.28.2 (https://registry.npmjs.org/esbuild/latest): `"bin": {"esbuild":
"bin/esbuild"}`, a `postinstall: node install.js`, and 26 `@esbuild/<platform>`
`optionalDependencies` pinned to the same version. Each platform package, for example
`@esbuild/linux-x64`, declares `"os": ["linux"], "cpu": ["x64"]` and ships the raw binary with
no `bin` field of its own.

**@biomejs/biome** 2.5.11 (https://registry.npmjs.org/@biomejs/biome/latest): same shape,
`"bin": {"biome": "bin/biome"}`, 8 `@biomejs/cli-*` optionalDependencies including musl
variants, and **no postinstall script at all**. The `bin/biome` shim does platform detection at
runtime, including parsing `ldd --version` to spot musl.

**The mechanism is entirely declarative.** npm, yarn and pnpm read each dependency's `os` and
`cpu` fields and skip any that do not match the running machine. Because they are
`optionalDependencies`, a mismatch does not fail the install. Exactly one platform package lands
in `node_modules`, and the `bin` shim resolves it. `npx` and `bunx` work with this for free,
same install path.

Biome's no-postinstall variant is the better model for `gfog`: fewer moving parts, and
postinstall scripts are commonly disabled in hardened CI.

cargo-dist's npm installer generates this shape, so it comes with the same config that generates
the tap.

### 3.4 cargo-binstall

v1.22.0, released 2026-08-22, last push 2026-08-27. 2.8k stars, actively maintained.
**GPL-3.0-only** (the tool itself; it imposes nothing on what it installs).
(https://github.com/cargo-bins/cargo-binstall/releases)

With no configuration it probes GitHub release URLs of the form
`{repo}/releases/download/{version}/…` against filename patterns like
`{name}-{target}-{version}{archive-suffix}`. `[package.metadata.binstall]` overrides this with
`pkg-url`, `bin-dir`, `pkg-fmt` and per-target `overrides`. If `gfog` publishes to crates.io
with conventionally named release assets, `cargo binstall gfog` works with a few lines of
metadata. A nice extra, not a primary channel.

## 4. Closing the syllable gap

The honest cost of choosing Rust. Two options, both priced.

**Option A: port the `syllable` package's rules.** npm `syllable` v5.0.1 is MIT: 353 lines of
`index.js` plus 63 lines of `problematic.js`. The bulk is two big alternation regexes of
orthographic patterns (`'cious'`, `'[^aeiou]giu'`, `'[oa]gue$'`, `'.[^aeiuoycgltdb]{2,}ed$'` and
so on) plus a dictionary of problem words. It also depends on `pluralize` for singularisation,
which is the one piece that does not port cheaply and could be replaced with a smaller suffix
rule.

Cost: **+59,256 bytes** for `regex-lite`, and roughly 400 lines of mechanical translation. This
is exactly the kind of bounded, verifiable job a coding agent does well, and it comes with a
built-in test oracle: run both implementations over a word list and diff. Result should land
near `syllable`'s measured 28/31.

**Option B: embed a polysyllabic word list.** Fog does not need a syllable *count*. It needs a
boolean: is this word 3+ syllables? That is a much smaller thing to store.

Derived from CMUdict, filtering to lowercase alphabetic entries with 3 or more stress-marked
vowels: **51,628 words, 524,564 bytes** as a newline-separated sorted list, 170,473 gzipped.
Measured with the list embedded via `include_str!` and binary-searched: **819,448-byte binary,
2.5 ms startup**. Combined with the recommended stack that is about 1.4 MB, comfortably inside
budget.

CMUdict is BSD-2-Clause (Copyright 1993-2015 Carnegie Mellon University,
https://github.com/cmusphinx/cmudict/blob/master/LICENSE), so it embeds fine with attribution.

The catch is out-of-vocabulary words, so I measured it on real target input: the 2,614 words of
this repo's own `existing-readability-tools.md`, which is agent-written technical markdown.

- All words: **14.6% of unique tokens, 9.6% of occurrences** are outside CMUdict.
- Restricted to fog-eligible words, dropping hyphenated compounds and capitalised tokens which
  the canonical rule excludes anyway: **5.4% of unique, 3.9% of occurrences**.

The remaining misses are `linter`, `runtime`, `regex`, `stdin`, `preprocessing`, `extractor`,
`traversal`, `implementers`, plus possessives and plurals that normalisation fixes. So a
dictionary still needs a heuristic fallback, which is what both `syllable` and `syllarust` do.

**Recommendation: Option A.** A ported rule set has no vocabulary ceiling, no BSD attribution to
carry, no 524 KB of data, and 1 ms startup instead of 2.5 ms. Option B is the fallback if the
port's accuracy disappoints, and the two combine well: dictionary first, ported rules for OOV.

Whichever is chosen, note that per the prior note's section 2.2 the canonical suffix rule
("do not include common suffixes as a syllable") means `gfog` strips `-es`, `-ed`, `-ing` before
counting anyway. That is `gfog`'s own rule, and it sits on top of whatever counter is underneath.

## 5. Rust against Bun

| | Rust | Bun |
| --- | --- | --- |
| **Binary size** | 936 KB measured (0.72 MB without clap). Meets the 2 MB goal with ~55% headroom. | 78.7 MB measured. 88x over. No flag reduces this; `--compile` embeds the runtime. |
| **Cold start** | 1.0 ms (spike), 2.5 ms with an embedded dictionary. | 6.5 ms, identical compiled or interpreted. |
| **Bespoke segmentation rules** | Cheaper. UAX #29 already handles decimals, versions, URLs and lowercase-following abbreviations. One abbreviation list closes the gap. pulldown-cmark gives byte offsets for free. | The prior note's spike worked, but its regex splitting failed on `1.2.3`; retext's nlcst tree gives line/column/offset for free, which is the JS side's real strength. |
| **Syllables / complex words** | The one real loss. Best in-budget option is a ~400-line port of `syllable`'s MIT rules plus `regex-lite` (+59 KB). Nothing on crates.io is both accurate and small. | `syllable` v5.0.1 works today at 28/31 measured. Zero effort. |
| **Distribution** | cargo-dist v0.32.0 generates Homebrew tap, npm platform packages, shell/PowerShell installers, MSI and cross-compiled artifacts from one config. `cargo binstall` as a bonus. | npm publish is trivial and `bunx gfog` works immediately. Homebrew means shipping a 79 MB bottle per platform, or a formula depending on Bun, which reintroduces a runtime dependency. |
| **Agent-friendliness** | 6 transitive dependencies. `cargo check` in 0.64 s, incremental rebuild in 0.11 s. The compiler catches the class of error an agent ships silently. Agents write less Rust than TypeScript, but this program is string processing with value semantics, the part of Rust that does not fight back. | Agents write TypeScript most fluently. No compile step. Against that: the prior note documented `unist-util-visit` silently corrupting traversal because `Array.push` returns a number, which is exactly the bug class a type checker does not catch and a Rust `match` would not permit. |

Bun wins on two things: the syllable counter exists today, and agents write TypeScript more
readily. Rust wins on binary size by two orders of magnitude, on startup by 6.5x, on
distribution breadth, and on segmentation correctness.

Given that the 2 MB Homebrew binary is the *stated goal* rather than a nice-to-have, and that
88x over budget is not a gap any amount of tuning closes, the syllable port is a fair price.

## 6. Recommendation

**Build `gfog` in Rust.** The 2 MB target is met at roughly half the budget, which leaves room
for the tool to grow.

Dependency set, all MIT or dual MIT/Apache-2.0:

```toml
[dependencies]
pulldown-cmark = { version = "0.13", default-features = false }
unicode-segmentation = "1"
clap = { version = "4", features = ["derive"] }
regex-lite = "0.1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Measured at 936,056 bytes and ~1 ms startup.

Work `gfog` still owns, none of it avoidable in either language:

1. The complex-word rule with the canonical exclusions, per the prior note's section 2.2.
2. A syllable counter. Port `syllable`'s MIT rules; keep the CMU-derived word list as a fallback
   if accuracy disappoints.
3. An abbreviation merge pass over UAX #29 boundaries, for the `Dr. Smith` case.
4. A byte-offset to line-number table over the source.
5. Markdown preprocessing. Mostly done by pulldown-cmark event filtering; the spike covers
   headings, fenced code, inline code and list-item boundaries in ~60 lines.

Things to avoid, with reasons:

- **The full `regex` crate.** +1.19 MB, most of the budget, for Unicode tables the syllable
  rules do not use. Use `regex-lite`.
- **`hyphenation` as a syllable counter.** 20/31, and every error an undercount, which biases
  fog downward.
- **`syllarust`.** Accurate, but 5.2 MB and 28.8 ms startup.
- **Building without `strip = true`.** The same binary is 5.2 MB unstripped.
- **`rust_readability`.** GPL-3.0 would make the whole binary copyleft.

## Sources

Measured on this machine, 2026-08-28: rustc 1.98.0, cargo 1.98.0, Bun 1.4.0, Linux x86_64,
16 cores. Timings by `hyperfine -N` with 20 warmup and 100+ timed runs.

Crates:
- pulldown-cmark: https://github.com/pulldown-cmark/pulldown-cmark,
  https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Parser.html,
  https://crates.io/api/v1/crates/pulldown-cmark, issue #441:
  https://github.com/pulldown-cmark/pulldown-cmark/issues/441
- comrak: https://github.com/kivikakk/comrak
- markdown-rs: https://github.com/wooorm/markdown-rs
- unicode-segmentation: https://github.com/unicode-rs/unicode-segmentation,
  https://docs.rs/unicode-segmentation/latest/unicode_segmentation/trait.UnicodeSegmentation.html
- srx: https://github.com/bminixhofer/srx; nlprule: https://github.com/bminixhofer/nlprule
- rust-punkt: https://github.com/ferristseng/rust-punkt
- text-splitter: https://github.com/benbrandt/text-splitter
- icu_segmenter: https://docs.rs/icu_segmenter
- hyphenation: https://github.com/tapeinosyne/hyphenation
- syllarust: https://github.com/cyalen/syllarust, https://crates.io/crates/syllarust
- gunning-fog: https://github.com/astuanax/gunning-fog
- syllable (crate): https://crates.io/crates/syllable
- cmudict (crate): https://crates.io/crates/cmudict
- rust_readability: https://github.com/ian-nai/rust_readability
- clap: https://github.com/clap-rs/clap; lexopt: https://github.com/blyxxyz/lexopt;
  pico-args: https://github.com/RazrFalcon/pico-args; argh: https://github.com/google/argh
- regex-lite: https://crates.io/crates/regex-lite

Specs and data:
- UAX #29, Unicode Text Segmentation: https://www.unicode.org/reports/tr29/
- CommonMark spec 0.31.2, vendored at `third_party/CommonMark/spec.txt` in the pulldown-cmark repo
- CMU Pronouncing Dictionary: https://github.com/cmusphinx/cmudict,
  licence https://github.com/cmusphinx/cmudict/blob/master/LICENSE

Distribution:
- cargo-dist: https://github.com/axodotdev/cargo-dist,
  https://axodotdev.github.io/cargo-dist/book/installers/index.html,
  https://crates.io/api/v1/crates/cargo-dist; archived fork:
  https://github.com/astral-sh/cargo-dist
- goreleaser Rust builder: https://goreleaser.com/customization/builds/rust/;
  brews deprecation discussion: https://github.com/goreleaser/goreleaser/discussions/5563
- taiki-e/upload-rust-binary-action: https://github.com/taiki-e/upload-rust-binary-action
- mislav/bump-homebrew-formula-action: https://github.com/mislav/bump-homebrew-formula-action
- dawidd6/action-homebrew-bump-formula: https://github.com/dawidd6/action-homebrew-bump-formula
- cargo-release: https://github.com/crate-ci/cargo-release
- Homebrew: https://docs.brew.sh/Formula-Cookbook, https://docs.brew.sh/Taps
- npm native-binary pattern: https://registry.npmjs.org/esbuild/latest,
  https://registry.npmjs.org/@esbuild/linux-x64/latest,
  https://registry.npmjs.org/@biomejs/biome/latest, https://registry.npmjs.org/@swc/core/latest
- cargo-binstall: https://github.com/cargo-bins/cargo-binstall,
  https://github.com/cargo-bins/cargo-binstall/blob/main/SUPPORT.md

npm `syllable` v5.0.1 (MIT) inspected locally at `node_modules/syllable`:
https://github.com/words/syllable
