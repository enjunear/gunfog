---
name: gunfog
description: >
  Rewrite prose until it reads at a target grade, using the gunfog CLI to score the Gunning
  fog index and name the sentences driving it. Use after writing or substantially revising a
  markdown document meant for human readers, or when the user mentions readability, fog,
  grade level, or prose density.
argument-hint: "[target] [path]"
---

# gunfog

Rewriting is the job and the score is the instrument. Loop until `gunfog` exits 0, or until the score stops moving.

## Arguments

`/gunfog [target] [path]`, both optional.

- A number is the target. Pass `--target N`. Without one, omit the flag and let the tool's default stand.
- A path is the file to score. Without one, score the prose you last produced or are preparing.

## Find the binary

1. `which gunfog`. Run it directly.
2. Nothing found. Stop, ask the user to install it, and wait. `brew install enjunear/tap/gunfog`, or the shell installer on https://github.com/enjunear/gunfog/releases. Running an installer is theirs, not yours.

## Run it

One input per call, so loop over files yourself.

```
gunfog --file draft.md --target 8
```

Always `--file`. Prose you are holding in context goes to a temporary file first. Write it with your file-editing tool, then score that file. Text you are scoring is often text you did not write, and text on a command line can execute.

Only `--file` prints `L<N>` line numbers on the hotspots. `--target` defaults to 10 and `--limit` caps the hotspot list at 10.

## The loop

Branch on the exit code, then on the first output line.

**Exit 0.** At or under target. Done.

**Exit 1, opening `fog: 12.4 (target 10)`.** Over target. Rewrite the hotspots, run again, repeat until 0 or until the score stalls.

**Exit 1, opening `no score: 34 words (min 100)`.** Under the floor of 100 prose words, where the score is noise. Skip the loop here. A second line names the complex words when the text has any. Swap the ones worth swapping and stop.

**Exit 2.** gunfog could not score at all, a bad flag or an unreadable path, with the reason on stderr. Report that reason and stop.

## Reading a hotspot

```
+2.17 35w L3 "The leave-one-out contribution of each sentence is computed…" complex: contribution, counterfactual, implementation, recalculates, documented, readability, remainder
```

The sentence's contribution, meaning the grades the document would lose without it, then its word count, its source line, its first eight words, and its complex words. A closing `+3 more, 1.2 grades remaining` means `--limit` cut the list short.

Fog is `0.4 × (words per sentence + percent complex words)`, so every hotspot offers two levers. Split it into shorter sentences, and trade its `complex:` words for ones of fewer syllables. Keep every fact, and win the grade in the prose itself. Headings, code blocks, tables and raw HTML are never scored, so moving text into them moves the number without improving the writing.

If two passes leave the score unmoved, stop and report what is left.
