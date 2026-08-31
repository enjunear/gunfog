# Shipping a native binary on npm so `bunx gunfog` works

Research for the `gunfog` planning effort. Date: 2026-08-31.

Question: `gunfog` is a Rust CLI that a coding agent should be able to run with no install
step. `bunx gunfog` is the shape we want. cargo-dist can publish an npm package for us, but the
one it generates downloads the binary from a GitHub Release inside a `postinstall` script, and
bun does not run `postinstall` for packages it does not already trust. This note establishes
what the alternative looks like in published packages, who generates that layout, whether
cargo-dist can be made to emit it, and what bun actually does.

Companion to `rust-feasibility.md`, which picked cargo-dist and named npm as one of three
release channels, and to `rust-project-conventions.md` §A, which already found one place where
cargo-dist's generated output diverges from what this repo assumes.

Every measurement below was taken on this machine on 2026-08-31: Linux compy-5700X3D, x86_64,
16 cores, kernel 6.8.0-136-generic, node v24.14.1, npm 11.19.0, bun 1.4.0 (34cbb9a40),
cargo-dist 0.32.0, hyperfine for timings. One caveat shaped the tests. This machine's
`~/.bunfig.toml` sets `minimumReleaseAge = 604800`, so bun refuses any npm version published in
the last seven days. Package versions in the tests are pinned to older releases for that
reason, not because the newest ones behave differently.

## Verdict

| | Question | Call |
| --- | --- | --- |
| A | Does the platform-package shape work under `bunx` with zero scripts? | **Yes.** Measured: `bunx @biomejs/biome@2.5.10 --version` succeeds on an empty cache in 2.04 s. Nothing runs but the JS shim. |
| B | Is cargo-dist's npm package broken under bun? | **No, but it is worse.** bun blocks the `postinstall`; the generated `run.js` then downloads the binary on first run instead. You trade an install-time download for a run-time one, with no checksum. |
| C | Can cargo-dist emit platform packages? | **No.** 0.32.0 has exactly three npm config keys (`npm-scope`, `npm-package`, `npm-shrinkwrap`), and `optionalDependencies` appears nowhere in `book/src/` or `cargo-dist/src/`, the two trees §3 greps. The feature request has been open since 2023-09-25. |
| D | Best route to the platform-package shape | **A separate GitHub Actions workflow that consumes the release artifacts, with cargo-dist's npm installer turned off.** A custom publish job buys nothing extra and couples you to cargo-dist's template churn. |
| E | Should `gunfog` ship a separate musl package? | **Probably not.** bun 1.4.0 ignores the `libc` field and downloads both the glibc and the musl package. On biome that is a 66 MB duplicate package on disk. (Total cache is 132 MB under bun against 27 MB under npm, but the two store packages differently, so the totals do not compare directly.) Ship one statically linked musl binary as the sole `linux-x64` package instead. |
| F | Cost of the JS shim | **26.6 ms per invocation.** Measured: 3.9 ms running the biome binary directly, 30.5 ms through its node shim. For a tool an agent calls in a loop this is the real price of npm distribution. |

## 1. The pattern, from published manifests

Every manifest quoted here came from the npm registry on 2026-08-31, either
`https://registry.npmjs.org/<pkg>/<version>` or the tarball itself, not from a blog post.

### The wrapper package

`@biomejs/biome@2.5.11` is the cleanest example in the set. Manifests below are read from
2.5.11, the current release. Timings further down are on 2.5.10, because this machine's
`minimumReleaseAge` refuses anything published in the last seven days. The manifests are
identical apart from the version strings.

Its whole manifest, minus metadata:

```json
{
  "name": "@biomejs/biome",
  "version": "2.5.11",
  "bin": { "biome": "bin/biome" },
  "optionalDependencies": {
    "@biomejs/cli-win32-x64": "2.5.11",
    "@biomejs/cli-win32-arm64": "2.5.11",
    "@biomejs/cli-darwin-x64": "2.5.11",
    "@biomejs/cli-darwin-arm64": "2.5.11",
    "@biomejs/cli-linux-x64": "2.5.11",
    "@biomejs/cli-linux-arm64": "2.5.11",
    "@biomejs/cli-linux-x64-musl": "2.5.11",
    "@biomejs/cli-linux-arm64-musl": "2.5.11"
  },
  "engines": { "node": ">=14.21.3" }
}
```

No `scripts`. No `dependencies`. No `os` or `cpu` on the wrapper itself, so it installs
everywhere. Every version is pinned exactly, not with a caret. The published tarball is 91,505
bytes, and most of that is fourteen translated READMEs and a JSON schema; the machinery is one
100-line file.

`esbuild@0.28.2` is the same shape with 26 optional dependencies, a `postinstall` (see below),
and one extra field: a custom `"esbuild.binaryHashes"` map holding a sha256 for each of 23
platform binaries. npm ignores unknown top-level fields, so it survives publication and the
install script reads it back out of its own `package.json`.

`oxlint@1.80.0` and `turbo@2.10.12` are the two Rust-authored CLIs in the set that carry no
install script at all. oxlint has 19 optional dependencies named `@oxlint/binding-<triple>`;
turbo has 6 named `@turbo/<os>-<arch>`.

### The platform packages

`@biomejs/cli-linux-x64@2.5.11`:

```json
{
  "name": "@biomejs/cli-linux-x64",
  "version": "2.5.11",
  "os": ["linux"],
  "cpu": ["x64"],
  "libc": ["glibc"],
  "engines": { "node": ">=14.21.3" }
}
```

`@biomejs/cli-linux-x64-musl@2.5.11` is byte-identical except `"libc": ["musl"]`.
`@biomejs/cli-darwin-arm64` drops `libc` entirely and carries `"os": ["darwin"], "cpu":
["arm64"]`. Unpacked sizes: 68,434,386 bytes for linux-x64, 60,346,950 for darwin-arm64,
80,652,845 for win32-x64. There is no `bin` field and no `main`; the package is a directory
with one executable called `biome` (or `biome.exe`) in it.

`@esbuild/linux-x64@0.28.2` is the same minus `libc`, plus `"preferUnplugged": true`, which
tells Yarn Plug'n'Play not to zip the package (a zipped native binary cannot be executed).
Unpacked size 11,428,465 bytes.

npm's own docs define all three fields
(https://github.com/npm/cli/blob/latest/docs/lib/content/configuring-npm/package-json.md):

> **os** You can specify which operating systems your module will run on [...] The host operating
> system is determined by `process.platform`
>
> **cpu** If your code only runs on certain cpu architectures, you can specify which ones [...]
> The host architecture is determined by `process.arch`
>
> **libc** If your code only runs or builds in certain versions of libc, you can specify which
> ones. This field only applies if `os` is `linux`.

And on `optionalDependencies`:

> If a dependency can be used, but you would like npm to proceed if it cannot be found or fails to
> install, then you may put it in the `optionalDependencies` object.

That last sentence is the whole trick. The wrapper depends on all 8 (or 26) platform packages
optionally. The package manager evaluates `os`/`cpu`/`libc` against the host, silently skips
the ones that do not match, and installs the one that does. Nothing executes.

### How the shim resolves the binary

`bin/biome` from the published `@biomejs/biome@2.5.11` tarball, trimmed to the part that
does the work:

```js
#!/usr/bin/env node
const { platform, arch, env, version, release } = process;

function isMusl() {
  let stderr;
  try { stderr = execSync("ldd --version", { stdio: ['pipe','pipe','pipe'] }); }
  catch (err) { stderr = err.stderr; }
  return stderr.indexOf("musl") > -1;
}

const PLATFORMS = {
  win32: { x64: "@biomejs/cli-win32-x64/biome.exe", arm64: "@biomejs/cli-win32-arm64/biome.exe" },
  darwin: { x64: "@biomejs/cli-darwin-x64/biome", arm64: "@biomejs/cli-darwin-arm64/biome" },
  linux: { x64: "@biomejs/cli-linux-x64/biome", arm64: "@biomejs/cli-linux-arm64/biome" },
  "linux-musl": { x64: "@biomejs/cli-linux-x64-musl/biome", ... },
};

const binPath = env.BIOME_BINARY ||
  (platform === "linux" && isMusl()
    ? PLATFORMS?.["linux-musl"]?.[arch]
    : PLATFORMS?.[platform]?.[arch]);

if (binPath) {
  const result = require("child_process").spawnSync(
    require.resolve(binPath), process.argv.slice(2),
    { shell: false, stdio: "inherit", env: { ...env, BIOME_DISTRIBUTION: "npm", ... } },
  );
  if (result.error) throw result.error;
  process.exitCode = result.status;
} else {
  console.error("The Biome CLI package doesn't ship with prebuilt binaries for your platform yet. ...");
  process.exitCode = 1;
}
```

`require.resolve("@biomejs/cli-linux-x64/biome")` is the entire mechanism. Node walks up
`node_modules` from the shim's own location, finds the platform package the installer put
there, and returns an absolute path to the executable. Then `spawnSync` with `stdio: "inherit"`
runs it and forwards the exit status. An escape hatch env var (`BIOME_BINARY`,
`ESBUILD_BINARY_PATH`, `TURBO_BINARY_PATH`) short-circuits the lookup in all three tools.

Note that biome duplicates the libc detection at run time by shelling out to `ldd --version`,
even though `libc` in the platform manifests already told the installer which package to place.
That is deliberate belt and braces. The run-time check is what catches a `node_modules` copied
from a glibc image into an Alpine container.

esbuild's shim does the same job with a lookup keyed on `` `${process.platform} ${os.arch()}
${os.endianness()}` `` (so `"linux x64 LE"`), which lets it distinguish big-endian s390x from
the rest, plus a WebAssembly fallback table for three targets with no native build. It ends
with `execFileSync(binPath, process.argv.slice(2), { stdio: "inherit" })`. turbo's shim is 400
lines and adds just-in-time `npm install` recovery, x64-under-emulation fallback on arm64
macOS and Windows, Windows Ctrl-C forwarding, and a diagnostic that reads your
`package-lock.json` to detect npm issue 4828. All of that is optional; biome's 100 lines are
the minimum that works.

### Who still ships a postinstall, and what it does

Four of the seven wrappers examined still have one.

| Wrapper | `postinstall` | Works without it? |
| --- | --- | --- |
| `@biomejs/biome@2.5.11` | none | n/a |
| `oxlint@1.80.0` | none | n/a |
| `turbo@2.10.12` | none (`postversion` is a repo-side script, not an install hook) | n/a |
| `esbuild@0.28.2` | `node install.js` | **Yes**, measured on 0.28.1 |
| `@swc/core@1.16.1` | `node postinstall.js` | Yes on supported platforms |
| `dprint@0.57.0` | `node ./install.cjs` | Yes, `bin.cjs` installs lazily on first run |
| `bun@1.4.0` | `node install.js` | Not tested |

**esbuild.** Its `install.js` (300 lines, read from the published tarball) does three things, none
of which is the primary install path.

1. `maybeOptimizePackage` hardlinks the native binary over `bin/esbuild`, so that later
   invocations skip the node process entirely. A performance optimisation, wrapped in a bare
   `try {} catch {}`.
2. If `require.resolve` cannot find the platform package (someone passed `--no-optional` or
   `--omit=optional`), it falls back to `npm install`-ing the platform package into a scratch
   directory, and if that fails, to fetching the tarball straight from
   `https://registry.npmjs.org/<pkg>/-/<name>-<version>.tgz` and untarring it with 20 lines of
   hand-written tar parsing. Both paths run `binaryIntegrityCheck`, which sha256s the bytes and
   compares against the `"esbuild.binaryHashes"` map in the wrapper's own `package.json`.
3. `validateBinaryVersion` runs the binary with `--version` and throws if the string does not
   equal `packageJSON.version`.

So the postinstall is a repair kit and a self-test, not the delivery mechanism. Measured
directly:

```
$ npm install --ignore-scripts        # esbuild@0.28.1
added 2 packages in 1s
$ ./node_modules/.bin/esbuild --version
0.28.1
```

`bin/esbuild` was still an ASCII node script afterwards, so the hardlink optimisation had not
run, and the tool worked anyway.

**@swc/core.** Its `postinstall.js` opens with its own docstring: "It checks if corresponding
optional dependencies for native binary is installed and can be loaded properly. If it fails,
it'll internally try to install `@swc/wasm` as fallback." Again a fallback, not the path.

**dprint.** Its `install.cjs` copies the executable out of the platform package into the wrapper
directory. Its `bin.cjs` covers the case where that never happened:

```js
const exePath = path.join(__dirname, os.platform() === "win32" ? "dprint.exe" : "dprint");
if (!fs.existsSync(exePath)) {
  const resolvedExePath = require("./install_api.cjs").runInstall();
  runDprintExe(resolvedExePath);
} else {
  runDprintExe(exePath);
}
```

Same lazy-repair idea cargo-dist uses, layered on top of a platform-package layout.

The conclusion for `gunfog` is that a `postinstall` is never required by this pattern. Every
tool that has one uses it for an optimisation or a repair path, and every one of them still
works when it is blocked.

## 2. Who generates the layout

There is one general-purpose generator, and it is aimed at something slightly different from
what `gunfog` needs. Everyone else hand-rolls a script of 100 to 200 lines.

### napi-rs is the only real tooling, and it builds addons, not CLIs

`@napi-rs/cli@3.8.6` has commands `create-npm-dirs`, `artifacts`, `pre-publish` and `version`
that together own the whole layout. `cli/src/api/create-npm-dirs.ts` writes each platform
manifest (https://github.com/napi-rs/napi-rs/blob/main/cli/src/api/create-npm-dirs.ts):

```ts
const scopedPackageJson: CommonPackageJsonFields = {
  name: `${packageName}-${target.platformArchABI}`,
  version: packageJson.version,
  cpu: target.arch !== 'universal' && target.arch !== 'wasm32' ? [target.arch] : undefined,
  main: binaryFileName,
  files: [binaryFileName],
  ...pick(packageJson, 'description', 'keywords', 'author', /* ... */),
}
// ...
if (target.abi === 'gnu')       { scopedPackageJson.libc = ['glibc'] }
else if (target.abi === 'musl') { scopedPackageJson.libc = ['musl'] }
```

and `cli/src/api/pre-publish.ts` has a `resolveRootOptionalDependencies` that rebuilds the
wrapper's `optionalDependencies` map from the target list, then publishes the lot. `@swc/core`
drives it directly: its `package.json` `scripts` still contain `"version": "napi version
--npm-dir scripts/npm"` and `"artifacts": "napi artifacts --npm-dir scripts/npm"`. oxlint's
`@oxlint/binding-<triple>` naming is napi-rs's convention too.

The catch: napi-rs packages N-API addons. Its platform manifests set `main` to a
`<name>.<triple>.node` file that the wrapper `require()`s in-process.
`@oxlint/binding-linux-x64-musl` has `"main": "oxlint.linux-x64-musl.node"` and no `bin`. For
`gunfog`, a plain Rust binary, using napi-rs would mean writing an N-API shim crate and linking
the whole tool into a Node addon. That is a much larger change than copying biome's 150-line
script, and it drags in a `cdylib` build and Node ABI concerns for no benefit.

### esbuild hand-rolls it, with the platform manifests checked into the repo

`npm/@esbuild/<triple>/package.json` is a committed file for each of 27 targets. `npm/@esbuild/linux-x64/package.json`
in the repo is exactly what the registry serves. The `Makefile` has a
target per platform that cross-compiles the Go binary straight into that directory:

```make
platform-win32-x64: version-go go-compiler
	@$(MAKE) --no-print-directory GOOS=windows GOARCH=amd64 NPMDIR=npm/@esbuild/win32-x64 BINPATH=esbuild.exe platform-internal
```

and `scripts/esbuild.js` regenerates the two derived fields on the wrapper:

```js
packageJSON.optionalDependencies = optionalDependencies
packageJSON['esbuild.binaryHashes'] = generateBinaryHashes()
```

`optionalDependencies` is derived by bundling and evaluating `lib/npm/node-platform.ts`, so the
shim's lookup tables and the dependency list cannot drift apart. That is the one idea worth
stealing outright.

### biome hand-rolls it in 150 lines of Node

`packages/@biomejs/biome/scripts/generate-packages.mjs`
(https://github.com/biomejs/biome/blob/main/packages/@biomejs/biome/scripts/generate-packages.mjs)
is the whole generator:

```js
const manifest = JSON.stringify({
  name: packageName, version, license, repository, engines, homepage,
  os: [os],
  cpu: [arch],
  libc: os === "linux" ? (packageName.endsWith("musl") ? ["musl"] : ["glibc"]) : undefined,
}, null, 2);
fs.writeFileSync(manifestPath, manifest);
// then copy the binary in and chmod 0o755
fs.copyFileSync(binarySource, binaryTarget);
fs.chmodSync(binaryTarget, 0o755);
```

with `const PLATFORMS = ["win32-%s", "darwin-%s", "linux-%s", "linux-%s-musl"]` and
`ARCHITECTURES = ["x64", "arm64"]` at the bottom. A companion `updateVersionInDependencies`
walks the wrapper's `optionalDependencies` and pins every `@biomejs/*` entry to the release
version. This is the model `gunfog` should copy. It is small enough to read in one sitting, and it
does nothing that a Rust project cannot do the same way.

### Things that turned out not to be this

- **`cargo-xwin` and `cargo-zigbuild`** are cross-compilation linkers.
  `rust-cross/cargo-zigbuild` describes itself as "Compile Cargo project with zig as linker" and
  `rust-cross/cargo-xwin` as "Cross compile Cargo project to Windows MSVC target with ease".
  Neither README mentions npm. They solve the "get a darwin-arm64 binary" half of the problem,
  which cargo-dist already solves for us with its own runner matrix.
- **`optic`.** No such generator found. `optic` on npm is an abandoned browser computer-vision
  library at 0.1.0; `@useoptic/optic` is a TypeScript API-diffing CLI with 80-odd ordinary
  dependencies and no platform packages. A web search for a tool by that name in this space
  returned nothing relevant.
- **`npm-binary-distributions`.** No such project. A GitHub repository search for the phrase
  returns only unrelated single-tool repos ("Binary distribution of `earthly` for npm" and
  similar), each of which hand-rolls its own download script.
- **`binary-install`** (npm, v1.1.2) is real but is the *other* pattern: a library that downloads
  a tarball from a URL during `postinstall`, depending on `axios`, `rimraf` and `tar`. It is the
  ancestor of what cargo-dist generates. Older cargo-dist output still carries its dependency
  set: `@axodotdev/axolotlsay@0.3.3` lists `axios`, `axios-proxy-builder`, `console.table`,
  `detect-libc` and `rimraf`, and pulls 60 packages into `node_modules`. 0.32.0 has since
  rewritten the file to use `node:https` and dropped the dependencies down to `detect-libc`
  alone. (That the 0.32.0 template descends from the `binary-install` package is an inference
  from the filename `binary-install.js` and the identical API; the file carries no attribution
  header.)

## 3. cargo-dist

### What 0.32.0 actually generates

Read from the 0.32.0 source tarball, not the docs. The wrapper manifest template is
`cargo-dist/templates/installer/package.json`:

```json
{
  "name": "axonpminstaller",
  "version": "0.0.0",
  "preferUnplugged": true,
  "artifactDownloadUrls": "",
  "glibcMinimum": { "major": 2, "series": 31 },
  "supportedPlatforms": {},
  "scripts": { "postinstall": "node ./install.js", "fmt": "...", "fmt:check": "..." },
  "engines": { "node": ">=14.14", "npm": ">=6" },
  "dependencies": { "detect-libc": "^2.1.2" },
  "devDependencies": { "prettier": "^3.8.3" }
}
```

One package. `install.js` is two lines that call `install(false)` in `binary.js`, which reads
`artifactDownloadUrls`, `supportedPlatforms` and `glibcMinimum` back out of the manifest,
builds a Rust target triple from `os.type()`, `os.arch()` and `detect-libc`, and downloads
`${artifactDownloadUrl}/${platform.artifactName}` into `node_modules/.bin_real` inside its own
package directory. There is no checksum anywhere in `binary-install.js`; a grep for `sha`,
`checksum`, `integrity` and `hash` returns nothing. Compare esbuild, which verifies a pinned
sha256 even on its fallback path.

The generated `run-<bin>.js` is what `bin` points at, and it repairs a skipped postinstall:

```js
run(binaryName) {
  const promise = !this.exists() ? this.install(true) : Promise.resolve();
  promise.then(() => { /* spawnSync the binary, forward exit status */ })
```

So the package is not *broken* when scripts are blocked. It just moves the download to the
first invocation, silently (`suppressLogs = true`), over the network, with no integrity check.

### There is no config key for a platform-package layout

`cargo-dist/src/config/v1/installers/npm.rs` defines every npm installer config key:

```rust
pub struct NpmInstallerLayer {
    pub common: CommonInstallerLayer,
    pub package: Option<String>,     // npm-package
    pub scope: Option<String>,       // npm-scope
    pub shrinkwrap: Option<bool>,    // npm-shrinkwrap
}
```

`grep '^#### `npm' book/src/reference/config.md` returns exactly three keys: `npm-scope`,
`npm-package`, `npm-shrinkwrap`. A case-insensitive grep for `optionaldependencies` or
`platform package` across `book/src/` and `cargo-dist/src/` returns nothing at all.

The maintainers know the pattern exists. Issue 450, "unlock installer strategies", opened
2023-09-25 and never updated since, tabulates fetching against bundling per installer and marks
npm+bundling as "possible", with this note:

> npm packages that bundle binaries could make sense, I think people do it, although they use a
> complicated system where they have one package for each platform, and then a meta-package that
> depends on them all and picks the right one at install-time

The repo is alive (last push 2026-08-28, 330 open issues) but 0.32.0 was released 2026-05-22
and is still the latest. Nothing suggests this is coming.

### Option A: a custom cargo-dist publish job

`publish-jobs` accepts a path to a reusable workflow. From `book/src/ci/customizing.md`:

> To add a custom job, you need to follow two steps:
> 1. Define the new job as a reusable workflow using the standard method defined by your CI
>    system. For GitHub actions, see the documentation on reusable workflows.
> 2. Add the name of your new workflow file to the appropriate array in your dist config,
>    prefixed with a `./`. For example, if your job name is `.github/workflows/my-publish.yml`,
>    you would write it like this:
>
> ```toml
> publish-jobs = ["./my-publish"]
> ```

The contract, read off `cargo-dist/templates/ci/github/release.yml.j2` rather than the prose:

```yaml
  custom-<name>:
    needs:
      - plan
      - host
      # plus any custom host-jobs
    if: ${{ !fromJson(needs.plan.outputs.val).announcement_is_prerelease
            || fromJson(needs.plan.outputs.val).publish_prereleases }}
    uses: ./.github/workflows/<name>.yml
    with:
      plan: ${{ needs.plan.outputs.val }}
    secrets: inherit
```

- **Inputs.** Exactly one, `plan`, a JSON string. It is the same document as the
  `dist-manifest.json` shipped with the release. Your workflow declares
  `on: workflow_call: inputs: plan: { required: true, type: string }`.
- **Secrets.** `secrets: inherit`, so `NPM_TOKEN` is reachable without extra wiring.
- **Permissions.** None by default. `[dist.github-custom-job-permissions]` grants them, and the
  config reference warns: "If you override a publish job's permissions, the default permissions
  will be removed."
- **Artifacts.** Not passed in, but downloadable two ways. The built-in `publish-npm` job pulls
  them out of the Actions artifact store with `actions/download-artifact`, `pattern: artifacts-*`,
  `merge-multiple: true`; a custom job can do the same. And because `github-release` defaults to
  `host` ("By default, the GitHub Release is created during the 'host' phase, as it hosts the
  files some installers will try to download"), the GitHub Release is already live by the time a
  publish job runs, so `gh release download` also works.
- **When.** After `host`, before `announce`. `announce` waits on every custom publish job and
  treats `skipped` as acceptable but not `failure`.

Cost: `dist generate` writes the wiring into `.github/workflows/v-release.yml`, so `dist
generate --check` stays green and the branch stays clean. You still write the packaging script
yourself; cargo-dist contributes the `needs`, the `if` and the plan JSON, and nothing else. On
upgrade: cargo-dist regenerates `v-release.yml`, which can reshape the `needs` graph or the `if`
guard around your job. Your reusable workflow file is untouched, but its call site is
generated, so a template change lands in your repo as a diff you must accept.

### Option B: a separate workflow outside cargo-dist

Turn cargo-dist's npm installer off entirely (drop `"npm"` from `installers` and
`publish-jobs`) and add `.github/workflows/publish-npm.yml` triggered on `release: { types:
[published] }` or `workflow_run`. It downloads the release assets with `gh release download`,
unpacks each archive, writes the platform manifests and the wrapper manifest with a biome-style
script, and runs `npm publish` once per package plus once for the wrapper.

Cost: you own the trigger, the ordering and the failure mode. A failed npm publish no longer
fails the release, which is both the upside and the downside. Nothing in this file is
generated, so `dist generate --check` never touches it and a cargo-dist upgrade cannot rewrite
it. The only coupling left is the artifact naming convention, which you read out of
`dist-manifest.json` anyway.

### Option C: drop cargo-dist for npm and use something else

There is nothing to move to. napi-rs is the only generator and it wants an N-API addon (§2).
Everything else in the survey is a per-project script. So "another tool" collapses into option
B with extra steps.

### The three side by side

| | Buys | Costs | On cargo-dist upgrade |
| --- | --- | --- | --- |
| A. Custom publish job | Release fails as one unit if npm publish fails; secrets already inherited; ordering handled | Your job's call site is generated; you still write all the packaging logic | `v-release.yml` is regenerated; the `needs`/`if` around your job can change and shows up as a diff |
| B. Separate workflow | Zero coupling; `dist generate --check` never sees it; independently re-runnable | npm publish failures do not fail the release; you wire the trigger and the token yourself | Nothing. Only the artifact names matter, and those come from `dist-manifest.json` |
| C. Another tool | Nothing | napi-rs needs an N-API addon; no other generator exists | n/a |

Option B is the call. The coupling option A buys is small (cargo-dist orders the job for you)
and the coupling it costs is exactly the kind this repo has already been bitten by once, in
`rust-project-conventions.md` §A, where a regenerated block silently overrode a setting.

## 4. bun

### Lifecycle scripts are blocked by default

Bun's primary documentation (https://bun.com/docs/install/lifecycle):

> Because running arbitrary code is a security risk, Bun does not execute arbitrary lifecycle
> scripts by default, unlike other npm clients.

and

> Defining `trustedDependencies` in `package.json` **replaces** the default list rather than
> extending it.

and, on sources:

> For packages from other sources (such as `file:`, `link:`, `git:`, or `github:` dependencies),
> you must explicitly add them to `trustedDependencies`.

Measured, with `@axodotdev/axolotlsay@0.3.3` (a real cargo-dist-generated npm package) as a
dependency and no `trustedDependencies` field:

```
$ bun install
+ @axodotdev/axolotlsay@0.3.3
72 packages installed [387.00ms]

$ ls node_modules/@axodotdev/axolotlsay/
binary-install.js  binary.js  install.js  npm-shrinkwrap.json  package.json  run-axolotlsay.js  ...

$ bun pm untrusted
./node_modules/@axodotdev/axolotlsay @0.3.3
 » [postinstall]: node ./install.js

These dependencies had their lifecycle scripts blocked during install.
```

No binary was downloaded. The tool is installed but inert until first run, when
`run-axolotlsay.js` fetches it.

### Bun does keep a default-trusted list

The docs point at
`https://github.com/oven-sh/bun/blob/main/src/install/default-trusted-dependencies.txt`, and
`bun pm default-trusted` prints it. On bun 1.4.0 that is **367 packages**. Checked against the
tools in this note:

| Package | On the default-trusted list? |
| --- | --- |
| `esbuild` | yes |
| `dprint` | yes |
| `@biomejs/biome` | no (it has no scripts, so it does not need to be) |
| `turbo` | no |
| `@swc/core` | no |
| `oxlint` | no |
| `detect-libc` | no |

A new package like `gunfog` is not on that list and has no route onto it in time for a release.
And since `trustedDependencies` replaces rather than extends the default list, a consumer who
adds `"trustedDependencies": ["gunfog"]` to unblock us silently disarms the trust bun grants to
everything else in their tree. Asking users to do that is a bad trade.

### bunx

Bun's `bunx` documentation (https://bun.com/docs/cli/bunx) says only:

> As with `npx`, `bunx` checks for a locally installed package first, then falls back to
> auto-installing it from `npm`. `bunx` stores installed packages in Bun's global cache for
> future use.

**It says nothing about lifecycle scripts.** Bun's own test suite shows `bunx` goes through the
same install machinery, and therefore the same trust rules: `test/cli/install/bunx.test.ts` has
a case named "should handle postinstall scripts correctly with symlinked bunx" that runs `bunx
esbuild@latest --version`, and esbuild is on the default-trusted list. The consequence for a
package that is *not* on that list is worse than for `bun install`. A `bunx` invocation has no
project `package.json` to put `trustedDependencies` in, so there is no way for the user to
grant trust at all. Only the built-in list can.

The first evidence that `bunx` skips an untrusted `postinstall` was indirect. `bunx
@axodotdev/axolotlsay@0.3.3 hi` succeeded with **empty stderr**, and that package's
`postinstall` calls `install(false)`, which prints `Downloading release from ...` to stderr,
while its `run.js` calls `install(true)`, which suppresses exactly that message. The binary
arrived silently, so the run path fetched it and the postinstall did not.

**The clean positive control, added after the first draft.** Two packages that carry a
`postinstall` and *no* lazy run-time repair path both fail under `bunx`, which is the control
this note originally could not build:

| Package | `postinstall` | `bunx` result |
| --- | --- | --- |
| `@vscode/ripgrep@1.15.14` | `node ./lib/postinstall.js`, downloads `rg` | `error: could not determine executable to run`; the `bin/` the script creates never appears |
| `fd-find@1.2.0` | `node download.js`, overwrites a 51-byte stub with a 3.6 MB ELF | exit 1; `dist/fd` is still the ASCII stub |

Under `npx` both work. So `bunx` really does skip the script, and cargo-dist's packages survive
only because `binary-install.js:321` reinstalls on the run path when the binary is missing.

**What a cold cargo-dist package costs under `bunx`.** With `/tmp/bunx-1000-@axodotdev` and the
matching `~/.bun/install/cache` entry deleted, `bunx @axodotdev/axolotlsay@0.3.3 "truly cold"`
took **2.185 s** and left the native binary at
`node_modules/@axodotdev/axolotlsay/node_modules/.bin_real/axolotlsay`. Warm runs are 0.163 s.
That is within noise of the 2.04 s measured for `@biomejs/biome` in the platform-package shape,
so the run-time download is not what separates the two layouts. The missing checksum is.

### Measured under bun and npm

`bunx` with the platform-package shape, empty cache, `BUN_INSTALL_CACHE_DIR` pointed at a fresh
directory:

```
$ bunx @biomejs/biome@2.5.10 --version
Resolving dependencies
Resolved, downloaded and extracted [22]
Version: 2.5.10
2.04 s     (cold)
0.10 s     (warm, same cache)
```

Zero scripts ran, and this is the whole install: no `node-gyp`, no network fetch outside the
registry, no GitHub dependency.

**bun ignores `libc`.** Installing `@biomejs/biome@2.5.10` as a dependency, on this glibc
machine:

| | Packages installed | `node_modules/@biomejs` | Cache after install |
| --- | --- | --- | --- |
| npm 11.19.0 | 2 | `biome`, `cli-linux-x64` | 27 MB |
| bun 1.4.0 | 3 | `biome`, `cli-linux-x64`, **`cli-linux-x64-musl`** | 132 MB |

bun filtered on `os` and `cpu` (no darwin or win32 package was fetched) but not on `libc`, so
it downloaded both linux-x64 variants. For biome that is 66 MB of waste. For a 2 MiB `gunfog`
binary it is about 2 MiB of waste, which is survivable, but the cleaner answer is to publish
one statically linked musl binary as the sole `linux-x64` package and omit `libc` entirely. A
static musl build runs on glibc hosts, so one package covers both and there is nothing for bun
to get wrong.

**The shim costs 26.6 ms.** hyperfine, 20 runs, 3 warmups, on the installed biome 2.5.10:

| Invocation | Mean |
| --- | --- |
| `@biomejs/cli-linux-x64/biome --version` (the binary itself) | 3.9 ms ± 0.2 |
| `node .../@biomejs/biome/bin/biome --version` (through the shim) | 30.5 ms ± 1.6 |
| `bun .../@biomejs/biome/bin/biome --version` (shim under bun's runtime) | 83.4 ms ± 2.4 |
| `bunx @biomejs/biome@2.5.10 --version` (warm cache, end to end) | 103.3 ms ± 2.6 |

Running the shim under bun's runtime instead of node is 2.7x slower, so `bunx --bun gunfog` is
the wrong advice to give anyone. The 26.6 ms node overhead is unavoidable for any
npm-distributed native CLI: `bin` entries must live inside the package, so something has to
resolve across the package boundary. Worth stating in the README that the npm build adds about
27 ms per invocation over the same binary installed by `cargo install` or the shell installer.

## What this means for `gunfog`

None of this ships in v0.1.0. `SPEC.md` defers the platform-package shape and v0.1.0 publishes
cargo-dist's npm package as generated; this is the target the deferral points at, tracked as
https://git.enjunear.com/enjunear/gunfog/-/work_items/25.

1. Publish a wrapper `gunfog` with no `scripts`, no `dependencies`, a `bin` pointing at a
   100-line shim modelled on biome's, and `optionalDependencies` pinning every platform package
   to the exact release version.
2. Publish one platform package per target, each with `os`, `cpu`, `preferUnplugged: true`, no
   `bin`, no `main`, and the executable at the package root. Skip `libc`: ship a static musl
   binary as the single `linux-x64` package.
3. Generate all of it from one script in the release workflow, deriving `optionalDependencies`
   from the same target list the shim's lookup table is built from, the way esbuild does.
4. Turn cargo-dist's npm installer off. Keep it for the shell installer, Homebrew and the GitHub
   Release; publish npm from a separate workflow triggered on the release.
5. Do not ask users for `trustedDependencies`. It replaces bun's default list and makes their
   tree less safe, not more.

## Sources

All fetched or run 2026-08-31.

Published manifests and tarballs (npm registry, primary):
- `https://registry.npmjs.org/@biomejs/biome`, `.../@biomejs/biome/-/biome-2.5.11.tgz`
- `https://registry.npmjs.org/@biomejs/cli-linux-x64/2.5.11`, `.../cli-linux-x64-musl/2.5.11`,
  `.../cli-darwin-arm64/2.5.11`, `.../cli-win32-x64/2.5.11`
- `https://registry.npmjs.org/esbuild`, `.../esbuild/-/esbuild-0.28.2.tgz`
- `https://registry.npmjs.org/@esbuild/linux-x64/0.28.2`, `.../darwin-arm64/0.28.2`,
  `.../win32-x64/0.28.2`
- `https://registry.npmjs.org/@swc/core/latest`, `.../@swc/core/-/core-1.16.1.tgz`,
  `.../@swc/core-linux-x64-musl/latest`
- `https://registry.npmjs.org/oxlint/latest`, `.../oxlint/-/oxlint-1.80.0.tgz`,
  `.../@oxlint/binding-linux-x64-musl/latest`
- `https://registry.npmjs.org/turbo/latest`, `.../turbo/-/turbo-2.10.12.tgz`,
  `.../@turbo/linux-64/latest`
- `https://registry.npmjs.org/dprint/latest`, `.../dprint/-/dprint-0.57.0.tgz`,
  `.../@dprint/linux-x64-musl/latest`
- `https://registry.npmjs.org/bun/latest`, `.../@oven/bun-linux-x64/latest`
- `https://registry.npmjs.org/@axodotdev/axolotlsay/latest`, `.../@axodotdev/oranda/latest`
- `https://registry.npmjs.org/binary-install/latest`, `.../@napi-rs/cli/latest`,
  `.../@useoptic/optic/latest`, `.../optic/latest`

Release scripts (repository source, primary):
- https://github.com/biomejs/biome/blob/main/packages/@biomejs/biome/scripts/generate-packages.mjs
- https://github.com/evanw/esbuild/blob/main/scripts/esbuild.js
- https://github.com/evanw/esbuild/blob/main/Makefile
- https://github.com/evanw/esbuild/blob/main/npm/@esbuild/linux-x64/package.json
- https://github.com/napi-rs/napi-rs/blob/main/cli/src/api/create-npm-dirs.ts
- https://github.com/napi-rs/napi-rs/blob/main/cli/src/api/pre-publish.ts

cargo-dist 0.32.0 (source tarball
`https://github.com/axodotdev/cargo-dist/archive/refs/tags/v0.32.0.tar.gz`, primary):
- `cargo-dist/templates/installer/package.json`
- `cargo-dist/templates/installer/npm/{install.js,binary.js,binary-install.js,run.js.j2}`
- `cargo-dist/templates/installer/npm-shrinkwrap.json`
- `cargo-dist/templates/ci/github/release.yml.j2`
- `cargo-dist/templates/ci/github/partials/publish_npm.yml.j2`
- `cargo-dist/src/config/v1/installers/npm.rs`
- `cargo-dist/src/backend/installer/npm.rs`
- `cargo-dist/src/backend/ci/github.rs`
- `book/src/installers/npm.md`, `book/src/ci/customizing.md`, `book/src/reference/config.md`
- https://github.com/axodotdev/cargo-dist/issues/450 (installer strategies, open since
  2023-09-25)

npm and bun documentation:
- https://github.com/npm/cli/blob/latest/docs/lib/content/configuring-npm/package-json.md
  (`os`, `cpu`, `libc`, `optionalDependencies`, `bin`)
- https://docs.npmjs.com/cli/v11/configuring-npm/package-json
- https://bun.com/docs/install/lifecycle
- https://bun.com/docs/cli/bunx and
  https://github.com/oven-sh/bun/blob/main/docs/pm/bunx.mdx
- https://github.com/oven-sh/bun/blob/main/src/install/default-trusted-dependencies.txt
- https://github.com/oven-sh/bun/blob/main/test/cli/install/bunx.test.ts
- https://github.com/oven-sh/bun/blob/main/src/runtime/cli/bunx_command.rs

Cross-compilation, checked and ruled out:
- https://github.com/rust-cross/cargo-zigbuild
- https://github.com/rust-cross/cargo-xwin

Claims resting on something other than a primary source, stated as such in the body:
- That cargo-dist's `binary-install.js` descends from the `binary-install` npm package is an
  inference from the filename and the matching API. The file carries no attribution header.
- That `bunx` never runs an untrusted `postinstall` rests on bun's install-path documentation,
  its test suite, and the stderr evidence above. Bun does not document `bunx` script behaviour
  either way. The `@vscode/ripgrep` and `fd-find` runs added after the first draft are a direct
  control and agree, so this is now measurement rather than inference, but it is still not a
  documented guarantee and a future bun release could change it.
- Whether `bun@1.4.0`'s own `postinstall` is required was not tested.
