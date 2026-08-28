# Counting English syllables, and the narrower question gfog actually asks

Research for the `g-fog` planning effort. Date: 2026-08-28.

Question: is there a well-tested regex, library or algorithm for counting English
syllables that beats the options already priced in `rust-feasibility.md`? And does it change
anything that `gfog` never needs a syllable count, only the boolean "3 or more"?

Companion to `existing-readability-tools.md` and `rust-feasibility.md`. The earlier note
measured four candidates against 31 hand-labelled words and recommended porting npm
`syllable`'s rules. This note re-runs that comparison at four orders of magnitude more scale,
against the boundary that actually matters.

Every accuracy number below was measured on this machine against the CMU Pronouncing
Dictionary. Sizes and timings are from Rust binaries built here. Scripts live in the session
scratchpad; the method is described inline so the numbers can be re-run.

## Verdict up front

**Three findings, in descending order of how much they should change the plan.**

**1. The 3+ boundary is a much easier question than the syllable count, and nobody has
published that.** Every counter tested gains 8 to 11 points moving from exact-count accuracy
to 3+-boundary accuracy. The naive vowel-group regex gets 82.7% of counts right on 58,743
held-out CMUdict words but **92.9% of the 3+ calls**. npm `syllable` goes from 92.4% to
96.5%. The errors that make syllable counters look bad are mostly ±1 errors well away from
2-versus-3. (These are not comparable to the 18/31 and 28/31 in `rust-feasibility.md`, which
were deliberately hard hand-picked words; the ranking is the same, the absolute level is much
higher on a full dictionary.)

**2. A 29-rule set tuned on the boundary matches npm `syllable` at a fraction of the cost.**
Vowel groups plus 29 orthographic corrections, selected greedily on half of CMUdict, scores
**96.09% on the held-out half** against npm `syllable`'s 96.53%. On real agent-written prose,
token-weighted, it is 98.16% against 98.41%. Hand-coded in Rust with **no regex engine at
all** it is 38 KB of binary over a hello-world floor and 0.38 µs per word.

**3. `regex-lite` is cheap in bytes and expensive in time, which the earlier note missed.**
The `kincaid` crate's 207-pattern rule set is the most accurate rules-only option measured
(96.89%), but run through `regex-lite` it costs **79 µs per word**, roughly 40 ms for a
500-word document against the 1.0 ms whole-program startup the earlier spike achieved. The
same logic written as byte comparisons is 208x faster. Recommend rules, not regexes.

Two corrections to `rust-feasibility.md` fall out of this:

- Its Option A (port npm `syllable`) named `pluralize` as "the one piece that does not port
  cheaply". **Measured: `pluralize` and the entire 60-entry `problematic` dictionary together
  are worth 0.02 percentage points on the 3+ boundary** (96.51% without them, 96.53% with).
  Drop both. There is nothing expensive left in the port.
- Its size table lists "Spike + embedded 51,628-word polysyllabic list" at 819,448 bytes,
  +66 KB over the 753,376-byte spike. That cannot be right: the list file alone is 524,564
  bytes. Measured here, a hello-world floor of 291,784 bytes becomes **828,216 bytes** with
  the same list embedded, a delta of **+536,432 bytes**. Budget 524 KB for the dictionary
  option, not 66 KB.

## 1. What exists, and what it claims

### 1.1 The classic vowel-group regex

Every language has a version of it. Count runs of `[aeiouy]`, subtract one for a silent final
`e`, floor at 1. The Rust `gunning-fog` crate v0.1.0 is five lines of exactly this
(https://github.com/astuanax/gunning-fog); py-readability-metrics ships a slightly longer
variant (https://github.com/cdimascio/py-readability-metrics,
`readability/text/syllables.py`).

**No published accuracy figure exists for it.** I searched for one and found only tool
READMEs and blog posts making unsourced claims. So the numbers in section 2 are, as far as I
can tell, the first measured figures for this family against a reference dictionary.

Measured here on 58,743 held-out CMUdict words: **82.71% exact, 92.94% on the 3+ boundary**.

### 1.2 Lingua::EN::Syllable, the ancestor, and the only documented error rate

Perl, by Greg Fast, v0.251 (1999). Source fetched from
https://fastapi.metacpan.org/source/GREGFAST/Lingua-EN-Syllable-0.251/Syllable.pm. Still on
CPAN, now maintained by Neil Bowers; latest release 0.31, 2022-04-16
(https://metacpan.org/dist/Lingua-EN-Syllable). Licensed under the Perl licence.

This is the only implementation in any language that states its own error rate. From the POD,
verbatim:

> Note that it isn't entirely accurate... it fails (by one syllable) for about 10-15% of my
> /usr/dict/words. The only way to get a 100% accurate count is to do a dictionary lookup, so
> this is a small and fast alternative where more-or-less accurate results will suffice, such
> as estimating the reading level of a document.

And from the source comment header:

> note that this is not infallible. it does fail for some percentage of words (10% seems a
> good guess)... so it's useful for approximation, but don't use this for running your nuclear
> reactor...

Both figures are Fast's own informal estimate against his local `/usr/dict/words`, not a
benchmark against labelled data. **Measured here, it holds up: 87.63% exact on held-out
CMUdict, a 12.4% failure rate, inside his stated 10-15% band.** That is a 27-year-old
accuracy claim that survives checking, which is more than most software documentation manages.

The algorithm is the shape everything downstream inherited. Lowercase, strip apostrophes,
strip a trailing `e`, split on non-vowel runs to get a base count, then walk two arrays of
regexes adding or subtracting one per match. The arrays, verbatim from 0.251:

```perl
@SubSyl = ('cial', 'tia', 'cius', 'cious', 'giu', 'ion', 'iou', 'sia$', '.ely$');
@AddSyl = ('ia', 'riet', 'dien', 'iu', 'io', 'ii', '[aeiouym]bl$', '[aeiou]{3}',
           '^mc', 'ism$', '([^aeiouy])\1l$', '[^l]lien', '^coa[dglx].',
           '[^gq]ua[^auieo]', 'dnt$');
```

Note `([^aeiouy])\1l$` (middle, twiddle, battle). That is a backreference, which neither
`regex` nor `regex-lite` supports. It has to be hand-coded or dropped in any Rust port.

One porting trap, since a coding agent will hit it. Perl's `split` discards trailing empty
fields; Python's `re.split` and Rust's `str::split` do not. Get that wrong and every word
ending in a consonant is overcounted by one, which drops exact accuracy from 87.6% to 23.3%.
I hit it on the first run.

### 1.3 npm `syllable`, the same algorithm two ports later

https://github.com/words/syllable, v5.0.1, MIT, by Titus Wormer. Its README states the
lineage:

> Based on the syllable functionality found in `Text-Statistics` (PHP), in turn inspired by
> `Lingua::EN::Syllable` (Perl). Support for word-breaks, non-ASCII characters, and many
> fixes added later.

Reading the two sources side by side confirms it. `cia(?:l|$)`, `tia`, `cius`, `cious`,
`iou`, `sia$`, `^jua`, `uai`, `eau`, `.[^aeiuoycgltdb]{2,}ed$`, `[^gq]ua[^auieo]`, `^mc`,
`ism$`, `riet`, `dien` are all Fast's patterns, carried through unchanged. The additions are
a prefix/suffix table worth 1-3 syllables, four more correction groups, and a hand-curated
`problematic.js` of about 60 irregular words.

**It publishes no accuracy figure.** Nothing in the README, changelog or repo. The 28/31 in
`rust-feasibility.md` was the first measurement of it anywhere I can find.

Its `package.json` still lists `pluralize`, `normalize-strings` and `@types/pluralize` as
dependencies. Section 2.2 prices what they buy.

### 1.4 Hyphenation patterns are not syllable boundaries, and the tools say so

`rust-feasibility.md` ruled out the `hyphenation` crate on measurement (20/31, every error an
undercount). The primary sources agree with that conclusion, which is worth recording because
the temptation to reach for a hyphenation dictionary keeps recurring.

- **Pyphen** (https://pyphen.org/, https://github.com/Kozea/Pyphen) describes itself as "a
  pure Python module to hyphenate words using included or external Hunspell hyphenation
  dictionaries". It makes no claim that hyphenation points approximate syllables. Its
  `hyph_en_US.dic` is 106,414 bytes, small enough to embed, and still the wrong data.
- **textstat** does not trust it for English. Its docs say it "uses the Python module Pyphen
  for syllable calculation in most languages, but defaults to `nltk.corpus.cmudict` for
  en_US" (https://github.com/textstat/textstat). A library reaching for a real pronunciation
  dictionary for the one language where it has one is a strong signal.
- **`readsight` v1.0.2** (https://github.com/MADEVAL/ReadSight, MIT, 2026-07-12), a new Rust
  crate built on Liang's algorithm over hyph-utf8 patterns, concedes the point in its own
  docs: "TeX hyphenation patterns are optimised for line-breaking, not phonetic
  syllabification." For `en-us` it falls back to a vowel-count heuristic first and uses TeX
  only as backup. 435 downloads.
- **`hypher` v0.1.7** (https://github.com/typst/hypher, MIT/Apache-2.0, 2.5M downloads,
  last release 2026-04-07) is the healthiest crate in this family by a wide margin, from the
  Typst team, with patterns compiled to automata at build time. It is still a hyphenation
  library: its README demonstrates line-break points ("ex-ten-sive"), not syllables. All 30+
  languages cost about 1.1 MiB.

Same category, same defect. Ruled out.

### 1.5 CMUdict and NLTK

NLTK ships **no syllable counter**. It ships `nltk.corpus.cmudict`, a corpus reader over
`cmudict.0.6` (127,069 entries), and counting stress-marked phonemes from it is a community
idiom rather than an API. There is nothing to measure: for an in-vocabulary word it is a
lookup, exact by construction, and for anything else it fails.

`nltk.tokenize.SyllableTokenizer` implements the Sonority Sequencing Principle and is
explicit that it is not an English orthography tool: its docstring says SSP is "a language
agnostic algorithm proposed by Otto Jesperson in 1904" and points at Bartlett et al. (2009)
as a benchmark "if utilizing IPA"
(https://www.nltk.org/api/nltk.tokenize.sonority_sequencing.html). Feeding it spelled English
is off-label.

Upstream CMUdict (https://github.com/cmusphinx/cmudict) is BSD-2-Clause since 0.7a,
"Copyright (C) 1993-2015 Carnegie Mellon University". Embeds fine with attribution.

**CMUdict has its own ambiguity floor at this boundary.** Measured: 1.35% of headwords have
pronunciation variants that disagree on syllable count, and **0.81% straddle the 3+ boundary
specifically**: `actual`, `actually`, `acreage`, `abler`. So a perfect dictionary lookup is
not a perfect classifier; roughly 0.8% of words genuinely have two right answers depending on
who is speaking. Any figure above about 99% should be read with that in mind.

### 1.6 What the academic literature does and does not measure

The literature is about **syllabification**, placing boundaries inside a word. That is a
harder task than counting and a much harder task than the binary call `gfog` needs. The
numbers do not transfer, but they set context.

| Work | Task | Best word accuracy |
| --- | --- | --- |
| Marchand, Adsett & Damper, SSW6 2007 | Orthographic, Webster's Pocket | Fisher/Kahn rules 54.2%; syllabification-by-analogy 88.5% |
| Bartlett, Kondrak & Cherry, ACL 2008 | Orthographic, CELEX English | Structured SVM "Break ONC" 89.99%; their re-run of SbA 84.97% |
| Bartlett, Kondrak & Cherry, NAACL 2009 | **Phonemic**, CELEX English | SVM-HMM 98.86% |

Sources: https://www.isca-archive.org/ssw_2007/marchand07_ssw.html,
https://aclanthology.org/P08-1065/, https://aclanthology.org/N09-1035/. The 2009 journal
version, Marchand, Adsett & Damper, *Language and Speech* 52(1):1-27,
doi:10.1177/0023830908099881, is paywalled and its exact table was not retrieved; the 2007
workshop numbers above are from the same group's precursor paper. Note the last row is
syllabifying a known phoneme string, an easier problem than starting from spelling, and is
not comparable to the rows above it.

**Nothing measures what `gfog` needs.** No peer-reviewed source I could find evaluates:
plain vowel-group syllable *counters* against a reference dictionary; CMUdict-versus-heuristic
disagreement rates; or binary 3+-syllable detection. McLaughlin's SMOG paper (1969, *Journal
of Reading* 12(8):639-646,
https://ogg.osu.edu/media/documents/health_lit/WRRSMOG_Readability_Formula_G._Harry_McLaughlin__1969_.pdf)
defines counting polysyllables by hand and reports r = 0.985 for the formula, but says
nothing about automating the count. The only thing close is an arXiv preprint,
Malhotra & Kamle 2021 (arXiv:2101.09464), reporting 82.5% for a rule-based approach, but it
is multi-class syllable-count prediction with no stated dataset size, ground truth or
train/test split, and no sign of peer review. Not citable.

There is one solid citation for why any of this matters. Mac et al., *JAMA Network Open*
2022;5(12):e2246051, doi:10.1001/jamanetworkopen.2022.46051
(https://pmc.ncbi.nlm.nih.gov/articles/PMC9856555/), scored the same 10 CDC health pages
through 8 online calculators and found scores **up to 12.9 grade levels apart** using
nominally the same formula, narrowing to about 2.1 grades after standardising text
preparation. That is the same disease `existing-readability-tools.md` measured at 4.9 grades
across shipped fog implementations.

I also could not find any source for the folk claim that syllable-count errors average out
across a document. The one paper arguing for Flesch-Kincaid's stability, Ehara, PACLIC 2024
(https://aclanthology.org/2024.paclic-1.94.pdf), argues something different: that English
phonetic structure is stable over decades while vocabulary drifts. Section 5 measures the
averaging-out question directly instead.

## 2. Measured: exact count versus the 3+ boundary

**Method.** Gold standard is CMUdict, first pronunciation per headword, syllables = phones
carrying a stress digit. Restricted to lowercase alphabetic headwords: **117,485 words**, of
which 41.1% are 3+ syllables. Split 50/50 with a fixed seed; all figures below are the
held-out half, 58,743 words. npm `syllable` v5.0.1 was run from the installed package under
Bun, so it is the real implementation and not a re-port.

### 2.1 Held-out CMUdict, 58,743 word types

| Counter | Exact % | **3+ acc %** | Precision % | Recall % | FP | FN |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `naive_gf` (the `gunning-fog` crate) | 82.71 | 92.94 | 93.64 | 88.85 | 1456 | 2691 |
| classic 5-line regex (py-readability-metrics) | 85.84 | 93.65 | 95.49 | 88.72 | 1010 | 2722 |
| vowel groups + silent-e/-le | 83.59 | 93.06 | 93.38 | 89.44 | 1530 | 2547 |
| Lingua::EN::Syllable 0.251 | 87.63 | 94.54 | 92.96 | 93.80 | 1714 | 1496 |
| **tuned rules, first 6** | 86.90 | **95.33** | 94.86 | 93.69 | 1224 | 1522 |
| **tuned rules, first 12** | 88.78 | **95.85** | 95.44 | 94.41 | 1089 | 1349 |
| **tuned rules, all 29** | 91.05 | **96.09** | 95.68 | 94.76 | 1031 | 1265 |
| `kincaid` crate (207 regexes) | 93.85 | **96.89** | 96.54 | 95.86 | 830 | 998 |
| npm `syllable`, no dict, no pluralize | 92.33 | 96.51 | 95.43 | 96.11 | 1111 | 938 |
| npm `syllable` 5.0.1, as shipped | 92.38 | 96.53 | 95.44 | 96.14 | 1108 | 931 |

The gap between the two accuracy columns is the whole point. **The worst counter in the table
answers the question `gfog` asks correctly 92.9% of the time while getting the count wrong
17.3% of the time.** Counting is the hard problem; `gfog` does not have it.

### 2.2 The npm `syllable` dependencies buy essentially nothing

I patched a copy of `index.js` to flag off the `problematic` lookup and the `pluralize`
singularisation, and re-ran all three configurations over the same 117,485 words:

| Configuration | Exact % | 3+ acc % |
| --- | ---: | ---: |
| Full, as shipped | 92.38 | 96.53 |
| Without `pluralize` | 92.37 | 96.53 |
| Without `pluralize` and without `problematic` | 92.33 | 96.51 |

**0.02 percentage points.** The 60-word exception table and the `pluralize` dependency
together move 12 words in 58,743. `rust-feasibility.md` called `pluralize` the one piece that
does not port cheaply; it does not need porting. Neither does `problematic.js`.

### 2.3 Token-weighted on real agent-written markdown

Types are not tokens. Weighting by how often words actually appear in this repo's own
research notes (15,438 tokens, 90.2% in CMUdict, in-vocabulary words only so this isolates
counter error from vocabulary error):

| Counter | 3+ acc % | Precision % | Recall % |
| --- | ---: | ---: | ---: |
| `naive_gf` | 95.92 | 89.39 | 83.70 |
| classic 5-line regex | 96.99 | 96.53 | 83.60 |
| Lingua::EN::Syllable 0.251 | 96.99 | 89.79 | 90.99 |
| **tuned rules, 29** | **98.16** | 96.22 | 91.78 |
| `kincaid` crate | 98.05 | 95.66 | 91.64 |
| npm `syllable` 5.0.1 | 98.41 | 97.74 | 91.87 |

Everything improves by roughly two points, because common words are short and easy. The
spread between the best and worst option narrows to 2.5 points.

## 3. Rust options not previously surveyed

`rust-feasibility.md` ruled out `syllarust`, `hyphenation`, the `syllable` crate, `cmudict`
and `rust_readability`. A fresh sweep of the crates.io API across `syllable`,
`syllabification`, `phoneme`, `pronunciation`, `readability`, `hyphenation`, `flesch`, `g2p`,
`espeak` and `phonetics` turned up one genuinely relevant crate and several dead ends.

### 3.1 `kincaid`, the best rules-only accuracy measured

https://crates.io/crates/kincaid v0.2.4, https://github.com/PawanHegde/kincaid, MIT, 7,126
downloads. **Last released 2020-06-05**, six years stale.

It is an independent Rust implementation of the Lingua/`syllable` family, not a port. From
`src/lib.rs`:

```rust
fn syllables_in_word(word: &str) -> usize {
    let vowel_groups: usize = VOWEL_GROUPS_REGEX.find_iter(word).count();
    let add_count: usize = ADD_PATTERN_REGEXES.matches(word).iter().count();
    let minus_count: usize = DEDUCT_PATTERN_REGEXES.matches(word).iter().count();
    if vowel_groups + add_count < minus_count + 1 { return 1; }
    return vowel_groups + add_count - minus_count;
}
```

74 deduct patterns and 133 add patterns, run as two `RegexSet`s. Extracted and checked: **none
of the 207 patterns uses lookaround or a backreference**, and the only non-basic syntax is
`\b`, so the set is `regex-lite`-compatible.

At 96.89% on the held-out boundary it is the most accurate rules-only option measured, ahead
of npm `syllable`. Two caveats. It is unmaintained. And unlike the tuned set in section 4, its
patterns were not developed on a known split, so I cannot rule out that some were fitted to a
word list overlapping CMUdict; treat 96.89% as an upper estimate.

Section 5 explains why I still would not use it as written.

### 3.2 Everything else

- **`textstat` v0.1.1** (https://github.com/trananhtung/textstat, MIT, June 2026, 135
  downloads). The crates.io description promises Gunning Fog and SMOG, but its `syllables()`
  is a plain vowel-group-plus-silent-e heuristic with no correction rules. Same family as the
  `gunning-fog` crate, three months old.
- **`phonetik` v0.3.2** (https://github.com/Void-n-Null/phonetik, MIT) and **`cmudict-fast`
  v0.8.0** (https://github.com/BenjaminHinchliff/cmudict-fast) both embed all of CMUdict.
  Same 3.6 MB problem that disqualified `syllarust`.
- **`hyphenator` v0.1.0**, **`readability-stats` v0.1.1**, **`spandex-hyphenation`**,
  **`kl-hyphenate`**: naive heuristics, near-zero adoption, or Knuth-Liang forks.
- **`rust_readability` v0.2.0** by ian-nai is GPL-3.0-or-later. Still copyleft, still out.
- **OpenEPD** (https://github.com/JackDanger/open-english-pronouncing-dictionary), a ~280K-word
  IPA dictionary, is CC-BY-SA 4.0, a worse licence for embedding than CMUdict's BSD, and
  carries no syllable-count field.

**No Rust port of npm `syllable`, `Lingua::EN::Syllable` or Python `textstat` exists.** The
only port of the Perl module in any language is to Raku
(https://github.com/coke/raku-lingua-en-syllable). No FFI-free, dictionary-free,
high-accuracy Rust counter is available off the shelf. That part of `rust-feasibility.md`'s
assessment stands.

## 4. A rule set built for the boundary

Since the boundary is the question, I tried optimising for it directly rather than optimising
counts and hoping.

**Method.** Base estimate is vowel-letter groups with the silent-`e` and consonant+`le`
corrections. Candidate rules are the union of Fast's `@SubSyl`/`@AddSyl`, npm `syllable`'s
correction groups, and every 2-5 character suffix and 2-4 character prefix appearing in at
least 150 training words, each as both a +1 and a -1 rule: 701 candidates. Greedy forward
selection on the training half, scoring **3+ boundary accuracy only**, stopping when no
candidate improves it. Reported on the held-out half, which was never touched during
selection.

Selection stopped at 29 rules. The gain curve, held-out at each step:

| Rules | 1 | 3 | 6 | 12 | 20 | 29 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Held-out 3+ accuracy % | 93.79 | 94.72 | 95.33 | 95.85 | 96.12 | 96.09 |

**Twelve rules get 95.85%; the next seventeen buy 0.24 points.** The first six alone beat
Lingua::EN::Syllable's 94.54%. The selected set, in the order chosen:

```
-1  [^td]ed$          +1  ^mc               +1  ia                +1  io
+1  [aeiou]y[aeiou]   +1  ier$              -1  lle$              +1  ee$
-1  tes$              -1  nes$              +1  ie$               +1  eo
+1  iu                -1  .ely$             -1  ion$              +1  [^gq]ua[^auieo]
-1  ions$             +1  ism$              +1  creat             -1  cious
+1  [^aeiou]y[ae]     -1  tia               +1  [^l]lien          -1  giu
+1  dien              -1  cial              -1  awe($|d|so)       -1  ^jua
+1  asm$
```

Twenty-three of the 29 came from the seeded prior art, Fast's 1998 arrays and npm
`syllable`'s groups, chosen over roughly 670 generated alternatives. That is a reasonable
independent check on the hand-written rules: given a free choice, the search kept most of
them. The six the search added from scratch are all endings that sit right on the 2-versus-3
line: `ier$`, `lle$`, `ee$`, `tes$`, `nes$`, `ie$`. The wider run added more of the same
shape, `des$`, `res$`, `ves$`, `mes$` for silent `-es` and `ied$`, `iest$` for the `y`-to-`i`
alternation. Silent plural and past-tense endings are where the published rule sets are
thinnest.

Widening the candidate pool (min count 40, up to 120 steps) selected 56 rules and reached
**96.68% held-out**, still short of `kincaid`'s 96.89% with 207. The ceiling for
"vowel groups plus orthographic corrections" is somewhere just under 97%, and the last point
costs an order of magnitude more rules.

## 5. What it costs in Rust, and what the error does to a fog score

### 5.1 Binary size and speed, measured

All builds used `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`,
`strip = true`, matching `rust-feasibility.md`. Timings are over all 117,485 CMUdict words,
best of three.

| Implementation | Binary | Over floor | Per word | Init |
| --- | ---: | ---: | ---: | ---: |
| Hello-world floor | 291,784 | n/a | n/a | n/a |
| **29 tuned rules, hand-coded bytes, no regex** | **330,248** | **+38,464** | **0.38 µs** | 0 |
| 29 tuned rules via `regex-lite` | 400,112 | +108,328 | 12.0 µs | 35 µs |
| `kincaid`'s 207 patterns via `regex-lite` | 400,112 | +108,328 | **79.1 µs** | 210 µs |
| Embedded 51,628-word list, `Vec<&str>` + binary search | 828,216 | +536,432 | 0.15 µs | 1.4 ms |
| Embedded list, binary search over the sorted blob | 828,216 | +536,432 | 0.19 µs | **0** |

The two `regex-lite` rows share one binary, which holds both rule sets; so do the two
dictionary rows, whose build carried both lookup functions and measured 846,280 bytes. The
828,216 figure is a build with the `Vec` path alone.

**The timing column is the surprise.** `regex-lite`'s own docs say plainly that "regex
searches in this crate are typically substantially slower than what is provided by the
`regex` crate" and that it "prioritizes smaller binary sizes and shorter Rust compile times
over performance and functionality"
(https://docs.rs/regex-lite/latest/regex_lite/). Running 207 separate `is_match` calls per
word turns that into 79 µs. For a 500-word document that is 40 ms of syllable counting on top
of a program whose entire measured startup was 1.0 ms.

It is also unnecessary. **Twenty-two of the 29 tuned rules are plain literals**, meaning
`contains`, `starts_with` or `ends_with`. The other seven are two- or three-byte windows with a
character-class test. Written that way the whole counter is about 65 lines of Rust, needs no
dependency at all, and reproduces the Python version's accuracy: **96.10% held-out versus
96.09%.**

The dictionary rows carry the correction from the verdict. The 51,628-word list costs 524 KB
of data, not 66 KB; a floor binary embedding it and binary-searching a `Vec<&str>` measured
828,216 bytes. (The 846,280-byte row is a build carrying both lookup functions, so it
overstates either one by about 18 KB.) Binary-searching the sorted blob directly avoids the
1.4 ms `Vec` construction entirely and is only 0.04 µs/word slower, so a short-lived CLI
should do that.

### 5.2 Does counter error actually move the score?

Measured over 889 sentences of this repo's own research markdown, in-vocabulary words only,
against gold CMUdict counts.

| Counter | Document fog | Δ | Complex words | Mean per-sentence \|Δfog\| | p95 |
| --- | ---: | ---: | ---: | ---: | ---: |
| gold (CMUdict lookup) | 12.96 | n/a | 2165 | n/a | n/a |
| `naive_gf` | 12.57 | -0.40 | 2027 | 1.23 | 5.71 |
| classic 5-line regex | 12.13 | **-0.83** | 1875 | 0.97 | 4.44 |
| Lingua::EN::Syllable | 13.05 | +0.08 | 2194 | 0.96 | 4.80 |
| tuned rules, 12 | 12.87 | -0.10 | 2131 | 0.77 | 4.21 |
| tuned rules, 29 | 12.68 | -0.29 | 2065 | 0.57 | 3.64 |
| `kincaid` | 12.70 | -0.26 | 2074 | 0.60 | 3.64 |
| npm `syllable` 5.0.1 | 12.59 | -0.37 | 2035 | **0.51** | 3.33 |

Two different answers depending on what `gfog` reports.

**At document scope, the counter barely matters.** The whole field spans 0.9 grade levels,
against the 4.9-grade spread `existing-readability-tools.md` measured between shipped tools.
Errors do partially cancel, and even the naive counter lands within half a grade. If `gfog`
only printed one number, this research would be moot.

**At sentence scope, it matters, and that is what `gfog` reports.** Mean absolute per-sentence
error runs 0.51 to 1.23 grades and the 95th percentile runs 3.3 to 5.7. A tool that says
"this sentence scores 26.2, here are the six complex words" is making a claim about specific
words on screen, where a wrong call is visible rather than averaged away. Going from the naive
counter to the tuned set halves the mean error and cuts p95 by two grades.

Note also that `classic`, the counter shipped by py-readability-metrics, has the largest
document-level bias in the table at -0.83, because its precision is high and recall is poor:
it undercounts complex words by 13.4%. A one-way bias is the failure mode that does not
average out.

## 6. Verdict

Ranked for `gfog`: a Rust binary under 2 MB, reporting per-sentence scores and naming the
complex words.

**1. Hand-coded 29-rule counter, no regex engine. Recommended.**
96.1% on the held-out 3+ boundary, 98.2% token-weighted on real prose. **+38 KB, 0.38 µs per
word, zero dependencies.** Within 0.4 points of npm `syllable` and 0.8 of `kincaid`, at a
twentieth the size cost of the dictionary and two orders of magnitude less time than the
regex route. The 29 rules are in section 4; twelve of them get 95.9% if brevity matters more
than the last quarter-point. Build it with a test that diffs against CMUdict, which makes the
accuracy claim reproducible in CI.

**2. Hybrid: embedded polysyllabic list, rules for the rest.** The accuracy ceiling.
`rust-feasibility.md` measured out-of-vocabulary rates on real target input at 3.9% of
fog-eligible occurrences; with the tuned rules as fallback (96.18% on held-out OOV words) that
gives **99.85% overall**, or 99.63% at the pessimistic 9.6% OOV rate. Cost: **+524 KB** and a
BSD-2-Clause attribution. That fits, at roughly 1.46 MB with the rest of the recommended
stack, but it spends a quarter of the budget to move from 96% to 99.8% on a boundary where CMUdict
itself is ambiguous for 0.81% of words. Worth doing only if hotspot precision turns out to be
the thing users complain about. Keep it as the documented fallback, exactly as the earlier
note proposed, and use blob binary search rather than a `Vec` so it costs no startup time.

**3. Port `kincaid`'s 207 patterns as hand-coded rules.** 96.89%, the best rules-only figure
measured, MIT, and the patterns are regex-free-able (no lookaround, no backreferences). The
extra 0.8 points over option 1 costs 178 more rules to hand-code and verify. Do this only if
option 1's error rate proves visible in practice. **Do not depend on the crate directly**: it
is six years unmaintained and pulls the full `regex` crate, which `rust-feasibility.md`
measured at +1.19 MB.

**4. Port npm `syllable`'s rules.** What the earlier note recommended, and still fine at
96.5%. The revision is that it is cheaper than advertised: skip `pluralize` and
`problematic.js`, which are worth 0.02 points. What remains is one prefix/suffix table and
eight correction groups.

**Rejected: a pure regex-only heuristic.** Not because accuracy is bad. 92.9% on the boundary
for five lines is a genuinely good trade, and the reason no one has noticed is that everyone
measures exact counts. Rejected because in Rust the regex engine is the expensive part. The
full `regex` crate costs 1.19 MB, more than half the budget, for Unicode tables an ASCII
letter heuristic never touches. `regex-lite` costs 59 KB but 12-79 µs per word. Hand-coded
byte matching is smaller, faster and no harder to read. The regex formulation is an artifact
of the algorithm having been born in Perl in 1998.

**Also rejected: any hyphenation-pattern approach.** Pyphen, `hypher`, `readsight` and the
`hyphenation` crate all encode where a line may break, not where a syllable divides. Their own
documentation says so, and textstat reaches past Pyphen for CMUdict on English specifically.

One thing to carry into the spec. `gfog`'s canonical complex-word rule strips `-es`, `-ed` and
`-ing` before counting (`existing-readability-tools.md` section 2.2), and several of the tuned
rules (`[^td]ed$`, `tes$`, `nes$`, `ions$`) do overlapping work. Order the two passes
deliberately and test them together, or the suffix rule will silently double-count what the
syllable rules already handled.

## Sources

Measured on this machine, 2026-08-28: Linux x86_64, rustc 1.98.0, Bun 1.4.0, Python 3.14.
Gold standard is CMUdict as vendored in `syllarust` 0.3.0, 117,485 lowercase alphabetic
headwords, first pronunciation, 50/50 split under a fixed seed. npm `syllable` v5.0.1 was run
from the installed package, not re-implemented.

Implementations:
- Lingua::EN::Syllable: https://metacpan.org/dist/Lingua-EN-Syllable, source 0.251 at
  https://fastapi.metacpan.org/source/GREGFAST/Lingua-EN-Syllable-0.251/Syllable.pm,
  maintained fork https://github.com/neilb/Lingua-EN-Syllable
- npm `syllable`: https://github.com/words/syllable
- Text-Statistics (PHP): https://github.com/DaveChild/Text-Statistics
- `kincaid` crate: https://crates.io/crates/kincaid, https://github.com/PawanHegde/kincaid
- `gunning-fog` crate: https://github.com/astuanax/gunning-fog
- py-readability-metrics: https://github.com/cdimascio/py-readability-metrics
- textstat: https://github.com/textstat/textstat
- `textstat` crate: https://github.com/trananhtung/textstat
- `phonetik`: https://github.com/Void-n-Null/phonetik; `cmudict-fast`:
  https://github.com/BenjaminHinchliff/cmudict-fast
- `readsight`: https://github.com/MADEVAL/ReadSight; `hypher`: https://github.com/typst/hypher
- `hyphenator`: https://github.com/d3z-the-dev/hyphenator
- OpenEPD: https://github.com/JackDanger/open-english-pronouncing-dictionary
- Raku port of Lingua::EN::Syllable: https://github.com/coke/raku-lingua-en-syllable
- `regex-lite`: https://docs.rs/regex-lite/latest/regex_lite/

Data and specs:
- CMU Pronouncing Dictionary: https://github.com/cmusphinx/cmudict, licence
  https://github.com/cmusphinx/cmudict/blob/master/LICENSE
- Pyphen: https://pyphen.org/, https://github.com/Kozea/Pyphen
- NLTK cmudict reader: https://www.nltk.org/api/nltk.corpus.reader.cmudict.html;
  SyllableTokenizer: https://www.nltk.org/api/nltk.tokenize.sonority_sequencing.html

Papers:
- Marchand Y, Adsett CR, Damper RI. "Evaluating Automatic Syllabification Algorithms for
  English." SSW6, 2007. https://www.isca-archive.org/ssw_2007/marchand07_ssw.html
- Marchand Y, Adsett CR, Damper RI. "Automatic Syllabification in English: A Comparison of
  Different Algorithms." *Language and Speech* 2009;52(1):1-27,
  doi:10.1177/0023830908099881 (paywalled, exact table not retrieved)
- Bartlett S, Kondrak G, Cherry C. "Automatic Syllabification with Structured SVMs for
  Letter-to-Phoneme Conversion." ACL-08: HLT, 568-576. https://aclanthology.org/P08-1065/
- Bartlett S, Kondrak G, Cherry C. "On the Syllabification of Phonemes." NAACL-HLT 2009,
  308-316. https://aclanthology.org/N09-1035/
- McLaughlin GH. "SMOG Grading: A New Readability Formula." *Journal of Reading*
  1969;12(8):639-646.
  https://ogg.osu.edu/media/documents/health_lit/WRRSMOG_Readability_Formula_G._Harry_McLaughlin__1969_.pdf
- Mac O, Ayre J, Bell K, McCaffery K, Muscat DM. "Comparison of Readability Scores for Written
  Health Information Across Formulas Using Automated vs Manual Measures." *JAMA Netw Open*
  2022;5(12):e2246051, doi:10.1001/jamanetworkopen.2022.46051.
  https://pmc.ncbi.nlm.nih.gov/articles/PMC9856555/
- Ehara Y. "An Analytical Study of the Flesch-Kincaid Readability Formulae to Explain Their
  Robustness over Time." PACLIC 2024. https://aclanthology.org/2024.paclic-1.94.pdf
- Malhotra A, Kamle S. "ARTH: Algorithm For Reading Text Handily." arXiv:2101.09464
  (preprint, not peer reviewed, numbers not relied on). https://arxiv.org/pdf/2101.09464
