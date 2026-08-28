# Naming collisions: `gfog` vs `gunfog`

Research for the `g-fog` planning effort. Date: 2026-08-28.

Question: this repo's CLI is currently `gfog` and may be renamed to `gunfog`. For each name,
is it free across the namespaces the project actually publishes to, and across a few adjacent
registries and general search where a collision would still cause confusion even though the
project does not publish there?

Companion to `rust-feasibility.md`, which picked cargo-dist as the distribution mechanism and
named crates.io, Homebrew and npm as the three channels the release pipeline targets.

This repo's own Cargo.toml does not exist yet. `find . -iname Cargo.toml -o -iname
package.json` from the repo root returns nothing. So the crate has never been published under
either name; there is no existing `gfog` publication on crates.io to protect or worry about.

## Verdict up front

**`gunfog` is clean everywhere checked, with zero hits of any kind.** `gfog` is also free on
every registry that matters for distribution, but it carries two low-stakes GitHub-side name
occupations that `gunfog` does not: a dormant GitHub username `Gfog` and an active but
unrelated `HubiRa/gfog` repository (a Python gradient-free-optimization library, 1 star,
pushed 2026-06-03). Neither blocks publishing a `gfog` crate, npm package, or Homebrew formula,
because none of those namespaces are shared with GitHub repo or user names. But `gunfog` beats
`gfog` on every axis measured: `gunfog` has no GitHub repo, no GitHub user, and no general-web
hit of any kind, literal or fuzzy. If collision risk is the deciding factor, **rename to
`gunfog`.**

## Findings table

| Namespace | `gfog` status | `gunfog` status |
| --- | --- | --- |
| crates.io | Free | Free |
| npm (bare name) | Free | Free |
| Homebrew core formula | Free | Free |
| Homebrew core cask | Free | Free |
| GitHub repo (exact name) | Taken, active (`HubiRa/gfog`) | Free |
| GitHub user (exact name) | Taken, dormant (`Gfog`) | Free |
| PyPI | Free | Free |
| Debian package | Free | Free |
| Ubuntu package | Free | Free |
| Arch AUR | Free | Free |
| General web (products, trademarks, games, bands) | No notable collision; two trivial hits (dead-for-sale domain, hobbyist profile) | No hits of any kind |

## crates.io

Checked with `curl -s -H "User-Agent: gfog-naming-research (michael@enjunear.com)"
https://crates.io/api/v1/crates/gfog` and the same for `gunfog`. crates.io's edge rejects
unauthenticated requests with no `User-Agent` header with a bare 403, which looks like a block
rather than a not-found; adding a descriptive User-Agent per crates.io's crawler policy gets
past it.

```
{"errors":[{"detail":"crate `gfog` does not exist"}]}
{"errors":[{"detail":"crate `gunfog` does not exist"}]}
```

Both names are free. This also answers the question raised in the research brief directly:
this project has not published a `gfog` crate yet. There is no Cargo.toml in the repo at all,
so the crate name has not even been declared locally, let alone reserved on crates.io.

## npm

Checked with `curl -s https://registry.npmjs.org/gfog` and `.../gunfog`. Both returned
`{"error":"Not found"}`, HTTP 404. Confirmed the bare unscoped name specifically, not a scoped
variant: the query path `/gfog` and `/gunfog` addresses the unscoped package directly on the
npm registry, which is a different endpoint from `/@user/gfog`. Both bare names are free.

## Homebrew

Checked all four combinations of formula/cask against both names:

- `https://formulae.brew.sh/api/formula/gfog.json` — HTTP 404
- `https://formulae.brew.sh/api/formula/gunfog.json` — HTTP 404
- `https://formulae.brew.sh/api/cask/gfog.json` — HTTP 404
- `https://formulae.brew.sh/api/cask/gunfog.json` — HTTP 404

Also checked `https://formulae.brew.sh/formula/gfog` (the human-facing page, not the API) and
got a GitHub Pages 404. No formula or cask exists for either name in homebrew-core. No
third-party tap turned up in the GitHub or general web search below, so there is nothing to
check there either.

## GitHub

Searched with `gh search repos gfog --limit 20` and `gh search repos gunfog --limit 20`
(GitHub code search API under the hood), then filtered to exact-name matches with `gh api
"search/repositories?q=<name>+in:name&per_page=30" --jq 'select(.name=="<name>")'`, and checked
usernames directly with `gh api users/gfog` and `gh api users/gunfog`.

**`gfog` repo.** The fuzzy search returns eighteen repos with `gfog` somewhere in the name
(mostly auto-generated dotfiles-style repos with random suffixes, `gfogwz`, `gfogxt`, `gfogip`,
and similar, none of them evidently related to text tooling). Filtering to the exact name
`gfog` leaves exactly one: `HubiRa/gfog`, "Gradient free optimisation via gradients", Python, 1
star, not archived, not a fork, created 2025-07-05, last pushed 2026-06-03. That is active
within the last three months and unrelated to readability scoring, so it is a real but
low-stakes collision: same exact repo path, different domain, actively maintained by someone
else.

**`gfog` user.** `gh api users/gfog` resolves to GitHub user `Gfog` (login case-folds; GitHub
usernames are case-insensitive), account id 63475780, created 2020-04-10, last updated
2020-04-10, zero followers, zero bio, one public repo. That one repo is `github-slideshow`,
GitHub's own onboarding template, not anything named gfog. The account has shown no activity
since creation day. This is a squatted, dormant username, not a live project.

**`gunfog` repo and user.** Both searches returned nothing. `gh search repos gunfog` returns an
empty list. `gh api users/gunfog` returns HTTP 404, `{"message":"Not Found"}`. Fully free.

## PyPI

Checked `curl -s https://pypi.org/pypi/gfog/json` and `.../gunfog/json`. Both returned
`{"message": "Not Found"}`, HTTP 404. Both free. This project does not plan to publish to PyPI,
but a squatted PyPI name under either candidate would still create confusion for anyone typing
`pip install gfog` by habit; neither exists.

## Debian and Ubuntu packages

Checked via WebFetch against the search UIs directly:

- `https://packages.debian.org/search?keywords=gfog` — "Sorry, your search gave no results"
- `https://packages.debian.org/search?keywords=gunfog` — "Sorry, your search gave no results"
- `https://packages.ubuntu.com/search?keywords=gfog` — "Sorry, your search gave no results"
- `https://packages.ubuntu.com/search?keywords=gunfog` — "Sorry, your search gave no results"

No package under either name in either distribution's archive. Both free.

## Arch AUR

Checked with `curl -s "https://aur.archlinux.org/rpc/v5/search/gfog"` and the same for
`gunfog`. Both returned `{"resultcount":0,"results":[],"type":"search","version":5}`. Both
free.

## General web search

**`gfog`.** A WebSearch for `"gfog" -github.com -crates.io` returns no company, trademark,
product, game, or band. The two hits with any content behind them are trivial: `gfog.com` is a
domain currently listed for sale by a domain reseller (Get On The Web Limited), not an active
site, and `sketchfab.com/gfog` is one hobbyist's 3D-model upload profile with no apparent
connection to text tooling. The rest of the result page is disambiguation pages for unrelated
three- and four-letter acronyms (`GFG`, `GFY`, `GFW`, `GFS`) and a word-unscrambler tool listing
"gfog" as an anagram fragment, not a term with independent meaning. Nothing here rises above
squatting-grade noise.

**`gunfog`.** A plain WebSearch for `"gunfog"` returns no literal match at all; the engine
silently splits the query into "gun" and "fog" and returns fog-machine and misting-device
retail pages (AliExpress fog guns, Wikipedia's "Gun (disambiguation)" page). A follow-up search
targeted at games, mods, and communities (`gunfog game mod discord steam`) also returned
nothing naming an actual "Gunfog" game, mod, Discord server, or Steam Workshop item; results
were generic modding-community links (GameBanana, Nexus Mods, a "GUNetwork" Steam group whose
name is coincidental, not a match). There is no game, band, company, or product called
"Gunfog" findable through general search. This is the strongest possible outcome for an
invented compound: it does not even produce false-positive noise.

## Sources

All checks run 2026-08-28.

- crates.io API: `https://crates.io/api/v1/crates/gfog`,
  `https://crates.io/api/v1/crates/gunfog` (queried with a descriptive `User-Agent` header;
  crates.io's edge 403s requests with no User-Agent)
- npm registry API: `https://registry.npmjs.org/gfog`, `https://registry.npmjs.org/gunfog`
- Homebrew formula API: `https://formulae.brew.sh/api/formula/gfog.json`,
  `https://formulae.brew.sh/api/formula/gunfog.json`
- Homebrew cask API: `https://formulae.brew.sh/api/cask/gfog.json`,
  `https://formulae.brew.sh/api/cask/gunfog.json`
- Homebrew formula page: `https://formulae.brew.sh/formula/gfog`
- GitHub repo search: `gh search repos gfog --limit 20`, `gh search repos gunfog --limit 20`;
  exact-name filter via `gh api "search/repositories?q=gfog+in:name&per_page=30"` and the
  `gunfog` equivalent
- GitHub repo detail: `gh api repos/HubiRa/gfog`
- GitHub user lookup: `gh api users/gfog`, `gh api users/gunfog`, `gh api users/Gfog/repos`
- PyPI JSON API: `https://pypi.org/pypi/gfog/json`, `https://pypi.org/pypi/gunfog/json`
- Debian package search: `https://packages.debian.org/search?keywords=gfog`,
  `https://packages.debian.org/search?keywords=gunfog`
- Ubuntu package search: `https://packages.ubuntu.com/search?keywords=gfog`,
  `https://packages.ubuntu.com/search?keywords=gunfog`
- Arch AUR RPC API: `https://aur.archlinux.org/rpc/v5/search/gfog`,
  `https://aur.archlinux.org/rpc/v5/search/gunfog`
- General web search (WebSearch tool, backed by live web results): `"gfog" -github.com
  -crates.io`, `"gunfog"`, `gunfog game mod discord steam`, `"GFOG" trademark OR company OR app
  -readability`
