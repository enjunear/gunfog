# Cutting a release

Releases originate from the GitHub mirror, not from GitLab.
`docs/adr/0001-github-release-origin.md` says why. This is the runbook.

## One-time setup

None of this is in the repo, and all of it blocks the first release.

- [ ] Create the public mirror `github.com/enjunear/gunfog` and configure push mirroring from
      `git.enjunear.com/enjunear/gunfog`, including tags.
- [ ] Create the tap repo `github.com/enjunear/homebrew-tap`.
- [ ] Mint a repo-scoped PAT with write access to the tap and add it to the mirror as the
      `HOMEBREW_TAP_TOKEN` Actions secret.
- [ ] Claim the unscoped npm name `gunfog` and add a token to the mirror as the `NPM_TOKEN`
      Actions secret. Use a granular token with write access to the `gunfog` package and
      nothing else, not a classic automation token, which can publish anything you own.
- [ ] Nominate the crates.io owner who runs `cargo publish`, and have them `cargo login`.

## Every release

1. Make sure the GitLab pipeline is green on the commit you are about to tag. The generated
   GitHub workflow runs no tests, so nothing downstream checks this for you.
2. Bump `version` in `Cargo.toml`, run `cargo build` so `Cargo.lock` follows, and commit both.
3. Tag with a `v`-prefixed plain semver tag and push it. Push mirroring carries the tag to GitHub,
   where the tag triggers the release workflow. A tag with a prerelease suffix, `v0.2.0-rc.1`,
   also matches the glob and publishes as a GitHub prerelease. The project runs no prerelease
   channel, so do not push one.
   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```
4. Watch the run at `github.com/enjunear/gunfog/actions`. It builds the five target triples,
   uploads them to a GitHub Release, pushes the Homebrew formula to the tap, and publishes to npm.
   There is no `CHANGELOG.md`, so the release body is whatever dist generates from the tag.
5. Publish to crates.io by hand. dist has no crates.io job, so nothing above does this:
   ```sh
   cargo publish --locked
   ```

## Changing the release config

`dist-workspace.toml` is the config, and `.github/workflows/v-release.yml` is generated from it.
Both are checked in and have to agree. The `v-` in the filename comes from `tag-namespace = "v"`,
which is also what puts `v` in the workflow's tag glob. Change that setting and dist writes a
differently named file, leaving the old one behind for you to delete. After editing the config,
regenerate and commit the workflow in the same change:

```sh
dist generate
```

To check that the committed workflow is still the one the config produces, without writing
anything:

```sh
dist generate --check
```

It exits 0 and prints nothing when they agree. The GitLab pipeline does not run this. Its four
jobs finish in about ten seconds on a `rust:slim` image that ships no dist, and downloading dist
to guard a file that only changes when someone edits the config alongside it is not worth
doubling that.

Install the pinned version, matching `cargo-dist-version` in `dist-workspace.toml`, with:

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/axodotdev/cargo-dist/releases/download/v0.32.0/cargo-dist-installer.sh | sh
```

Two things to check after any regeneration, both measured in
`docs/research/rust-project-conventions.md` §A:

- `[profile.dist]` in `Cargo.toml` still contains `inherits = "release"` and nothing else. `dist
  init` writes `lto = "thin"` into that block, which overrides the inherited `lto = true` and adds
  about 124 KB to the shipped binary.
- `cargo build --release` and `cargo build --profile dist` produce byte-identical binaries. Both
  were 952,920 bytes on 2026-08-30, on Linux 6.8 with the pinned 1.98.0 toolchain. The
  `dist-size` job in `.gitlab-ci.yml` measures the dist profile for the same reason. It will not
  catch this on its own. Its 2 MiB budget has over a megabyte of slack, so the 124 KB
  that `lto = "thin"` adds passes it silently.
