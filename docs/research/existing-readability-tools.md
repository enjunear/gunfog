# Existing Gunning fog and readability tooling

Research for the `g-fog` planning effort. Date: 2026-08-28.

Question: what already exists for Gunning fog / readability scoring that a coding agent
could call from a CLI, and what would a new tool need to implement?

Everything below was checked against source code, package registries, or official docs.
Where a number is quoted, it was measured on this machine. The test scripts live in the
session scratchpad, not in this repo; the exact inputs are reproduced inline so the
measurements can be re-run.

## Verdict up front

There is a genuine gap, but a narrow one.

No shipped tool does all three of: (1) install as a standalone CLI, (2) report a Gunning
fog number, (3) localise the score to the sentences and words driving it. The pieces all
exist, split across projects:

| Capability | Who has it | What is missing |
| --- | --- | --- |
| Fog number, CLI, JSON | `textlens`, `readlevel` (npm) | No hotspots. Never names the complex words it counted. |
| Fog number, CLI, plain text | GNU `style` | Document-scoped fog. Sentence-level flags exist but key off ARI and word count, not fog. |
| Fog-capable formula engine, JSON | Vale `metric` check | Explicitly summary-scoped. Cannot localise. |
| Per-sentence hotspots, fog as one input | `retext-readability` | Library, no CLI. Discards the fog number, reports only a vote. |
| Complex-word list | `textstat.difficult_words_list()` | Library, no CLI. Not the canonical complex-word rule. |

The closest existing tool is `readlevel` (npm, v0.2.0, published 2026-06-02). It has a fog
score, JSON output, a `--max-grade <n>` CI gate that resembles `--target`, and a `--long`
flag listing the longest sentences. It still does not name complex words, does not score
individual sentences, and gates on the mean of six formulas rather than on fog.

The second finding matters more than the first: **the shipped implementations disagree with
each other by about 5 grade levels on identical input**, because none of them implements the
canonical complex-word rule. A new tool has to pick a definition and state it, because
"the Gunning fog score" is not a single number in practice.

## 1. What exists

### 1.1 Python

**textstat** (https://github.com/textstat/textstat). MIT. v0.7.13, released 2026-02-18.
1378 stars, active. Depends on `pyphen`, `nltk`, `setuptools`.

- No CLI. Confirmed by inspecting the installed distribution: no `entry_points.txt`, and
  `.venv/bin/` contains only the `nltk` and `tqdm` scripts.
- `gunning_fog()` is `0.4 * (words_per_sentence + 100 * difficult_words / words)`.
- Its complex-word test is not the canonical one. `is_difficult_word()` returns true when a
  word is **absent from the Dale-Chall easy-word list** (2940 entries) **and** has 3 or more
  syllables. It lowercases first, so proper nouns are counted, and it has no suffix rule.
  Measured: `created`, `included`, `reported`, `expanding`, `California`, `Anthropic` and
  `Kubernetes` are all counted as complex. `following` and `interesting` are not, purely
  because they happen to be on the easy-word list.
- Syllables come from NLTK's CMUdict with a `pyphen` fallback
  (`textstat/backend/counts/_count_syllables.py`). CMUdict is fetched over the network on
  first use via `nltk.download('cmudict', quiet=True)`, which is a runtime dependency on
  network access that a CLI would inherit.
- `difficult_words_list(text, syllable_threshold=3)` is public and returns the actual words.
  This is the one off-the-shelf hotspot primitive in the Python ecosystem, though it returns
  bare strings with no positions.
- Cold start measured at 0.58 s and 103 MB RSS for import plus one call.

**py-readability-metrics** (https://github.com/cdimascio/py-readability-metrics). MIT.
v1.4.5, last release 2020-08-02, last repo push 2024-09-15. 406 stars.

- No CLI.
- **Refuses texts under 100 words.** `readability/scorers/gunning_fog.py` raises
  `ReadabilityException('100 words required.')` in the constructor. Measured: a 98-word
  sample raised; an 11-word sample raised. This alone disqualifies it for `g-fog`, whose
  whole point is scoring short passages an agent just wrote.
- It is the only surveyed implementation that attempts the canonical exclusions, in
  `readability/text/analyzer.py`:
  ```python
  def is_gunning_complex(t, syllable_count):
      return syllable_count >= 3 and \
          not (self._is_proper_noun(t) or self._is_compound_word(t))
  ```
  But `_is_proper_noun` is `return token[0].isupper()` with the POS-tagging version
  commented out, so **the first word of every sentence is exempted** whether or not it is a
  proper noun. `_is_compound_word` is `re.match('.*[-].*', token)`, so any hyphenated word is
  exempted regardless of syllable count. There is no suffix rule.
- Its syllable counter is a five-line regex (`readability/text/syllables.py`) that returns 1
  for any word of 3 characters or fewer.
- Uses NLTK `sent_tokenize` (Punkt) for sentences, so it needs the `punkt` corpus downloaded.

**textstat-cli-tddschn** (https://pypi.org/project/textstat-cli-tddschn/) is a third-party
wrapper that adds a CLI with `-j/--json` over textstat. Whole-text aggregate only.

### 1.2 JavaScript and TypeScript

None of the library-grade packages ship a CLI. Confirmed by reading the `bin` field from the
npm registry for each: `text-readability`, `readability-scores`, `retext-readability`,
`readability-kit`, `@power-seo/readability`, `text-readability-ts` and `readability-cyr` all
have `bin: null`.

**text-readability** (npm, https://github.com/clearnote01/readability). v1.1.1, published
2025-03-06. Licence is inconsistent: ISC in the npm manifest, MIT on GitHub. 81 stars.
A port of textstat, so it inherits the Dale-Chall-plus-syllable-threshold definition
(`main.js:236-241`, `main.js:210`) with a different syllable counter. Cold start measured at
0.01 s and 30 MB RSS under Bun, roughly 58x faster to start than the Python path.

**readability-scores** (npm, https://github.com/MichaelChambers/readability-scores). MIT.
v1.0.8, published 2020-05-15. 5 stars. Counts plain polysyllabic words with no exclusions.

**retext-readability** (https://github.com/retextjs/retext-readability). MIT. v8.0.0,
published 2023-09-11. 101 stars, 0 open issues, no commits since publication.

Architecturally this is the closest thing to the `g-fog` output model, and it is worth
reading before designing the new tool. It parses prose to an nlcst tree, scores **each
sentence** with 7 formulas including `gunning-fog`, converts each to an age, and emits a
vfile message when at least `threshold` (default 4/7) of them exceed the target age. Measured
output carries `place.start` with line, column and offset, plus `actual` holding the sentence
text:

```
1:1-1:104: Unexpected hard to read sentence, according to 5 out of 7 algorithms
```

Two limits for our purposes. It **throws the fog number away**, keeping only the vote count,
so an agent cannot see how far off target it is. And its complex-word test, in
`lib/index.js:85-93`, is:

```js
// Count complex words for gunning-fog based on whether they have three
// or more syllables and whether they aren't proper nouns.  The last is
// checked a little simple, so this index might be over-eager.
if (syllables >= 3) {
  polysillabicWord++
  if (value.charCodeAt(0) === caseless.charCodeAt(0)) {
    complexPolysillabicWord++
  }
}
```

The same first-word-of-sentence flaw as py-readability-metrics, and the authors say so in the
comment. `minWords` defaults to 5, so short sentences are skipped entirely.

**gunning-fog** (npm, https://github.com/words/gunning-fog). MIT. v2.0.1, published
2022-11-01. 20 stars, dormant. This is the cleanest building block available in any language:
a pure function over counts, no tokenisation, no opinion about what a complex word is.
The entire implementation:

```js
return weight * (counts.word / counts.sentence +
  complexWordWeight * ((counts.complexPolysillabicWord || 0) / counts.word))
```

**syllable** (https://github.com/words/syllable). MIT. 248 stars, last push 2022-11-02.
The standard JS syllable counter, used by retext-readability. Dictionary plus heuristics, no
network dependency.

**write-good** (https://github.com/btford/write-good). MIT. v1.0.8, published 2021-02-16.
5086 stars, last push 2025-03-10. Ships a `write-good` binary, and it is per-location, but it
is a rule-based prose linter. It computes no readability score at all.

**textlens** (npm, MIT, v1.0.11, published 2026-03-22) and **@didrod2539/readlevel** (npm,
MIT, v0.2.0, published 2026-06-02) both ship a real CLI with a Gunning fog number. Both are
new, tiny and single-author. Measured behaviour:

- `textlens <file>` defaults to a box-drawing ASCII table that is both token-heavy and
  visibly broken (the interpretation column truncates mid-word: `Very Confusi`). `--json`
  is 3471 bytes for a 23-word input. `--format minimal` gives one line but drops fog:
  `Grade 19.4 | 23 words | 1 min read | Neutral sentiment`. No hotspots.
- `readlevel` reads stdin or a file, `--json` is 656 bytes, and it has `--max-grade <n>` as a
  CI gate. Its `--long` flag is the nearest thing to a hotspot in any shipped tool, but it
  sorts by word count only, truncates with an ellipsis, and lists every sentence including
  trivial ones:
  ```
  Longest sentences:
    [11w] The deployment pipeline requires considerable configuration before the…
    [10w] Engineers must establish authentication credentials and provision the …
    [2w] It works.
  ```
  It reports `complexWords: 15` but never says which words those are.

### 1.3 Standalone CLIs

**GNU diction / style** (https://www.gnu.org/software/diction/). GPLv3. v1.14, in Debian
since 2023-08-17 (https://tracker.debian.org/pkg/diction). Low-frequency but not abandoned;
the previous release 1.11 was 2014.

`style` does report a Fog Index, alongside Kincaid, ARI, Coleman-Liau, Flesch, Lix and SMOG,
plus passive-voice, nominalisation and sentence-length statistics
(https://manpages.debian.org/testing/diction/style.1.en.html). Output is a plain-text summary
block, not JSON. It has sentence-level flags via `-l length`, `-r ari`, `-p` and `-N`
(https://man.freebsd.org/cgi/man.cgi?query=style), but there is no fog equivalent: the
per-sentence threshold flag keys off ARI, not fog. So fog is document-scoped.

`diction` flags wordy and clichéd phrases inline in `[brackets]` with `->` suggestions. No
score, no JSON.

**Vale** (https://vale.sh, https://github.com/vale-cli/vale). MIT. Very active, v3.19.0
published 2026-08-26.

Vale can express Gunning fog. The `metric` check
(https://docs.vale.sh/checks/metric/) exposes exactly the variables needed, verbatim from the
docs: "blockquote, characters, complex_words, heading.h{n}, list, long_words, paragraphs,
polysyllabic_words, pre, sentences, syllables, words". A fog rule is one small YAML file in
the same shape as the docs' Flesch-Kincaid example.

But it cannot localise. The docs state plainly: "Since the pre-defined variables are
calculated using the entire document, all metric-based rules are summary-scoped." Vale's
other check types (`existence`, `occurrence`, `substitution` and so on) do give line and
column, and `--output=JSON` is genuinely compact, but those are regex checks and cannot carry
a formula. Vale also does not document what its `complex_words` variable means, which is the
same ambiguity as everywhere else.

**proselint** (https://github.com/amperser/proselint). BSD-3-Clause. Last tagged release
v0.16.0 (2022-11-14), last push 2026-08-26. It computes no readability score of any kind. It
is a rule-based linter with `--output-format json` giving check id, message and span.

**Others checked and ruled out.** `readability-cli` and the `readability` crates and npm
packages are almost all wrappers around Mozilla's Readability.js content extractor, a naming
collision with readability *scoring*; they compute no score. `alex` is an inclusive-language
linter. `textlint`'s readability-adjacent rules (`textlint-rule-sentence-length`,
`preset-ja-technical-writing`) are Japanese character-count rules, not syllable formulas.
Go's `cirello.io/gunning-fog` (MIT) prints a single aggregate number from stdin. The Rust
`gunning-fog` crate (MIT, 0.1.0, one release) is library-only with a single aggregate score.
Heydon Pickering's `readability-checker`
(https://github.com/Heydon/readabilityCheckerCLI) is a genuine hotspot CLI listing words over
4 syllables and sentences over 35 words, but it reports Flesch only, has no licence file, and
has not been touched since 2016-08-09.

## 2. Algorithm reference

### 2.1 The formula

```
fog = 0.4 * [ (words / sentences) + 100 * (complex words / words) ]
```

Wikipedia states it as `0.4 [ ( words / sentences ) + 100 ( complex words / words ) ]`
(https://en.wikipedia.org/wiki/Gunning_fog_index), attributing the index to Robert Gunning,
*The Technique of Clear Writing*, McGraw-Hill, 1952, pp. 36-37. The book itself is loan-only
on the Internet Archive (https://archive.org/details/techniqueofclear0000gunn_y0m0) and was
not read directly, so the page citation is Wikipedia's attribution rather than verified
primary text. The traditional reference site gunning-fog-index.com is dead (expired TLS
certificate, then 403). ReadabilityFormulas.com gives the same formula
(https://readabilityformulas.com/the-gunnings-fog-index-or-fog-readability-formula/).

The coefficient 0.4 and the structure are not in dispute anywhere. Every implementation
surveyed agrees on them. All the disagreement is in the inputs.

### 2.2 The canonical complex word

Wikipedia, verbatim: "Count the 'complex' words consisting of three or more syllables. Do not
include proper nouns, familiar jargon, or compound words. Do not include common suffixes
(such as -es, -ed, or -ing) as a syllable."

Note the exact phrasing of the last rule. It is not "exclude words that reach three syllables
via a suffix", it is "do not count the suffix as a syllable". Operationally that means strip
the suffix, then count: `created` becomes `creat`, 2 syllables, not complex. Same outcome in
most cases, but the stated rule is about syllable counting, not word exclusion.

ReadabilityFormulas.com agrees on proper nouns ("Baltimore," "Maryland," "Mrs. Madison") and
compound words ("sunflower" or "bookkeeper"), but its suffix list is **-ed and -es only, with
-ing omitted** ("Words made complex by suffixes -ed or -es (like 'created' or 'trespasses')").
That is a real disagreement between the two most-cited public statements of the rule.

"Familiar jargon" is defined nowhere. No source surveyed gives a test for it. It is left to
the analyst, which is why textstat substitutes the Dale-Chall easy-word list as a proxy. That
substitution is a defensible engineering choice but it is not the canonical rule, and it
should be named as a deviation rather than presented as fog.

The traditional counterexample to the whole approach is "interesting": three syllables, not
remotely hard.

### 2.3 Sentence counting

Gunning's original counted **each independent clause as a separate sentence**. Wikipedia:
"Until the 1980s, the fog index was calculated differently. The original formula counted each
clause as a sentence... In the 1980s, the calculation method changed... the clause counting
step was left out", citing Gunning 1952 and Judith Bogert, "In Defense of the Fog Index",
*Business Communication Quarterly*, 1985.

ReadabilityFormulas.com still states the older operational rule: "Gunning Fog considers any
sentence with a semicolon, comma, or colon that link a complete thought as a sentence stop.
Such a sentence is scored as a compound sentence (2 sentences)."

Modern implementations universally use terminal punctuation only. For `g-fog` this is a real
decision: clause-splitting would make the score more sensitive to exactly the run-on
constructions an agent should be told to break up, but it would put the tool out of step with
every other implementation. Worth an explicit note in the spec either way.

No source surveyed gives rules for hyphenated words, numbers or abbreviations. This is a gap
in the canonical description, not a disagreement, and implementers decide for themselves.
Zheng and Yu (2017), *JMIR* 19(3):e59, doi:10.2196/jmir.6962
(https://pmc.ncbi.nlm.nih.gov/articles/PMC5355629/) note the specific failure that "COPD"
counts as roughly one syllable while representing a technically hard term.

### 2.4 Interpretation

| Fog | Level |
| --- | --- |
| 6 | 6th grade |
| 8 | 8th grade |
| 9 | high-school freshman |
| 12 | high-school senior |
| 17 | college graduate |

Wikipedia: "Texts for a wide audience generally need a fog index less than 12. Texts requiring
near-universal understanding generally need an index less than 8." No source quoting Gunning's
own words for these thresholds was found, only later summaries attributing them to him.

The proposed `--target 9` default sits sensibly inside this range.

### 2.5 Validity limits and minimum length

- Zheng and Yu (2017), cited above, found low correlation between fog scores and
  user-perceived difficulty for medical text, and note the formula was built for
  general-audience material below grade 12.
- The Guilford College writing manual
  (https://library.guilford.edu/c.php?g=111810&p=723930) instructs "choose at least 100 words
  of writing" as step one. This is the clearest explicit statement of a ~100-word minimum tied
  specifically to fog, and it is presumably where py-readability-metrics' hard limit comes
  from. No peer-reviewed source quantifying fog's error at short lengths was found.
- General critique across sources: surface features (sentence length, syllable count) ignore
  coherence and organisation, so jumbled short-worded text can score "easy".

This matters for `g-fog`'s core use case. Scoring a 20-word paragraph is outside what the
formula was validated for. Reporting a bare number for short input is misleading; either
report the input size alongside the score, or degrade to hotspots-only below some threshold.
Note that py-readability-metrics chose to hard-fail, which is the one behaviour `g-fog`
definitely should not copy.

## 3. How much the implementations actually diverge

Measured on one 107-word paragraph. Every tool agreed on 107 words and 8 sentences, so the
entire spread comes from the complex-word definition and the syllable counter.

| Implementation | Fog | Complex-word rule |
| --- | --- | --- |
| py-readability-metrics 1.4.5 | 25.54 | 3+ syllables, minus capitalised words and hyphenated words |
| text-readability 1.1.1 | 25.92 | not on Dale-Chall easy list AND 3+ syllables |
| textstat 0.7.13 | 27.03 | not on Dale-Chall easy list AND 3+ syllables |
| readability-scores 1.0.8 | 28.90 | plain 3+ syllables |
| textlens 1.0.11 (CLI) | 30.40 | plain 3+ syllables, different syllable counter |
| readlevel 0.2.0 (CLI) | 30.40 | plain 3+ syllables, different syllable counter |

**Spread: 4.9 grade levels between shipped tools on identical input.** textstat and
text-readability implement the same rule and still differ by 1.1 because their syllable
counters differ (CMUdict versus a JS heuristic).

Isolating each exclusion on the same text, using the `syllable` package throughout so only
the rule varies:

| Rule | Complex words | Fog |
| --- | --- | --- |
| Plain 3+ syllables, no exclusions | 63 | 28.90 |
| plus proper-noun exclusion | 58 | 27.03 |
| plus suffix stripping (-es/-ed/-ing) | 57 | 26.66 |
| plus hyphen/compound exclusion | 63 | 28.90 |
| All three combined | 52 | 24.79 |

The canonical exclusions are worth **4.1 grade levels** against the naive count. This is not a
rounding difference. A tool that says "Gunning fog" without saying which rule it used is
reporting a number the user cannot reproduce.

The hyphen rule changed nothing on this sample only because it contains no hyphenated words.
It matters a great deal on technical prose, and the implementations handle it in three
mutually incompatible ways:

- textstat **strips hyphens and joins**, manufacturing tokens that were never words.
  Measured on `The state-of-the-art system is well-known. It works fine today.`, its word
  list is `['The', 'stateoftheart', 'system', 'is', 'wellknown', 'It', 'works', 'fine',
  'today']` and `difficult_words_list()` returns `['stateoftheart']`. Writing the same
  sentence without hyphens moves the fog score from **6.24 to 2.60**. A 3.6-point swing
  from punctuation alone, on text with identical meaning.
- py-readability-metrics exempts any hyphenated token entirely.
- The `syllable` package counts `state-of-the-art` as 4 syllables, treating it as one word.

## 4. Segmentation pitfalls for agent-written text

These were measured against `textstat`, whose sentence splitter is
`re.findall(r"\b[^.!?]+[.!?]*", text)` with a post-filter dropping any fragment of 2 words or
fewer (`textstat/backend/counts/_count_sentences.py`). The failure modes are representative of
regex-based splitting generally, and every one of them is something an agent's own prose will
hit routinely.

| Input | Sentences reported | Correct | Fog |
| --- | --- | --- | --- |
| `The version 1.2.3 release improved performance by 12.5 percent overall.` | 3 | 1 | 9.33 |
| `Setup steps:\n- Install the dependencies\n- Configure the environment\n- Run the migrations` | 1 | 3 or 4 | 18.95 |
| `## Configuration\n\nThe application requires environment variables.` | 1 | 1 plus a heading | 35.73 |
| `See https://example.com/docs/v1.2/index.html for details about configuration options.` | 1 | 1 | 14.23 |
| ` ```python\nx = compute_aggregate_statistics(dataframe)\n``` ` in context | 1 | code, should be excluded | 12.49 |
| `Configuration.` | 1 | 1 | 40.40 |

Concrete conclusions for the spec:

1. **Decimal versions and numbers split sentences.** `1.2.3` produced three sentences. Any
   splitter keying on `.` needs a digit-boundary guard.
2. **Markdown lists collapse into one giant sentence.** A three-bullet list scored 18.95
   because no bullet ends in terminal punctuation, so the whole list reads as one 11-word
   sentence with no sentence break. Bullets must be treated as sentence boundaries, or the
   tool systematically punishes lists, which are exactly the structure that makes agent
   writing readable.
3. **Headings merge into the following paragraph.** `## Configuration` has no terminator, so
   it fused with the next sentence and produced 35.73. Headings should be stripped or scored
   separately.
4. **URLs, code spans and fenced blocks are not prose.** They must be removed before scoring.
   A single long identifier can dominate a short passage's score. `g-fog` will be pointed at
   markdown by definition, so this is not an edge case.
5. **Abbreviations break naive splitting.** `p.m.` and `Dr.` produced fragments; textstat's
   2-word-fragment filter masked the damage on that sample by coincidence rather than design.
6. **Very short inputs produce absurd numbers.** A single word, `Configuration.`, scored
   40.40. See section 2.5.

There is also a subtle trap specific to the unified/retext route, hit while building the
feasibility spike below. `unist-util-visit` interprets a **number** returned from the visitor
as an index to resume traversal from (https://github.com/syntax-tree/unist-util-visit: "An
`Index` is treated as a tuple of `[CONTINUE, Index]`"). So the natural-looking
`visit(node, 'WordNode', w => words.push(toString(w)))` silently corrupts traversal, because
`Array.push` returns the new length. It produced 44 words for a 107-word paragraph, with some
words duplicated and others skipped. Wrap the visitor body in braces.

## 5. Feasibility spike

Per-sentence fog with word-level hotspots and positions took 25 lines on the JS side, using
`unified` + `retext-english` + `nlcst-to-string` + `unist-util-visit` + `syllable` +
`gunning-fog`. Actual output on a three-sentence input:

```json
[
 {"line":1,"col":1,"words":11,"fog":26.2,
  "hot":[["deployment",4],["pipeline",15],["considerable",33],
         ["configuration",46],["application",71],["operational",91]]},
 {"line":1,"col":105,"words":5,"fog":34,
  "hot":[["Engineers",104],["establish",119],["authentication",129],["credentials",144]]},
 {"line":1,"col":158,"words":2,"fog":0.8,"hot":[]}
]
```

The retext tree gives line, column and byte offset for every word for free, which is the hard
part of hotspot reporting. Note that `syllable` scores `pipeline` as 3, so it appears as a
hotspot; under the canonical compound-word exclusion it should not. That is the rule question
from section 2.2 showing up immediately in real output.

## 6. Recommendations

**Language: TypeScript.** Three reasons, in order of weight.

1. Cold start. Measured 0.01 s / 30 MB under Bun against 0.58 s / 103 MB for Python plus
   textstat. For a tool an agent calls repeatedly in a write-check-revise loop, a 58x startup
   difference compounds.
2. No runtime network dependency. textstat calls `nltk.download('cmudict')` on first use.
   `syllable` is self-contained.
3. The building blocks are better factored. `gunning-fog` is a pure function over counts,
   which means the complex-word rule stays entirely under our control instead of being buried
   in a library's opinion.

The counterargument for Python is `textstat.difficult_words_list()`, the only ready-made
complex-word lister. It is not worth 0.57 s of startup and a network fetch, especially since
it implements the wrong rule.

**Build on:** `gunning-fog` (MIT, pure formula), `syllable` (MIT, self-contained), and
`retext-english` + `unist-util-visit` for segmentation and positions. All MIT. The retext
packages are dormant rather than abandoned: `retext-readability` has zero open issues and no
commits since 2023-09-11, `gunning-fog` and `syllable` were last touched in 2022. Dormancy is
acceptable for a formula that has not changed since 1952, but it means no upstream fixes.

**Do not build on:** py-readability-metrics (100-word hard floor, capitalisation-based proper
noun test). `retext-readability` as a whole, since it discards the score and votes across
seven formulas, which is the opposite of what a single-formula tool with a `--target` needs.
Read its source, do not depend on it.

**Implement, because nothing provides it:**

1. A stated complex-word rule with the canonical exclusions, and a flag to select the naive
   one for comparison with other tools. Given the 4.9-point spread, the tool should say which
   rule produced the number.
2. Markdown-aware preprocessing: strip fenced and inline code, URLs and headings; treat list
   items as sentence boundaries. No surveyed tool does any of this, and every one of them
   produces garbage on the markdown an agent actually writes.
3. Per-sentence fog plus the complex words driving it, with positions. This is the actual gap.
4. Short-input handling that reports the word count alongside the score rather than failing.
5. Token-efficient default output. `textlens`' default is ASCII box art with truncated
   columns; its JSON is 3.4 KB for 23 words. The target to beat is `readlevel`'s 656-byte
   JSON, and a `--target`-aware default should be far smaller than that: the score, the
   verdict against target, and only the sentences over target.

**Naming note.** `readability-cli` on npm is Mozilla Reader Mode, and the `readability` name
is taken by content extractors in npm, PyPI and crates.io. `g-fog` avoids the collision.

## Sources

- Gunning fog index, Wikipedia: https://en.wikipedia.org/wiki/Gunning_fog_index
- Gunning, Robert. *The Technique of Clear Writing*, McGraw-Hill, 1952 (not read directly):
  https://archive.org/details/techniqueofclear0000gunn_y0m0
- Bogert, Judith. "In Defense of the Fog Index", *Business Communication Quarterly*, 1985
- ReadabilityFormulas.com fog page:
  https://readabilityformulas.com/the-gunnings-fog-index-or-fog-readability-formula/
- Guilford College writing manual: https://library.guilford.edu/c.php?g=111810&p=723930
- Zheng J, Yu H. *JMIR* 2017;19(3):e59, doi:10.2196/jmir.6962:
  https://pmc.ncbi.nlm.nih.gov/articles/PMC5355629/
- textstat: https://github.com/textstat/textstat and https://pypi.org/pypi/textstat/json
- py-readability-metrics: https://github.com/cdimascio/py-readability-metrics
- textstat-cli-tddschn: https://pypi.org/project/textstat-cli-tddschn/
- text-readability: https://github.com/clearnote01/readability
- readability-scores: https://github.com/MichaelChambers/readability-scores
- retext-readability: https://github.com/retextjs/retext-readability
- gunning-fog: https://github.com/words/gunning-fog
- syllable: https://github.com/words/syllable
- unist-util-visit: https://github.com/syntax-tree/unist-util-visit
- write-good: https://github.com/btford/write-good
- textlens: https://www.npmjs.com/package/textlens
- readlevel: https://www.npmjs.com/package/@didrod2539/readlevel
- GNU diction: https://www.gnu.org/software/diction/,
  https://manpages.debian.org/testing/diction/style.1.en.html,
  https://man.freebsd.org/cgi/man.cgi?query=style, https://tracker.debian.org/pkg/diction
- Vale: https://vale.sh, https://docs.vale.sh/checks/metric/,
  https://github.com/vale-cli/vale
- proselint: https://github.com/amperser/proselint
- readability-checker: https://github.com/Heydon/readabilityCheckerCLI
- Go gunning-fog: https://pkg.go.dev/cirello.io/gunning-fog
- Rust gunning-fog: https://crates.io/crates/gunning-fog/0.1.0
