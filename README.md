# gunfog

`gunfog` is a CLI that scores prose with the Gunning fog index and reports hotspots so a coding agent can revise its own writing. Token-efficient output is the prime directive: every printed character must earn its place in an agent's context window.

## Install

Nothing is published yet. The channels below go live with v0.1.0.

Homebrew:

```sh
brew install enjunear/tap/gunfog
```

Shell installer:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/enjunear/gunfog/releases/latest/download/gunfog-installer.sh | sh
```

npm, for a one-off run with nothing to install first:

```sh
bunx gunfog
```

Or globally:

```sh
npm install -g gunfog
```

crates.io:

```sh
cargo install gunfog
```

## Usage

```
$ gunfog --file CONTEXT.md
fog: 10.7 (target 10)
+0.42 41w L14 "Within a token a placeholder is a boundary…" complex: placeholder, boundary, remainder…
+0.50 54w L33 "Excerpt The quoted snippet that identifies a hotspot…" complex: identifies, placeholder, space-separated…
```

Each hotspot line names one sentence, what it costs the score, and the complex words driving it. A document at or under target prints the score line and nothing else. Run `gunfog --help` for the flags, and read [SPEC.md](SPEC.md) for the output format in full.

Exit codes follow grep and diff. 0 when the document scores at or under target, 1 when it scores over target or is too short to score, 2 when gunfog could not score at all. The first output line tells the two exit-1 cases apart.

## Agent skill

```sh
pnpm dlx skills add enjunear/gunfog
```

The skill teaches a coding agent the score-and-rewrite loop: run gunfog, rewrite the sentences it names, run it again, stop when it exits 0 or the score stops moving.

## More

[SPEC.md](SPEC.md) is the full CLI contract. [CONTEXT.md](CONTEXT.md) defines the terms.
