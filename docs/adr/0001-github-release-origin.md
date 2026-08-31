# GitHub is the release origin, GitLab stays the development home

Development lives on the self-hosted GitLab at `git.enjunear.com/enjunear/gunfog`, and that does
not change. Releases originate from a public GitHub mirror at `github.com/enjunear/gunfog`,
because cargo-dist can only generate GitHub Actions workflows and can only upload to GitHub
Releases. Its CI reference lists `github` as the sole backend, and the GitLab backend is upstream
issue https://github.com/axodotdev/cargo-dist/issues/48, still open. The mirror is therefore a
requirement of the tool, not a preference.

## Considered options

**Hand-write a GitLab release pipeline.** GitLab can cross-compile the five target triples,
but every artifact cargo-dist generates for free would become ours to write and maintain: the
shell installer, the Homebrew formula, the npm package, the checksums, the release notes. That
is the work cargo-dist exists to avoid.

**Wait for upstream issue #48.** Opened 2023-01-21, last touched 2024-02-29, still
unimplemented. Blocking v0.1.0 on it is blocking on nothing.

**Mirror to GitHub and release from there.** Chosen. One `git push` mirror configuration buys the
whole generated pipeline.

## Consequences

`repository` in `Cargo.toml` names the GitHub mirror, not the GitLab origin. cargo-dist reads
that field to build the download URLs its installers fetch from, so it has to point at the host
serving the releases. That URL is also what crates.io, Homebrew and npm advertise, so the mirror
is the public face of the project and the GitLab instance stays private.

Pushing a `v`-prefixed plain semver tag to the mirror cuts a release. The GitLab CI runs the
four check jobs on every push and knows nothing about releases; the GitHub workflow builds and
publishes and runs no tests. Neither side gates the other, so a tag pushed to a commit that fails
GitLab CI will still release. The runbook in `docs/releasing.md` puts a green pipeline first for
that reason.

crates.io is not part of the generated pipeline. cargo-dist's `publish-jobs` accepts only
`homebrew`, `npm` and custom jobs, so `cargo publish` is a manual step run by a named owner.

The npm package cargo-dist generates is a single package with a `postinstall` script that
downloads the binary from the GitHub Release. cargo-dist has no config key for the biome-style
platform packages `SPEC.md` originally asked for, and no way to add a checksum to that download.

`bunx gunfog` still works, which is what the spec was after. bun skips the `postinstall` for any
package outside its default-trusted list, but cargo-dist's `binary-install.js` reinstalls on the
run path when the binary is missing, so the download moves to first invocation rather than being
lost. Measured at 2.185 s cold against 2.04 s for a platform-package layout, so the layouts cost
about the same. That 2.185 s is an upper bound. It was measured on `@axodotdev/axolotlsay@0.3.3`,
built by an older cargo-dist that pulls 60 packages through axios, where 0.32.0 ships
`detect-libc` alone. What differs between the layouts is that nothing verifies the downloaded
binary. cargo-dist publishes a `.sha256` beside every archive and its npm installer checks none
of them.

That is the reason to move to platform packages later, and it is a better reason than the one the
spec gave. `docs/research/npm-platform-packages.md` has the measurements and the three routes.
