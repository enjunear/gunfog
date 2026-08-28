# Rust project conventions for gunfog

Research for the `gunfog` planning effort. Date: 2026-08-28.

The planning map has already picked a project layout, a toolchain policy, a lint policy, a
release profile, and a two-sided CI split. This note checks each of those against the documents
that own the behaviour, and measures the ones that can be measured. It is a companion to
`rust-feasibility.md`, which established the language choice and the binary-size budget.

Every measurement below was taken on this machine on 2026-08-28: Linux x86_64, 16 cores,
rustc 1.98.0 (88d9e12ae 2026-08-18), cargo 1.98.0 (797e8a9bc 2026-08-05), clippy 0.1.98,
rustfmt 1.9.0-stable, Docker 29.7.2. The spike used to measure binary sizes is a ~90-line
program that really parses markdown with `pulldown-cmark`, really segments with
`unicode-segmentation`, and really parses arguments with `clap` derive, so nothing is dead-code
eliminated. It lives in the session scratchpad, not in this repo.

## Verdicts

| | Decision | Call |
| --- | --- | --- |
| A | Release profile governs shipped artifacts | **Amend.** cargo-dist builds `--profile dist`, and its generated profile silently overrides your LTO setting. Costs 689 KB. |
| B | `rust-version` mirrors the 1.98.0 pin | **Stands, with a cost to write down.** It blocks `cargo install` for anyone below 1.98.0. |
| C | `cargo clippy --all-targets -- -D warnings` | **Stands.** It is what the Clippy book recommends and the cache-friendly form. One caveat about doc-tests. |
| D | Edition 2024, no migration concerns | **Stands.** Confirmed default; nothing to migrate. |
| E | GitLab CI shape | **Amend on two points.** The official Rust image has no clippy and no rustfmt. Caching is a net loss here. |
| F | Nothing else needed | **Amend.** Five gaps would bite. The rest are correctly absent. |

## A. cargo-dist builds a different binary than your CI measures

**The decision as written does not hold.** The measured release profile governs
`cargo build --release`. It does not govern what users download.

### cargo-dist uses its own profile

From the "A Simple Application" guide
(https://axodotdev.github.io/cargo-dist/book/workspaces/simple-guide.html):

> First let's talk about `[profile.dist]`. This is a custom Cargo Profile that dist will use to
> build your app. If you want to, you can use it yourself by passing `--profile=dist` to cargo
> (i.e. `cargo run --profile=dist`). We define a separate profile from the normal "release" one
> so that you can be comfortable giving your Shippable Builds more aggressive settings without
> making local development too tedious.

> In this case the default profile dist recommends is essentially the same as --release (hence
> `inherits = "release"`), but with thin LTO enabled (`lto = "thin"`).

> dist uses the existence of `[profile.dist]` in your Cargo.toml to detect if your project has
> been properly initialized, and will generally refuse to run other commands otherwise. Sorry
> but you can't delete the profile!

`dist init` writes this block, per the Rust quickstart
(https://axodotdev.github.io/cargo-dist/book/quickstart/rust.html): it will "add a shippable
build profile to your `Cargo.toml`". The block it writes is:

```toml
[profile.dist]
inherits = "release"
lto = "thin"
```

cargo-dist's own repository carries exactly that block and nothing more
(https://github.com/axodotdev/cargo-dist/blob/main/Cargo.toml). There is no config key to point
dist at a different profile; the name `dist` is how it detects an initialised project.

### What that costs, measured

Same source, same dependencies, four manifests. Binary is `target/<profile>/gf` on
x86_64-unknown-linux-gnu.

| Manifest | Built with | Bytes |
| --- | --- | --- |
| Stock `[profile.dist]`, no custom `[profile.release]` | `--profile dist` | **1,612,424** |
| Measured `[profile.release]` + stock `[profile.dist]` | `--profile dist` | **1,047,712** |
| Measured `[profile.release]` + `[profile.dist]` with only `inherits = "release"` | `--profile dist` | **923,048** |
| Measured settings restated under `[profile.dist]` | `--profile dist` | **923,048** |
| Measured `[profile.release]` | `--release` | **923,048** |

Two things fall out of this.

**`inherits = "release"` does pick up your settings.** Rows three and four are byte-identical to
the `--release` build. Cargo's `inherits` starts from the release profile *as you configured
it*, not from Cargo's stock release defaults. So the five measured settings do reach the dist
profile through inheritance.

**The one line dist writes is the one that undoes the work.** `lto = "thin"` is set directly on
`[profile.dist]`, so it beats the inherited `lto = true`. That single line costs 124,664 bytes,
13.5% of the binary. `opt-level = "z"`, `codegen-units = 1`, `panic = "abort"` and `strip = true`
all survive.

And if the project ever loses its custom `[profile.release]`, the shipped binary is 1,612,424
bytes, 1.75x what CI reports and 77% of the 2 MiB budget, while the GitLab size guard keeps
reporting 923 KB and passing. The guard would be measuring an artifact nobody downloads.

### Amendment

1. After `dist init`, delete the `lto = "thin"` line so the block is exactly:

   ```toml
   [profile.dist]
   inherits = "release"
   ```

   Keep the five measured settings in `[profile.release]` as the single source of truth.
   Restating all five under `[profile.dist]` also works and is equally correct; it is more
   explicit but gives you two blocks that can drift.

2. **Point the GitLab size guard at `cargo build --profile dist`, not `--release`.** This is the
   real fix. It makes the guard measure the artifact that ships, and it fails the build if a
   future `dist init` or `dist generate` re-adds `lto = "thin"`. The dist profile exists in the
   repo either way, so this costs nothing.

### No conflict with `panic = "abort"` or `strip = true`

Both were live concerns. Neither is real.

`panic = "abort"` does not break tests. Cargo ignores the panic setting for test targets;
measured `cargo test --release` against a manifest with `panic = "abort"` in
`[profile.release]`, and both a normal test and a `#[should_panic]` test passed. It also cannot
break dist's CI, because dist's generated `release.yml` runs no test step at all
(https://github.com/axodotdev/cargo-dist/blob/main/.github/workflows/release.yml). GitLab keeps
owning the tests, which is what decision 6 already says.

`strip = true` does not conflict with debuginfo. Current dist sets neither `debug` nor
`split-debuginfo`, so there is nothing to collide with. Even if it did, measured
`debug = true` + `split-debuginfo = "packed"` + `strip = true` together: the binary stayed
stripped at 292,744 bytes against 292,632 without, and the symbols went to a separate `.dwp`
file. An early design discussion (https://github.com/axodotdev/cargo-dist/issues/118) mentions
`debug=true` and `split-debuginfo='packed'` as former defaults, but they are absent from today's
book and from dist's own manifest.

### Metadata cargo-dist needs, and one conflict

From the config reference (https://axodotdev.github.io/cargo-dist/book/reference/config.html),
on `repository`:

> This is *essentially* required as almost all dist features are blocked behind knowing where
> your project is hosted. All distable packages must agree on this value.

And from troubleshooting (https://axodotdev.github.io/cargo-dist/book/troubleshooting.html):

> The most common issue users encounter here is not having a defined Source Host, which
> basically just means you need to audit the `[package].repository` values you set in your
> Cargo.tomls and make sure they consistently point to your GitHub repo.

**This conflicts with the GitLab-primary plan.** `gunfog` develops on
`git.enjunear.com/enjunear/gunfog`, but `repository` has to name the GitHub mirror for dist to
work. That URL is also what crates.io, the Homebrew formula and the npm package will show as the
project's home. The planning session should decide that deliberately rather than discover it
during `dist init`.

`description`, `license`, `readme`, `homepage` and `authors` are all optional to dist and
inherited from Cargo.toml when present. `description` and `license` are separately required by
crates.io; see section F.

### Two more corrections to decision 6

**crates.io is not a cargo-dist job.** `publish-jobs` accepts `"homebrew"` (since 0.2.0),
`"npm"` (since 0.14.0), and custom job paths (since 0.3.0). There is no built-in crates.io
publish. The book's own quickstart shows `cargo publish` as a separate manual step after the tag
push, and the companion guide
(https://axodotdev.github.io/cargo-dist/book/workspaces/cargo-release-guide.html) hands that job
to `cargo-release`. Decision 6 lumps crates.io in with the dist-generated release; someone has
to own that step explicitly.

**Config lives in `dist-workspace.toml` now.** Per
https://axodotdev.github.io/cargo-dist/book/workspaces/structure.html, a workspace file may be
`dist-workspace.toml`, or `Cargo.toml` under `[workspace.metadata.dist]`, the latter marked
"(deprecated in 0.23.0)". A project initialised today gets `dist-workspace.toml`.
`[profile.dist]` still lives in `Cargo.toml`, because Cargo profiles can only be declared there.

**GitLab CI is not a dist backend.** The CI page
(https://axodotdev.github.io/cargo-dist/book/ci/index.html) lists `github` as the only supported
provider, with `gitlab` under "Future CI Providers" pointing at
https://github.com/axodotdev/cargo-dist/issues/48, still open. The GitHub mirror is a
requirement of the tool, not a preference.

The installer keys the plan needs all exist:
`installers = ["shell", "npm", "homebrew"]`, `publish-jobs = ["homebrew", "npm"]`,
`tap = "owner/homebrew-tap"` (the tap repo must already exist and the release token needs write
access to it), and `npm-scope = "@scope"` if scoped.

## B. Mirroring the toolchain pin in `rust-version` is defensible, and it has a bill

**Stands, but the cost belongs in the spec text.**

### What `rust-version` actually is

Cargo Book, the manifest reference
(https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field):

> The `rust-version` field tells cargo what version of the Rust toolchain you support for your
> package.

It is a declaration consumed by tooling. It never changes what the compiler accepts; that is the
edition's job. Four things read it:

1. **Cargo's own error.** From https://doc.rust-lang.org/cargo/reference/rust-version.html, when
   the running toolchain is older than the declared version, "Cargo will report that as an error
   to the user. This makes the support expectations clear and avoids reporting a less direct
   diagnostic like invalid syntax or missing functionality in the standard library."
   `--ignore-rust-version` overrides it.
2. **The MSRV-aware resolver.** Resolver version 3 is the default for edition 2024 and sets
   `resolver.incompatible-rust-versions` to `fallback`, which makes the resolver "prefer packages
   with a Rust version that is less than or equal to your own Rust version"
   (https://doc.rust-lang.org/cargo/reference/resolver.html#rust-version). It does not hard-fail
   when no compatible version exists; it picks something and moves on.
3. **`cargo add`**, which selects the newest version requirement compatible with your
   `rust-version`.
4. **Clippy.** `clippy.toml`'s `msrv` key "Defaults to the `rust-version` field in `Cargo.toml`"
   (https://doc.rust-lang.org/clippy/configuration.html), and the `clippy::incompatible_msrv`
   lint checks "that no function newer than the defined MSRV (minimum supported rust version) is
   used in the crate."

`rust-toolchain.toml` and `rust-version` have no documented interaction at all. The rustup book's
override chapter (https://rust-lang.github.io/rustup/overrides.html) never mentions Cargo.toml.
One picks which compiler runs; the other declares which compilers you claim to support.

### The tradeoff

Setting it to 1.98.0 makes the claim true by construction: you build with 1.98.0 and you support
1.98.0. Nothing is untested. The bill is that **`cargo install gunfog` fails for anyone on any
older toolchain**, including 1.97.0 released seven weeks earlier, unless they pass
`--ignore-rust-version`. Since crates.io is a stated release channel, that is a real user-facing
consequence, not a theoretical one.

Setting it to 1.85 (the `unicode-segmentation` floor) makes a claim nothing verifies. Cargo
checks that your *dependencies'* declared versions are compatible; it never checks that your own
code compiles on 1.85. An untested MSRV is worse than an honest high one, and testing it means a
second CI toolchain job for a promise nobody asked for.

No official guidance distinguishes binaries from libraries here. RFC 2495
(https://rust-lang.github.io/rfcs/2495-min-rust-version.html) does not mention the distinction,
and the Rust API Guidelines do not discuss MSRV at all.

**Verdict: keep `rust-version = "1.98.0"` mirroring the pin, and write the `cargo install`
consequence into the spec** so it is a chosen cost rather than a surprise. Homebrew, npm and the
shell installer all ship prebuilt binaries and are unaffected; only the source-install path
through crates.io is gated.

One operational note that belongs with this: `rust-toolchain.toml`'s channel, `rust-version`, and
the CI image tag must all move together in one commit. Section E measures what happens when they
drift.

## C. The clippy invocation is right, and it is the cache-friendly one

**Stands.**

The Clippy book's CI chapter
(https://doc.rust-lang.org/clippy/continuous_integration/index.html):

> It is recommended to run Clippy on CI with `-Dwarnings`, so that Clippy lints prevent CI from
> passing. To enforce errors on warnings on all `cargo` commands not just `cargo clippy`, you can
> set the env var `RUSTFLAGS="-Dwarnings"`.

> We recommend to use Clippy from the same toolchain, that you use for compiling your crate for
> maximum compatibility.

So the planned form is the recommended one, and the toolchain-file pin satisfies the second
sentence for free.

The usage page (https://doc.rust-lang.org/clippy/usage.html) adds a detail worth knowing:

> Adding `-D warnings` will cause your build to fail if **any** warnings are found in your code.
> That includes warnings found by rustc (e.g. `dead_code`, etc.).

That is the desired behaviour, and it means the plan does not need a separate rustc-warnings gate.

### RUSTFLAGS would be worse here, measured

`RUSTFLAGS` is part of Cargo's fingerprint and applies to dependencies too, so mixing it with a
plain `cargo build` in the same target directory invalidates everything. Measured on the spike,
after a warm debug build:

| Command | Time |
| --- | --- |
| `cargo clippy --all-targets -- -D warnings` | 1.05 s (rechecks the workspace crate only) |
| `cargo build` immediately after | 0.05 s (no rebuild) |
| `RUSTFLAGS=-Dwarnings cargo clippy --all-targets` | 2.55 s (rechecks all 24 dependencies) |

The same effect is reported upstream at https://github.com/rust-lang/cargo/issues/9280. The
planned `-- -D warnings` form keeps the flag on the final crate only. Use it, and set `RUSTFLAGS`
nowhere.

### `--all-targets` does not cover doc-tests

Cargo Book, `cargo build` target selection
(https://doc.rust-lang.org/cargo/commands/cargo-build.html):

> Build all targets. This is equivalent to specifying `--lib --bins --tests --benches
> --examples`.

Doc-tests are not in that list. Confirmed by measurement: a `///` example containing
`v.iter().map(|a| a).count()`, which clippy flags anywhere else, produced
`cargo clippy --all-targets` exit code 0. The same example is compiled and run by bare
`cargo test`. See section F item 4.

### The `[lints]` table is a real option, and rejecting it is right

The table works. Measured with `[lints.clippy] all = { level = "deny", priority = -1 }`, a bare
`cargo clippy --all-targets` exits 101 with no CLI flags. `cargo build` is unaffected, because
rustc silently ignores registered tool lints. The Cargo Book documents it at
https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section, respected as of 1.74,
with `priority` as "a signed integer that controls which lints or lint groups override other lint
groups".

To get the rustc half you would add `[lints.rust] warnings = "deny"`, and measured, that makes
every local `cargo build` fail on any warning, including a stray unused variable mid-edit. That
is hostile during development and is exactly the tedium the CLI flag avoids by applying only in
CI. Decision 4 stands: no `[lints]` table.

No edition-2024-specific clippy behaviour turned up in the Clippy book.

## D. Edition 2024 is the default and there is nothing to migrate

**Stands.**

Stabilised in Rust 1.85.0 (https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/): "We are excited
to announce that the Rust 2024 Edition is now stable!". The Cargo Book's `cargo new` reference
(https://doc.rust-lang.org/cargo/commands/cargo-new.html) states "Specify the Rust edition to
use. Default is 2024."

Verified locally: `cargo new` under cargo 1.98.0 writes `edition = "2024"`, and
`cargo generate-lockfile` writes a `version = 4` lockfile.

The edition-2024 changes that are commonly called migration hazards are all about *existing*
code. For a greenfield project the honest count is zero. Two are worth a sentence because they
change the meaning of ordinary code rather than rejecting it:

- **RPIT lifetime capture.** In 2024, "all in-scope generic parameters, including lifetime
  parameters, are implicitly captured when the `use<..>` bound is not present"
  (https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html). Relevant if
  `gunfog` returns `impl Iterator<Item = ...>` from a function borrowing the source text, which
  it plausibly will. The new rule is the more permissive one.
- **Never type fallback.** Diverging expressions now fall back to `!` rather than `()`
  (https://doc.rust-lang.org/edition-guide/rust-2024/never-type-fallback.html). Can surface in
  match arms mixing a value with `panic!()` or `unreachable!()`.

The rest need code this project will not have: the `gen` keyword reservation matters only if you
name something `gen`; unsafe attributes and `unsafe_op_in_unsafe_fn` need FFI or `unsafe fn`;
the match-ergonomics restriction bites redundant explicit binding modes that new code will not
contain.

Worth noting the connection back to section B: edition 2024 implies resolver 3, which is what
turns on the MSRV-aware resolver, which is the main thing `rust-version` feeds.

## E. GitLab CI: two amendments

### The official Rust image ships no clippy and no rustfmt

**This breaks two of the four planned jobs on the first run.** Verified by running both images:

```
$ docker run --rm rust:1.98 rustup component list --installed
cargo-x86_64-unknown-linux-gnu
rust-std-x86_64-unknown-linux-gnu
rustc-x86_64-unknown-linux-gnu

$ docker run --rm rust:1.98 cargo clippy --version
error: 'cargo-clippy' is not installed for the toolchain '1.98.0-x86_64-unknown-linux-gnu'.
```

Same for `cargo fmt`, and same on `rust:1.98-slim`. The cause is in the image's own Dockerfile
template (https://raw.githubusercontent.com/rust-lang/docker-rust/master/Dockerfile-debian.template):
the install line is

```
./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION ...
```

`--profile minimal`, and no `rustup component add` anywhere in the file. The full `rust:1.98`
image is 1.65 GB and still does not have them; `rust:1.98-slim` is 905 MB. The slim image also
has no `git`, which matters if any job needs to clone or tag.

**Fix: put the components in `rust-toolchain.toml`.**

```toml
[toolchain]
channel = "1.98.0"
components = ["clippy", "rustfmt"]
```

Measured inside `rust:1.98-slim`: rustup notices the file on the first `cargo` invocation and
pulls the two components in 0.9 s. This is better than a `rustup component add` line in
`before_script` because it also provisions every contributor's laptop, and it keeps one file as
the source of truth for the toolchain.

### Pin the image tag and the toolchain version together

`rust:1.98`, `rust:1.98.0` and `rust:1.98-slim` all exist. GitLab accepts either a tag or a
digest, `image: <image-name>@<digest>`
(https://docs.gitlab.com/ci/docker/using_docker_images/), and its pipeline-security page
recommends digests generally.

For this project the digest is not the interesting control. The toolchain file already makes the
compiler version exact regardless of image drift. What matters is that the two agree. Measured
inside `rust:1.98-slim`:

| `rust-toolchain.toml` channel | First `rustc --version` |
| --- | --- |
| `1.98.0` (matches the image) | 0.9 s, downloads only clippy and rustfmt |
| `1.97.0` (does not match) | **31.5 s, downloads a full 5-component toolchain** |

Thirty seconds per job, on every job, on every push. So: use `rust:1.98-slim`, pin the toolchain
file to `1.98.0`, and bump both in the same commit. That is three files moving in lockstep with
`rust-version`; call it out in the spec so nobody bumps one alone.

Note that GitLab's own Rust template
(https://gitlab.com/gitlab-org/gitlab/-/raw/master/lib/gitlab/ci/templates/Rust.gitlab-ci.yml)
uses `image: "rust:latest"`, has no fmt or clippy job, and no cache. It is a starter, not a
model.

### Caching is a net loss here. Do not add it.

Measured on the spike, which has the real dependency set:

| | |
| --- | --- |
| Full cold pipeline (fmt, clippy, test, release build), warm registry | **9.9 s** |
| Cold `cargo fetch --locked` into an empty `CARGO_HOME` | **0.72 s** |
| Registry size after fetch | 34 MB |
| `target/` size after the full pipeline | **137-139 MB** |

The entire pipeline is ten seconds. The cache you would upload and download to save part of that
is 139 MB. On any runner this loses, and three separate factors make it worse:

1. **GitLab cannot reach `~/.cargo` without a workaround.** From
   https://docs.gitlab.com/ci/caching/: "Both artifacts and caches define their paths relative to
   the project directory, and can't link to files outside it." The registry lives at
   `/usr/local/cargo` in the official image. You would have to set
   `CARGO_HOME: $CI_PROJECT_DIR/.cargo`, which is fine but is extra machinery for a 0.72 s saving.
2. **Three runners, three separate caches.** With the docker executor the cache is stored
   "Locally, under Docker volumes" unless a distributed S3 backend is configured in the runner's
   `config.toml`. With three runners and no shared backend, the best case hit rate is one in
   three.
3. **Cargo's `target/` is not a safe thing to carry across toolchain changes**, and it is by far
   the biggest item.

Verdict: no `cache:` block. Revisit only if the dependency graph grows by an order of magnitude.

### Everything gets `--locked`

The Cargo Book (https://doc.rust-lang.org/cargo/commands/cargo-build.html) on `--locked`:

> Asserts that the exact same dependencies and versions are used as when the existing
> `Cargo.lock` file was originally generated. ... It may be used in environments where
> deterministic builds are desired, such as in CI pipelines.

Put it on `clippy`, `test` and `build`. `--frozen` and `--offline` only pay off with a cached
registry, which section E just ruled out.

### The size guard

GitLab has no native threshold mechanism. `artifacts:reports:metrics`
(https://docs.gitlab.com/ci/testing/metrics_reports.html) requires OpenMetrics format and only
displays a comparison in the merge request widget; it does not gate a pipeline. So it is a shell
check.

GitLab already fails a job on the first non-zero exit, per
https://docs.gitlab.com/ci/yaml/script/: "When script commands return an exit code other than
zero, the job fails and further commands do not execute." No `set -e` needed.

```yaml
size-guard:
  script:
    - cargo build --profile dist --locked
    - size=$(wc -c < target/dist/gunfog)
    - echo "shipped binary $size bytes (budget 2097152)"
    - test "$size" -le 2097152
```

`wc -c` over `stat -c %s`: it behaves identically on GNU, BSD and BusyBox, so the job survives a
future move to an Alpine image. And note the profile: `dist`, per section A, because that is what
users download.

### Merge request pipelines

Decision 6 says "on every push", which is branch pipelines and needs no `workflow:` block at all.
If MR pipelines are ever turned on, both fire and you get duplicates. GitLab's documented fix
(https://docs.gitlab.com/ci/yaml/workflow.html):

```yaml
workflow:
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH && $CI_OPEN_MERGE_REQUESTS
      when: never
    - if: $CI_COMMIT_BRANCH
```

Given the house rule of working on feature branches and opening MRs, adding this now costs three
lines and prevents a confusing duplicate-pipeline moment later. Worth doing.

## F. Gaps

Applying a simplicity filter: an item is listed only if its absence would cost this project
something concrete. Everything considered and rejected is named too, so the planning session
knows it was weighed.

### Would bite

**1. `repository` has to name the GitHub mirror.** Covered in section A. It is a metadata
decision with a public face, not an implementation detail, and it needs an owner's answer.

**2. crates.io requires `description` and `license`; cargo only warns.** Measured
`cargo publish --dry-run` on a manifest without them:

```
warning: manifest has no description, license, license-file, documentation, homepage or repository
```

A warning locally, a rejection at the registry. Since crates.io is a stated channel, both fields
belong in the manifest from the first commit, along with the matching `LICENSE` file(s). The
dependencies are all MIT or dual MIT/Apache-2.0, so they constrain nothing; the choice is
`gunfog`'s to make.

**3. Someone has to own the crates.io publish step.** cargo-dist does not do it (section A). The
options are a manual `cargo publish` after the tag, a custom dist publish job, or `cargo-release`.
Pick one in the spec, otherwise the first release ships to Homebrew and npm and quietly skips
crates.io.

**4. Keep `cargo test` bare.** Measured: `clippy --all-targets` does not lint doc-tests, and
`cargo test` does compile and run them. For a lib + thin bin layout the doc examples on the public
API are the only thing checking that the documented usage still compiles. Do not narrow the CI
step to `cargo test --lib`.

**5. The size guard measures the wrong binary.** Covered in section A. This is the single change
with the largest consequence in this document.

### Considered and not needed

- **cargo-deny or cargo-audit.** The dependency set is three direct crates and 26 packages in the
  lockfile, all actively maintained and among the most-downloaded on crates.io. `gunfog` reads local
  files, makes no network calls, writes no `unsafe`, and deserializes nothing but markdown text.
  The realistic exposure is build-time proc macros (`clap_derive`, `syn`, `quote`,
  `proc-macro2`), which an advisory scanner would not have caught in any historical incident this
  project resembles. Revisit if the dependency count grows materially or if `unsafe` appears.
- **A `[lints]` table.** Section C. The CLI flag does the job without making local builds fail on
  a warning.
- **`rustfmt.toml`.** Stock rustfmt is the point of stock rustfmt. Nothing in the codebase shape
  argues for a deviation.
- **An MSRV verification job.** Only needed if `rust-version` drops below the pinned toolchain.
  Section B recommends it does not.
- **cargo-nextest.** The whole test suite will run in well under a second. `cargo test` also runs
  doc-tests, which nextest does not.
- **Code coverage.** No coverage target has been stated and the scoring logic is better served by
  a table of hand-labelled fixtures, which `rust-feasibility.md` already sketches.
- **Criterion benchmarks.** Startup and throughput were already measured with `hyperfine` in
  `rust-feasibility.md`. A benchmark harness would add a dev-dependency tree larger than the
  entire runtime one.
- **Cross-platform builds on GitLab.** cargo-dist's GitHub Actions covers the target matrix.
  Duplicating it on GitLab would double the maintenance for no extra signal; Linux x86_64 catches
  essentially every bug this program can have.
- **Dependabot or Renovate.** Three direct dependencies. `cargo update` when you feel like it,
  with the lockfile committed, is proportionate.
- **A workspace.** Single package. `[workspace]` adds a layer for nothing, and cargo-dist's
  single-package path is the simplest one.

### Cheap enough to just do

- **`#![forbid(unsafe_code)]` at the top of `src/lib.rs` and `src/main.rs`.** One line each. It
  makes the "no unsafe" assumption behind the cargo-deny rejection above enforceable rather than
  aspirational.
- **A `.gitignore` with `/target`.** Obvious, and easy to forget when `cargo new` is not the thing
  that creates the repo, which here it is not.

### Not verified

Whether cargo-dist sources release notes from a `CHANGELOG.md`. It plausibly does, but nothing in
the pages read here says so, and I did not want to assert it. Check before deciding whether the
repo needs a changelog on day one.

## Sources

Measured on this machine, 2026-08-28: rustc 1.98.0, cargo 1.98.0, clippy 0.1.98, rustfmt
1.9.0-stable, Docker 29.7.2, Linux x86_64, 16 cores. Binary sizes are `wc -c` on the linked
executable. Timings are single runs of a cold-target-directory build unless stated.

cargo-dist (v0.32.0):
- Book: https://axodotdev.github.io/cargo-dist/book/workspaces/simple-guide.html,
  https://axodotdev.github.io/cargo-dist/book/quickstart/rust.html,
  https://axodotdev.github.io/cargo-dist/book/reference/config.html,
  https://axodotdev.github.io/cargo-dist/book/workspaces/structure.html,
  https://axodotdev.github.io/cargo-dist/book/ci/index.html,
  https://axodotdev.github.io/cargo-dist/book/troubleshooting.html,
  https://axodotdev.github.io/cargo-dist/book/workspaces/cargo-release-guide.html
- Source: https://github.com/axodotdev/cargo-dist/blob/main/Cargo.toml,
  https://github.com/axodotdev/cargo-dist/blob/main/.github/workflows/release.yml,
  https://github.com/axodotdev/cargo-dist/blob/main/CHANGELOG.md
- Issues: https://github.com/axodotdev/cargo-dist/issues/118 (profile defaults),
  https://github.com/axodotdev/cargo-dist/issues/48 (GitLab CI support, open)

Rust and Cargo:
- https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
- https://doc.rust-lang.org/cargo/reference/rust-version.html
- https://doc.rust-lang.org/cargo/reference/resolver.html#rust-version
- https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section
- https://doc.rust-lang.org/cargo/reference/environment-variables.html
- https://doc.rust-lang.org/cargo/commands/cargo-build.html
- https://doc.rust-lang.org/cargo/commands/cargo-new.html
- https://doc.rust-lang.org/clippy/continuous_integration/index.html
- https://doc.rust-lang.org/clippy/usage.html
- https://doc.rust-lang.org/clippy/configuration.html
- https://rust-lang.github.io/rustup/overrides.html
- https://rust-lang.github.io/rfcs/2495-min-rust-version.html
- https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/
- https://doc.rust-lang.org/edition-guide/rust-2024/index.html and the per-change pages linked in
  section D
- https://github.com/rust-lang/cargo/issues/9280 (RUSTFLAGS vs clippy CLI cache invalidation)

Docker and GitLab:
- https://hub.docker.com/_/rust
- https://raw.githubusercontent.com/rust-lang/docker-rust/master/Dockerfile-debian.template
- https://docs.gitlab.com/ci/docker/using_docker_images/
- https://docs.gitlab.com/ci/caching/
- https://docs.gitlab.com/ci/yaml/script/
- https://docs.gitlab.com/ci/yaml/workflow.html
- https://docs.gitlab.com/ci/testing/metrics_reports.html
- https://gitlab.com/gitlab-org/gitlab/-/raw/master/lib/gitlab/ci/templates/Rust.gitlab-ci.yml
