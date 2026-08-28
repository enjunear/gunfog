# Gunning Fog vs Flesch-Kincaid for gfog

Research ticket: how do the two formulas differ, and should gfog prefer one, or support both?

Date: 2026-08-28. Sources are primary where primary text was reachable: Gunning's own book,
the original Navy report that defines Flesch-Kincaid, library source code, and statute text.
Where a claim rests on a secondary review it says so.

## Verdict

**Important finding, and it is not the one the question expects.**

Fog-only survives on the merits. Gunning Fog measures the right thing for this corpus and its
author aimed it squarely at business, government and technical report writing. But four things
turned up that should change the design around the formula:

1. **Two implementation choices each move the score more than the choice of formula does.** How gfog
   segments a bulleted list into sentences shifts the grade by **3.9 grades on average**, eight
   times the effect of swapping a dictionary syllable counter for a heuristic (0.49), and more than
   twice the entire spread between documents (sd 1.79). It hits Fog and FK almost identically (3.94
   vs 3.84), so it is not a formula question. Markdown is mostly bullets. This decision *is* the
   tool.

2. **The formula choice barely matters; the per-sentence attribution method matters enormously.**
   On 23,728 sentences of real agent-written markdown, the naive "score each sentence on its own"
   hotspot ranking that the current design implies overlaps the *correct* attribution by only
   **14 of 100** sentences for Fog. Naive per-sentence Fog is not a weak signal for what drives a
   document's score. It is close to the wrong signal, and it is wrong in a specific, embarrassing
   direction: it flags **6-word sentences** as the worst offenders. Fix the attribution and the two
   formulas agree on 75 of 100 hotspots anyway. This is the finding that should change code.

3. **Gunning Fog has no reference implementation and no institutional standing; Flesch-Kincaid has
   both.** Every library surveyed implements a *different* Gunning Fog, because each picks a
   different subset of Gunning's exclusion rules. Flesch-Kincaid is unambiguous, is what Microsoft
   Word reports, and is the metric Red Hat's documentation linter actually checks. If gfog's number
   is meant to be quotable or comparable, Fog-only costs something real.

4. **Both formulas are noise below roughly 200 words.** At 25 words, a typical commit message,
   sampling noise is over twice the between-document spread. gfog should refuse to score short
   inputs rather than print a confident grade for them.

Recommendation: keep Gunning Fog as the headline number, then spend the effort where it actually
moves the number. That means sentence segmentation for markdown, leave-one-out hotspot attribution,
an explicit complex-word policy, and a minimum input length. Expose Flesch-Kincaid as a secondary
number; it is about five lines once Fog exists, since both need the same syllable counter.

A caveat worth carrying into the design: in the one study that checked these formulas against
*reader-perceived* difficulty rather than against each other, both correlated weakly. FKGL scored r ≈ 0.30
and 0.18, Gunning Fog r ≈ 0.13 (PMC5355629, section 7). The formulas are a smoke alarm, not a
thermometer. Gunning said the same thing in 1968: "a tool, not a rule."

---

## 1. The two formulas side by side

### Gunning Fog, from Gunning's own text

Robert Gunning, *The Technique of Clear Writing*, revised edition 1968, pp. 38-39
([full text, Internet Archive](https://archive.org/details/the-technique-of-clear-writing-revised-edition-1968)).
Verbatim:

> **One:** Jot down the number of words in successive sentences. If the piece is long, you may wish
> to take several samples of 100 words, spaced evenly through it. [...] Divide the total number of
> words in the passage by the number of sentences. This gives the average sentence length of the
> passage.
>
> **Two:** Count the number of words of three syllables or more per 100 words. Don't count the words
> (1) that are proper names, (2) that are combinations of short easy words (like "bookkeeper" and
> "manpower"), (3) that are verb forms made three syllables by adding -ed or -es (like "created" or
> "trespasses"). This gives you the percentage of hard words in the passage.
>
> **Three:** To get the Fog Index, total the two factors just counted and multiply by .4.

So `Fog = 0.4 x (words/sentence + 100 x complex/words)`.

Note there are **three** exclusions in Gunning's own words, not the four usually quoted. The
"familiar jargon" exclusion that circulates widely is not in this passage. Gunning's attitude to
jargon is the opposite of forgiving: the book's worked examples treat jargon as the disease the
index detects, and he singles out the National Electrical Code as "probably the foggiest writing
found in business and industry" precisely because "the jargons of engineers, lawyers, and government
officials mingle" in it (p. 238).

Validation and intended domain: Gunning calibrated against the McCall-Crabbs *Standard Test Lessons
in Reading* and against the average sentence length and hard-word percentage of magazines with known
circulation (pp. 37-38). His audience was working writers. The preface records that half the
thousands of people he trained "were not professional writers but persons who had to write as part
of their professions or specialties, in business, industry, government, or the armed forces" (p. xv).
He also flags the misuse he expects:

> we emphasize that the Fog Index is a tool, not a rule. It is a warning system, not a formula for
> writing. Testing without the support of analysis based on experience can be detrimental. (p. xiii)

His stated danger line is 13; he says anyone writing above 12 "is putting his communication under a
handicap" (p. 39).

### Flesch-Kincaid, from the report that defines it

Kincaid, Fishburne, Rogers & Chissom, *Derivation of New Readability Formulas (Automated Readability
Index, Fog Count and Flesch Reading Ease Formula) for Navy Enlisted Personnel*, Research Branch
Report 8-75, Chief of Naval Technical Training, February 1975
([DTIC ADA006655, full text](https://archive.org/details/DTIC_ADA006655);
[ERIC record](https://eric.ed.gov/?id=ED108134);
[UCF STARS record](https://stars.library.ucf.edu/istlibrary/56/)).

Table 3, verbatim:

> Flesch Reading Ease Formula
> Old: RE = 206.835 - 1.015 (words/sentence) - .836 (syllables/100 words)
> **New: GL = .39 (words/sentence) + 11.8 (syllables/word) - 15.59**
> Simplified: GL = .4 (words/sentence) + 12 (syllables/word) - 16

Validation: 531 Navy enlisted personnel at four technical training schools, reading level measured by
the comprehension section of the Gates-MacGinitie Reading Test, comprehension of 18 passages drawn
from Rate Training Manuals measured by cloze test, with a passage assigned a grade level when 50% of
subjects reading at that level scored 35% or better. Scaled reading grade levels of the 18 passages
ranged 5.5 to 16.3.

The original validation domain was therefore **military technical documentation read by its actual
technicians**, closer to gfog's corpus than Gunning's magazines, and worth knowing.

### The same report also recalibrated a Fog variant

Table 3 also gives a "Fog Count", which is *not* Gunning Fog: hard words are simply words of more
than two syllables with no exclusions, and the formula is
`GL = ((easy words + 3 x hard words) / sentences) / 2 - 3`. Useful context, not something gfog should
implement, but it means the 1975 report compared a Fog-family measure and Flesch-Kincaid head to head
on the same data.

---

## 2. Why the two numbers differ, and by how much

### They do not share a zero point

The single best explanation for the systematic gap is the criterion score each author chose, meaning the
comprehension level the formula is predicting. From William DuBay, *The Principles of Readability*
(2004), p. 62 ([ERIC ED490073, full text](https://files.eric.ed.gov/fulltext/ED490073.pdf)):

> The formulas — like reading tests — simply do not have a common zero point (Klare 1982). The
> criterion score is the required level of comprehension indicating reading success [...] The FORCAST
> and Dale-Chall formula use a 50% criterion score as measured by multiple-choice tests. The Flesch
> formula use a 75% score, **Gunning Fog formula, a 90% score**, and the McLaughlin SMOG formula a
> 100% score. The formulas developed with the higher criterion scores tend to predict higher scores.

DuBay gives a worked example on the same four paragraphs: Flesch grade 8.9, Fog grade 12.3 (p. 62).

So Fog reads high *by construction*. A Fog of 12 and an FK of 9 can be the same text meeting the same
standard of clarity. Fog is asking "who reads this with 90% comprehension", FK is asking "who reads
it with 75%". Any threshold gfog ships has to be calibrated to whichever formula it prints, and the
two thresholds are not interchangeable.

### Measured on agent-written markdown

I ran both formulas over a corpus of **310 documents / 25,605 sentences / 323,730 words** of real
agent-written markdown from `~/.claude`: merge request descriptions, code review notes, issue text,
agent skills and instructions. Code fences, indented blocks, inline code, URLs, link targets and any
line still carrying code-like tokens were stripped first. Syllables come from the CMU Pronouncing
Dictionary with a vowel-group heuristic fallback for out-of-vocabulary words.

The Fog implementation reproduces Gunning's own worked example (the Maugham passage, p. 38-39)
exactly: 118 words, 8 sentences, 15 hard words = 12.7%, Fog 11.0 against his reported 10.9.

| Measure | Value |
|---|---|
| Pearson r(Fog, FK), document level | **0.9256** |
| Spearman rank r(Fog, FK) | 0.9248 |
| Mean Fog | 12.25 (sd 1.79) |
| Mean FK | 9.62 (sd 1.48) |
| Mean Fog − FK | **+2.63** (sd 0.70, range 1.0 to 5.0) |
| Corpus mean sentence length | 12.7 words |
| Corpus mean syllables/word | 1.72 |
| Corpus mean complex words | 17.9% |

The +2.63 offset matches DuBay's criterion-score explanation and his worked example's +3.4.

The correlation is high but not interchangeable. If gfog fails the worst *N%* of documents:

| Gate | Documents both formulas fail | Disagreements |
|---|---|---|
| worst 5% | 9/15 (60%) | 6 |
| worst 10% | 20/31 (65%) | 11 |
| worst 20% | 51/62 (82%) | 11 |
| worst 50% | 140/155 (90%) | 15 |

At a strict gate, a third to 40% of the documents flagged differ depending on the formula. That is
worth knowing, but it is an argument for not over-claiming precision, not an argument for either
formula.

---

## 3. Hotspot attribution, the actual finding

gfog's whole reason for existing is the hotspot list: "these sentences drove your score up, here are
their complex words." Getting that right is more important than the formula.

### Naive per-sentence scoring is wrong, and worst for Fog

Both formulas are of the shape `a x (mean sentence length) + b x (word-difficulty ratio) + c`, where
the word-difficulty ratio is a **ratio of sums over the whole document**, not a mean of per-sentence
ratios. Scoring a sentence in isolation therefore does not decompose the document score. It computes
a different quantity.

For Fog specifically the isolated score is pathological on short sentences. In an *n*-word sentence,
the word term `40c/n` can only take multiples of `40/n`:

| Sentence length | One complex word moves isolated Fog by | One syllable moves isolated FK by |
|---|---|---|
| 5 words | **8.00 grades** | 2.36 |
| 8 words | 5.00 | 1.48 |
| 10 words | 4.00 | 1.18 |
| 15 words | 2.67 | 0.79 |
| 25 words | 1.60 | 0.47 |
| 40 words | 1.00 | 0.30 |

Because Fog's word term is a *percentage* scaled by 0.4 it spans 0-40, while FK's is
syllables-per-word scaled by 11.8 and realistically spans about 18-25. Fog's per-sentence word term
is roughly 3.4x more violent at short lengths.

The consequence is visible in the corpus. Median sentence length among the top-100 hotspots:

| Ranking method | Median length of top-100 |
|---|---|
| naive per-sentence Fog | **7 words** |
| naive per-sentence FK | **57 words** |
| leave-one-out Fog | 26 words |
| leave-one-out FK | 27 words |
| (corpus median) | 11 words |

Naive Fog flags terse sentences; naive FK flags long ones. Neither is measuring contribution. Real
examples of top-ranked naive-Fog "hotspots" from the corpus:

- `Production debugging (use targeted debugging instead).` Fog 29.1, 6 words
- `Performance comparison of picomatch and minimatch.` Fog 29.1, 6 words
- `Enable or disable existing hookify rules using an interactive interface.` Fog 29.1, 10 words

Telling an agent to "revise this sentence" about a six-word bullet is noise, and worse, it teaches
the agent that short sentences are the problem.

### Leave-one-out attribution fixes it, for both formulas

The correct question is *how far does the document score fall if this sentence is fixed*. Compute
the document score, then recompute with the sentence removed; the drop is that sentence's
contribution. Results:

| Comparison | Top-100 overlap | Pearson r |
|---|---|---|
| naive Fog vs leave-one-out Fog | **14/100** | 0.717 |
| naive FK vs leave-one-out FK | 25/100 | 0.716 |
| naive Fog vs naive FK | 59/100 | n/a |
| **leave-one-out Fog vs leave-one-out FK** | **75/100** | **0.894** |

Two things follow. The naive ranking agrees with the correct one on 14% of the top 100. It is not a
usable approximation. And once attribution is done properly the formula choice mostly stops
mattering, which is a further reason not to agonise over Fog vs FK.

Leave-one-out is O(n) with running totals: keep document `W`, `S`, `C`, `SY`, and for each sentence
recompute the score from `(W - w_i, S - 1, C - c_i)`. No extra passes.

### Where Fog does win for hotspots

Fog's word term counts a **discrete, nameable set**: you can literally print "the complex words in
this sentence are X, Y, Z" and the agent can act on it. FK's word term is a continuous syllable
total; to explain it you would have to show per-word syllable counts, and to make it actionable you
would have to threshold them, at which point you have reinvented Fog's complex-word list. This is a
genuine advantage for the "with their complex words" part of the spec, and it holds regardless of
the attribution fix.

---

## 4. Implementability and syllable counting

### Measured sensitivity to heuristic syllables, where the intuition is wrong

The ticket asks which formula degrades more gracefully with an imperfect syllable counter, on the
theory that Fog's binary 3+ call is more forgiving than FK's total. Measured on the corpus,
comparing a pure vowel-group heuristic against CMUdict ground truth:

| | Fog | Flesch-Kincaid |
|---|---|---|
| mean absolute score shift | **0.490 grades** | **0.415 grades** |
| p95 score shift | 1.306 | 0.916 |
| shift as % of that formula's document sd | 27.4% | 28.0% |
| r(dictionary, heuristic) | 0.967 | **0.982** |

Underlying error rates: mean absolute syllable error 0.065 per in-dictionary word; the 3+/not-3+
verdict flips for **2.14%** of in-dictionary words.

Flesch-Kincaid is marginally *more* robust in absolute terms and preserves document ranking better.
The reason is that a threshold is a cliff: each flip costs Fog `40/W` of the document score with no
partial credit, whereas FK's syllable errors are signed and cancel across the document. Normalised
to each formula's own spread the two are a dead heat (27.4% vs 28.0%). **Neither formula has a
meaningful robustness advantage, and the assumed advantage for Fog does not exist.** Do not choose
Fog on this basis.

What *does* matter: **5.45% of tokens in this corpus are absent from CMUdict**, even after stripping
code. Whatever gfog does for out-of-vocabulary words dominates both formulas' error.

### There is no reference Gunning Fog

This is the strongest implementability argument against Fog-only, and it came out of reading library
source rather than docs. Every implementation picks a different subset of Gunning's exclusions:

| Implementation | Complex-word definition |
|---|---|
| [textstat](https://github.com/textstat/textstat) (`backend/metrics/_gunning_fog.py`) | Reuses the **Dale-Chall** difficult-word test with the syllable cutoff raised to 3: not in the easy-word list *and* 3+ syllables. **None** of Gunning's exclusions. |
| [quanteda.textstats](https://github.com/quanteda/quanteda.textstats) (`R/textstat_readability.R`) | `n_syll(word) >= 3`, no exclusions at all. |
| [py-readability-metrics](https://github.com/cdimascio/py-readability-metrics) | 3+ syllables and not proper noun (`token[0].isupper()`) and not compound (`.*[-].*`). Two of three exclusions. |
| [jdkato/prose](https://github.com/jdkato/prose) (used by Vale) | Decrements the syllable count for `es`/`ed`/`ing` suffixes, then `> 2`. The suffix exclusion only. |
| [readability-scores](https://github.com/words/gunning-fog) (npm) | 3+ syllables, with an **optional, off-by-default** `capsAsNames` proper-noun exclusion. |

textstat's substitution is a known, open gap, acknowledged in its own tracker:
[issue #73](https://github.com/textstat/textstat/issues/73) states outright that its difficult-word
definition "is not true for Gunning Fog", and
[issue #150](https://github.com/textstat/textstat/issues/150) tracks the missing inflection handling.
The `words/gunning-fog` README is candid about why: Gunning Fog is "hard to implement with a computer
(needs POS-tagging and Named Entity Recognition)".

No library implements all of Gunning's rules. Two of the three need real NLP to do properly: "proper
names" needs named-entity recognition, and "combinations of short easy words" needs compound
splitting. gfog will be *a* Gunning Fog, not *the* Gunning Fog, and its number will not match
textstat's. That is fine if documented and bad if not.

Flesch-Kincaid has no such ambiguity. `0.39 x ASL + 11.8 x SPW - 15.59` is fully specified; the only
freedom is the syllable counter, which Fog needs anyway.

Measured cost of the exclusions on this corpus: applying an approximation of Gunning's proper-noun
and `-ed`/`-es` rules removes **12.4%** of raw 3+-syllable words and drops the mean Fog by 0.86
grades, with r = 0.951 against unexcluded Fog. So the exclusions matter enough to change a threshold
verdict but not enough to reorder documents much. Whichever gfog picks, it should say so in the
output.

### Syllable counting has no citable accuracy figure

The heuristic everything descends from is Greg Fast's 1998 Perl `Lingua::EN::Syllable`, whose author
is the only source giving a real number, and it is his own informal one
([source](https://metacpan.org/release/GREGFAST/Lingua-EN-Syllable-0.251/source/Syllable.pm)):

> it isn't entirely accurate... it fails (by one syllable) for about 10-15% of my `/usr/dict/words`.
> The only way to get a 100% accurate count is to do a dictionary lookup

The "95% accurate" figures that circulate are not attributable to any described corpus or method.
gfog's docs should not repeat them. My own measurement above (mean absolute error 0.065
syllables/word, 2.14% of words flipping the 3+ verdict, on English prose after code stripping) is
the concrete number for this corpus.

Practical notes from library source: textstat is CMUdict-first with Pyphen as fallback, but Pyphen is
Liang hyphenation-pattern data built for line breaking, not phonetic syllabification. That is a silent
approximation for exactly the technical vocabulary gfog cares about. No surveyed tokenizer splits
`camelCase` or `snake_case` before counting; identifiers get counted as one long pseudo-word and
almost always land as "complex". All the heuristics floor vowel-less strings like `SQL` and `HTTP` at
one syllable.

The practical consequence for gfog: **the markdown pre-pass matters more than the formula.** My first
run of this experiment used weak code stripping, and the top hotspots under both formulas were link
anchors and API signatures. `SourceMapGenerator.prototype.applySourceMap(...)` scored FK 84.9. Strip
code fences, indented blocks, inline code, URLs, link targets and identifier-shaped tokens before
scoring anything.

---

## 5. Sentence segmentation in markdown, the largest single lever

Janice Redish's standing objection to readability formulas is that they "penalize bulleted lists and
white space — both of which help real readers — because they count periods as sentence boundaries"
([Redish & Jarrett, *UXmatters*, 2019](https://www.uxmatters.com/mt/archives/2019/07/readability-formulas-7-reasons-to-avoid-them-and-what-to-do-instead.php);
see also [Redish, *ACM Journal of Computer Documentation* 24(3), 2000](https://dl.acm.org/doi/10.1145/344599.344637)).
For a tool whose input is mostly markdown, this stops being a philosophical objection and becomes the
main implementation decision. So I measured it.

Two defensible policies over 627 documents:

- **Split.** Each bullet is its own sentence. What a naive splitter produces once list items get
  terminal periods.
- **Joined.** A bullet list plus its lead-in line is one sentence. Closer to what the prose is
  syntactically, since list items are usually clause fragments completing a stem.

| | Split | Joined | Delta |
|---|---|---|---|
| Mean Fog | 13.33 | 17.27 | **+3.94** |
| Mean FK | 10.80 | 14.64 | **+3.84** |

Mean absolute shift per document: Fog 3.94 grades (p95 10.43), FK 3.84 (p95 10.12). Splitting bullets
raises the sentence count by 1.66x on average.

Put that next to the other effects measured here:

| Design decision | Mean effect on the score |
|---|---|
| **Bullet segmentation policy** | **3.94 grades** |
| Gunning's exclusion rules on/off | 0.86 grades |
| Dictionary vs heuristic syllables | 0.49 grades |
| *(between-document spread, for scale)* | *sd 1.79* |

The segmentation choice is eight times the syllable-counter choice and more than twice the spread
gfog is trying to measure. It moves Fog and FK by the same amount, so it is not an argument for
either. It is an argument that the formula debate is the small half of this problem.

This is the same failure mode Language Log documented for prose: Mark Liberman re-punctuated an
identical transcript, changing only where sentence breaks fell, and the Flesch-Kincaid grade moved
between 4.4 and 12.5 ([Language Log](https://languagelog.ldc.upenn.edu/nll/?p=21847)). In the same
series he ROT13-enciphered a German poem into gibberish and the score barely moved, 4.1 to 3.9
([Language Log](https://languagelog.ldc.upenn.edu/nll/?p=15456)). The formulas count characters and
boundaries, not comprehension.

gfog must pick a policy, document it, and hold it stable, because a score that moves four grades on
formatting is worse than no score. Concrete suggestions: parse markdown properly rather than
line-splitting; treat a list item as a clause joined to its lead-in stem rather than a sentence;
exclude headings, tables and single-fragment lines from the sentence count; and never let a
reformatting-only diff change the reported grade. That last one is testable and would make a good
regression test.

## 6. Score stability on short inputs

The spec includes commit messages. They are too short to score. Bootstrap over 4,000 draws of
contiguous sentences from the corpus:

| Target words | Sentences | Fog mean | Fog sd | FK mean | FK sd |
|---|---|---|---|---|---|
| 25 | 2.7 | 12.71 | **3.94** | 10.23 | **3.38** |
| 50 | 4.7 | 12.49 | 2.80 | 10.00 | 2.42 |
| 100 | 8.7 | 12.25 | 1.97 | 9.74 | 1.67 |
| 200 | 16.6 | 12.15 | 1.34 | 9.65 | 1.09 |
| 400 | 32.6 | 12.06 | 0.93 | 9.58 | 0.76 |
| 1000 | 81.0 | 12.02 | 0.59 | 9.54 | 0.48 |

Expressed against the between-document spread (Fog sd 1.79, FK sd 1.48), sampling noise is 212% of
it at 25 words, 111% at 100 words, 75% at 200, and 52% at 400. The ratio Fog sd / FK sd is a flat
1.17-1.24 across every size, which is just the ratio of their document spreads. **Neither formula is
relatively noisier.**

Note that even at Gunning's own recommended 100-word sample, noise equals the entire spread between
documents. Gunning got away with this because he was sampling several 100-word blocks from one long
document, not scoring 100-word documents.

Corroborating precedent: `py-readability-metrics` hard-raises `ReadabilityException('100 words
required.')` below 100 words, and `retext-readability` guards short sentences with a `minWords`
option (default 5) because "for short sentences, one long or complex word can strongly skew the
results".

gfog should refuse to print a grade below roughly 200 words, or print it with an explicit
uncertainty. A commit message will essentially never qualify. For those, hotspot-style advice
("this sentence has 6 complex words") is honest where a grade number is not.

---

## 7. Institutional standing

This is where the two diverge most sharply, and it cuts against Fog.

**Flesch-Kincaid has real standing.**

- Microsoft Word reports Flesch Reading Ease and Flesch-Kincaid Grade Level, with the formulas
  published by name ([Microsoft support](https://support.microsoft.com/en-us/office/get-your-document-s-readability-and-level-statistics-85b4969e-e80a-4777-8dd3-f7fc3c8b3fd2)).
  It is the number most people have actually seen.
- Red Hat's documentation linter checks it explicitly. From
  [vale-at-red-hat](https://raw.githubusercontent.com/redhat-documentation/vale-at-red-hat/main/.vale/styles/RedHat/ReadabilityGrade.yml):
  `extends: readability`, `grade: 9`, `metrics: [Flesch-Kincaid]`, `level: suggestion`. This is the
  closest thing to a developer-documentation standard anyone found, and it names FK.
- The US DoD authorised it in 1978 for validating readability of technical manuals for the armed
  services, and the IRS and Social Security Administration issued similar directives (DuBay 2004,
  p. 50, [ED490073](https://files.eric.ed.gov/fulltext/ED490073.pdf)). The frequently cited
  MIL-STD-1685 could not be verified against primary text.
- Flesch **Reading Ease** is written into state law. Texas requires a score of 40+ for a policy form
  to qualify as plain language ([Tex. Ins. Code § 2301.053](https://law.justia.com/codes/texas/insurance-code/title-10/subtitle-i/chapter-2301/subchapter-b/section-2301-053/));
  New York ([Ins. Law § 3102](https://law.justia.com/codes/new-york/isc/article-31/3102/)) and
  Florida ([§ 627.4145](https://codes.findlaw.com/fl/title-xxxvii-insurance/fl-st-sect-627-4145/))
  require 45.

**Gunning Fog has none.** No government mandate, no official style guide, no standards body naming
it. It is the default proxy in academic finance research on 10-K readability, and it is common in
blog posts and web checkers, but nothing institutional requires or recommends it.

**And the plain-language establishment recommends neither.** This is worth stating plainly because it
tempers any appeal to authority in either direction:

- The [Plain Writing Act of 2010](https://www.congress.gov/bill/111th-congress/house-bill/946/text)
  names no formula.
- The [Federal Plain Language Guidelines](https://wid.org/wp-content/uploads/2022/03/FederalPLGuidelines.pdf)
  name no formula and set no numeric target. Every occurrence of "Flesch" in that document cites
  Rudolf Flesch's 1979 prose advice, not a formula. (Search snippets claiming it sets an FK target of
  10 are wrong.)
- plainlanguage.gov hosts ["Revisiting Plain Language"](https://www.plainlanguage.gov/resources/articles/revisiting-plain-language/),
  which says formulas are "of questionable validity" and that plain-language resources mentioning
  them "do not recommend their use".
- The SEC's 1998 *Plain English Handbook* (p. 57), quoted in that article: "you should be aware of a
  major flaw in every readability formula. No formula takes into account the content of the document
  being evaluated [...] The final test of whether any piece of writing meets its goal of communicating
  information comes when humans read it."
- Current GOV.UK content-design guidance names no formula and sets no numeric reading age; it
  mentions the Hemingway app once, for checking active voice. The widely quoted "reading age 9"
  target propagates through downstream manuals such as the
  [Home Office design manual](https://design.homeoffice.gov.uk/accessibility/written-content/readability),
  not the live GOV.UK pages.

So the lecturer's recommendation is not contradicted by any standard. It is simply not backed by one
either, and the competing formula is.

---

## 8. Known criticisms that apply to gfog

**Shared by both formulas.** They use only surface features and ignore content, organisation and
coherence. DuBay (2004, p. 34) summarises the critics fairly (Manzo 1970, Bruce et al. 1981, Selzer
1981, Redish & Selzer 1985, Schriver 2000) and also the defence, that surface features "with all
their limitations have remained the best predictors of text difficulty as measured by comprehension
tests". The canonical statements of the critique are
[Bruce, Rubin & Starr, "Why readability formulas fail", *IEEE Trans. Prof. Comm.* PC-24(1), 1981](https://eric.ed.gov/?id=ED205915)
and [Davison & Kantor, *Reading Research Quarterly* 17(2), 1982](https://eric.ed.gov/?id=EJ257811),
the latter showing that texts deliberately rewritten to hit a target formula score did not reliably
become more comprehensible and sometimes became less coherent. That is precisely the loop gfog puts
an agent into, so it is the risk to design against.

**They correlate weakly with what readers actually find hard.** The most consequential empirical
finding in this whole area is not how the two formulas relate to each other but how poorly both
relate to human judgement. A study of 140 Wikipedia diabetes articles and 242 de-identified EHR
notes, rated by 15 human raters, found FKGL correlated r = 0.30 (medical) and 0.18 (Wikipedia) with
perceived difficulty, SMOG r = 0.10, and Gunning Fog r = 0.13
([PMC5355629](https://pmc.ncbi.nlm.nih.gov/articles/PMC5355629/)). Readers rated the EHR notes 21.3%
*harder* than the Wikipedia articles despite the notes scoring markedly easier on FKGL (9.87 vs
14.75).

The mechanism behind that inversion matters for gfog: **short jargon is invisible to both formulas.**
`CHF`, `EKG`, or `CI`, `PR`, `diff`, `repo`, `stub`, `hook`, are one or two syllables, so neither
the complex-word count nor the syllable ratio registers them, however much specialist knowledge they
demand. Meanwhile ordinary long words that a technical reader finds trivial (`configuration`,
`repository`, `authentication`, `idempotent`) are penalised by both. Agent-written prose is dense in
both categories, so gfog's score will be wrong in both directions at once. Neither formula has a word
familiarity or frequency signal; the Dale-Chall family does, which is a possible future direction but
not this ticket.

**Gaming by cutting sentences.** The Flesch-Kincaid authors flagged this themselves in 1975: "It is
fairly simple to simply cut sentences in two if a formula contains sentence length as a factor. The
word length factor is much more difficult to mechanically manipulate." Both Fog and FK carry sentence
length, so both are gameable this way, and an LLM told to lower a Fog score is a *very* efficient
sentence-splitter. This is a real risk for gfog specifically. The naive per-sentence hotspot ranking
makes it worse, because it rewards the agent for producing exactly the short sentences the correct
attribution would ignore.

**Precision claims.** Also from the 1975 report: "readability formulas are only accurate to within
one grade level." gfog should not print two decimal places.

**Technical vocabulary is over-penalised, and the FK authors said so.** From the same report, on
their motivation:

> It appears likely that the repeating of technical words in Navy technical material would tend to
> inflate the computed readability level beyond the actual difficulty experienced by the technician
> or technical training student reading the material.

Their data bore it out: the recalculated formulas score Navy material about a grade lower than the
originals, "consistent with the prediction that Navy personnel familiar with the vocabulary of Navy
training documents should understand this material better than other types of narrative material."
This is the strongest single caution for gfog. Its readers are coding agents and engineers, for whom
"authentication", "repository", "idempotent" and "configuration" are not hard words, yet all four
count as complex under Fog and inflate syllables under FK. Gunning's proper-noun and compound
exclusions blunt this slightly; his rules contain nothing that spares familiar domain terms.

Leon Hull argued at the 1979 STC conference that technical writing needs a formula without a word
length variable at all, and produced one using sentence length plus modifier density (DuBay 2004,
p. 51). Not a proposal for gfog, but it shows the concern is old and specific to this domain.

**Literature correlations.** The 1975 report is the cleanest primary comparison available: on its 18
test passages, the recalculated Fog Count correlates **.90** with the recalculated Flesch-Kincaid,
which the authors read as showing the Fog family "measures much the same thing" as Flesch. My
measurement on agent markdown, r = 0.926, lands in the same place fifty years later on a very
different corpus.

The systematic offset also replicates in the modern literature. A study of 101 blood-cancer patient
education documents reports **mean FKGL 9.60 against mean Gunning Fog 12.69**, a gap of roughly three
grades, significant at p < 0.001 ([PMC11101262](https://pmc.ncbi.nlm.nih.gov/articles/PMC11101262/)).
That independently reproduces both the direction and the rough size of the +2.63 I measured, and
matches DuBay's criterion-score explanation. Two published corpora and mine now agree: Fog reads
about 2.5 to 3.5 grades above Flesch-Kincaid on the same text. Any threshold gfog ships must be set
for the formula it prints.

A caution on one figure that circulates: Kurdi (2020) is often cited for r = −0.859 between Gunning
Fog and "Flesch-Kincaid" on 6,171 ESL texts ([arXiv](https://arxiv.org/pdf/2001.01863)). The negative
sign is almost certainly Flesch **Reading Ease** mislabelled. The paper explains it by the 206.83
constant, which belongs to Reading Ease, not to the grade-level formula. Two grade-level formulas
built from the same ingredients cannot correlate negatively. Take the magnitude, not the sign, and
prefer the 1975 report's .90.

---

## 9. What this means for gfog

Ordered by how much they should change the code.

1. **Decide and freeze the markdown sentence-segmentation policy.** Worth 3.9 grades, the largest
   effect measured here. Parse markdown rather than line-splitting; decide explicitly whether a list
   item is a sentence or a clause joined to its lead-in; exclude headings and table cells. Add a
   regression test asserting that a reformatting-only diff does not change the grade.
2. **Replace naive per-sentence hotspot scoring with leave-one-out contribution.** Naive ranking
   matches correct ranking on 14 of 100 sentences and systematically flags 6-word bullets.
   Leave-one-out is O(n) with running totals.
3. **Refuse to print a grade below ~200 words**, or print an explicit uncertainty. At commit-message
   length the number is noise. Hotspots and complex-word lists are still honest at that size; the
   grade is not.
4. **Strip aggressively before scoring.** Code fences, indented blocks, inline code, URLs, link
   targets, and identifier-shaped tokens. This affects the score more than the formula does. An
   unstripped API signature scored FK 84.9 in my first run.
5. **Pick and document one complex-word policy.** There is no canonical Gunning Fog. Say which of
   Gunning's three exclusions gfog applies (the `-ed`/`-es` rule is cheap and worth doing; proper
   nouns via capitalisation is crude but real; compound splitting probably is not worth it), and say
   gfog's number will not match textstat's.
6. **Keep Fog as the headline, add Flesch-Kincaid as a secondary number.** Both need the same
   syllable counter, so FK costs about five lines. Fog's complex-word list is the better hotspot
   explanation and its domain of origin fits; FK is what Word shows, what Red Hat's linter checks,
   and the only one with institutional standing. Printing both also stops any single number being
   over-trusted, which is what Gunning asked for in the first place: "a tool, not a rule".
7. **Do not choose Fog for robustness to heuristic syllable counting.** That advantage does not
   exist; measured, FK is if anything slightly better. Choose Fog for the complex-word list and the
   domain fit.
8. **Watch for sentence-splitting as a degenerate fix.** Both formulas reward it and the FK authors
   warned about it in 1975. Consider warning when a revision lowers the score mainly by raising
   sentence count.

---

## Appendix: reproducing the measurements

Scripts live in the session scratchpad, not the repo:
`/tmp/claude-1000/-home-michael-g-fog/51f47d99-c78d-4ccc-99da-6affe6022c3a/scratchpad/`: `exp.py`
core metrics, `exp3.py` clean-corpus comparison, `exp4.py` hotspot attribution, `exp5.py` stability
bootstrap, `exp6.py` gate agreement, `exp7.py` bullet segmentation, `validate.py` Gunning
worked-example check. They are throwaway; if any of this needs to hold up over time it should be
rebuilt as tests against gfog itself.

Corpus: `~/.claude/**/*.md`, holding merge request descriptions, review notes, issue text, agent skills and
instructions. 310 documents with 300+ prose words after stripping (627 for the segmentation test,
which strips less). Syllables from `cmudict` (126,052 entries) with a vowel-group fallback.

Caveats on my own numbers. This is one corpus from one machine and one agent harness, so treat the
absolute means as indicative and the *comparisons* between conditions as the result. CMUdict is the
ground truth for syllables, which is itself imperfect for technical vocabulary. The Gunning-exclusion
approximation uses capitalisation for proper nouns and a stem check for `-ed`/`-es`; it does not
attempt compound splitting. The sentence splitter is a regex, which is exactly the component section
5 argues should be a markdown parser, so the segmentation effect I measured is, if anything,
understated. Fog was validated against Gunning's own worked example (1968, pp. 38-39): 118 words,
8 sentences, 15 hard words at 12.7%, Fog 11.0 against his published 10.9.

## Sources

Primary:
- Gunning, *The Technique of Clear Writing*, rev. ed. 1968. https://archive.org/details/the-technique-of-clear-writing-revised-edition-1968
- Kincaid, Fishburne, Rogers & Chissom 1975, Research Branch Report 8-75 (DTIC ADA006655). https://archive.org/details/DTIC_ADA006655
- ERIC record for the above. https://eric.ed.gov/?id=ED108134
- UCF STARS record. https://stars.library.ucf.edu/istlibrary/56/
- Plain Writing Act of 2010. https://www.congress.gov/bill/111th-congress/house-bill/946/text
- Federal Plain Language Guidelines (2011 rev. 1). https://wid.org/wp-content/uploads/2022/03/FederalPLGuidelines.pdf
- Tex. Ins. Code § 2301.053. https://law.justia.com/codes/texas/insurance-code/title-10/subtitle-i/chapter-2301/subchapter-b/section-2301-053/
- N.Y. Ins. Law § 3102. https://law.justia.com/codes/new-york/isc/article-31/3102/
- Fla. Stat. § 627.4145. https://codes.findlaw.com/fl/title-xxxvii-insurance/fl-st-sect-627-4145/

Peer-reviewed studies:
- Blood-cancer patient education readability, FKGL 9.60 vs GFI 12.69. https://pmc.ncbi.nlm.nih.gov/articles/PMC11101262/
- Formula scores vs reader-perceived difficulty, Wikipedia and EHR notes. https://pmc.ncbi.nlm.nih.gov/articles/PMC5355629/
- Bruce, Rubin & Starr, "Why readability formulas fail", 1981. https://eric.ed.gov/?id=ED205915
- Davison & Kantor, "On the Failure of Readability Formulas to Define Readable Texts", 1982. https://eric.ed.gov/?id=EJ257811
- Redish, "Readability formulas have even more limitations than Klare discusses", 2000. https://dl.acm.org/doi/10.1145/344599.344637
- Redish & Jarrett, "Readability Formulas: 7 Reasons to Avoid Them", 2019. https://www.uxmatters.com/mt/archives/2019/07/readability-formulas-7-reasons-to-avoid-them-and-what-to-do-instead.php
- Kurdi 2020, ESL corpus formula correlations (sign of the Fog/FK figure is suspect, see section 8). https://arxiv.org/pdf/2001.01863
- Liberman, Language Log: punctuation swings FK from 4.4 to 12.5. https://languagelog.ldc.upenn.edu/nll/?p=21847
- Liberman, Language Log: ROT13 gibberish scores the same as the original. https://languagelog.ldc.upenn.edu/nll/?p=15456

Authoritative reference:
- DuBay, *The Principles of Readability*, 2004 (ERIC ED490073). https://files.eric.ed.gov/fulltext/ED490073.pdf
- Microsoft, readability statistics in Word. https://support.microsoft.com/en-us/office/get-your-document-s-readability-and-level-statistics-85b4969e-e80a-4777-8dd3-f7fc3c8b3fd2
- plainlanguage.gov, "Revisiting Plain Language" (quotes the SEC Plain English Handbook). https://www.plainlanguage.gov/resources/articles/revisiting-plain-language/
- GOV.UK writing standards. https://guidance.publishing.service.gov.uk/writing-to-gov-uk-standards/writing-guidelines/clear-language/
- Home Office User-Centred Design Manual, readability. https://design.homeoffice.gov.uk/accessibility/written-content/readability

Implementations read as source:
- textstat. https://github.com/textstat/textstat (issues [#73](https://github.com/textstat/textstat/issues/73), [#150](https://github.com/textstat/textstat/issues/150))
- quanteda.textstats. https://github.com/quanteda/quanteda.textstats
- py-readability-metrics. https://github.com/cdimascio/py-readability-metrics
- jdkato/prose. https://github.com/jdkato/prose
- words/gunning-fog. https://github.com/words/gunning-fog
- retext-readability. https://github.com/retextjs/retext-readability
- Red Hat Vale ReadabilityGrade rule. https://raw.githubusercontent.com/redhat-documentation/vale-at-red-hat/main/.vale/styles/RedHat/ReadabilityGrade.yml
- Lingua::EN::Syllable. https://metacpan.org/release/GREGFAST/Lingua-EN-Syllable-0.251/source/Syllable.pm
