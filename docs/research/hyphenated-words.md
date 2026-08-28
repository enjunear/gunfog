# Hyphenated words: one token or two

Research for the `g-fog` planning effort. Date: 2026-08-28.

Question: when `gfog` divides words by sentences, is `well-known` one word or two? Robert
Gunning's own instructions are the first place to look. If they are silent, the
Flesch-Kincaid primary sources are the fallback, since fog and FK share the same
words-per-sentence term.

Companion to `existing-readability-tools.md`, which flagged that hyphenation alone swung one
sentence from 2.60 to 6.24 across implementations, and to `syllable-counting.md`.

Both editions of Gunning's book are lending-restricted on the Internet Archive, so the OCR
text cannot be downloaded. Every Gunning quote below came out of Internet Archive
search-inside, which returns verbatim snippets from the scanned page, queried through
`https://openlibrary.org/search/inside.json`. Quotes were confirmed against all three scans
(two of the 1952 first edition, one of the 1968 revision) so an OCR slip in one scan cannot
carry a claim. The Kincaid report is public domain and was read in full. Measurements at the
end were run on this machine; the script lives in the session scratchpad.

## Verdict up front

**Count `well-known` as one word. Two primary sources say so and none says otherwise.**

**1. Gunning never mentions hyphens.** His three-step recipe, printed twice in both editions,
defines the sentence-length term as "the number of words in successive sentences" and stops
there. There is no definition of a word anywhere in the procedure. The only compound rule he
gives sits in the *other* term, the complex-word count, and it is about spelling-joined
compounds: "combinations of short easy words (like 'bookkeeper' and 'manpower')". No hyphen
appears in either example, in either edition.

**2. Flesch is explicit, three times, in his own words.** "Count contractions and hyphenated
words as one word" appears verbatim in *The Art of Plain Talk* (1946), *The Art of Readable
Writing* (1949) and *How to Test Readability* (1951).

**3. The Navy report behind Flesch-Kincaid repeats it and gives an example.** Kincaid et al.
1975, Appendix B, page 39: "Count as a word any numbers, letters, symbols, groups of letters
surrounded by white spaces. Hyphenated words [and] contractions count as one word", with
`second-grade` listed among the one-word examples. That appendix is the counting rulebook
behind the formula the DoD adopted.

So the ruling is settled on the words-per-sentence term. **The awkward part is the other
term.** `gfog` has already decided to skip compound detection in the complex-word rule. Pair
that with the one-word ruling and hyphenated compounds start scoring as complex words on
their combined syllable count, which is exactly what Gunning's exclusion existed to prevent.
Measured on this repo's own prose: **13.2% of all complex words are hyphenated tokens, and
two thirds of those are compounds whose every part is one or two syllables** (`complex-word`,
`per-sentence`, `hand-coded`, `leave-one-out`). That is a real bias, and section 6 argues it
should be handled with a cheap rule rather than by mis-tokenising the word.

## 1. What Gunning actually wrote

The recipe appears in Part One of both editions, running from around page 36. Verbatim from
the 1952 first edition (Internet Archive `techniqueofclear0000gunn`, confirmed against
`techniqueofclear0000gunn_y0m0`):

> To find the Fog Index of a passage, then, take these three simple steps:
>
> One: Jot down the number of words in successive sentences. If the piece is long, you may
> wish to take several samples of 100 words, spaced evenly through [...] Divide the number of
> words in the passage by the number of sentences. This gives the average sentence length of
> the passage.
>
> Two: Count the number of words of three syllables or more per 100 words. Don't count the
> words (1) that are capitalized, (2) that are combinations of short easy words (like
> "bookkeeper" and "butterfly"), (3) that are verb forms made three syllables by adding -ed or
> -es (like "created" or "trespasses").
>
> Three: To get the Fog Index, total the two factors just counted and multiply by .4.

The bracketed gap is a snippet-window boundary, not an ellipsis in the original. The 1968
revision (`techniqueofclear00gunn`) changes two things in step Two and nothing else that
matters here: "(1) that are proper names" replaces "that are capitalized", and the compound
examples become "bookkeeper" and "manpower".

The rules are restated later in both editions, in the same terms: `"bookkeeper," "inasmuch,"
"butterfly"; (3) verb forms which are made three syllables because -ed or -es has been added`
in 1952, `"bookkeeper" and "inas-much"` in 1968.

Three things follow.

**Gunning defines no word boundary.** Step One says "jot down the number of words". That is
the entire specification. Flesch, writing at the same time for the same audience, spent a
paragraph on what counts as a word. Gunning did not. He was writing for people counting by
eye on a printed page, where the question does not come up, because a reader looking at
`well-known` sees one word.

**His compound rule is about spelling, not punctuation.** `bookkeeper`, `manpower`,
`butterfly` and `inasmuch` are all written solid. Not one of his four examples has a hyphen
in it. Whatever he meant by "combinations of short easy words", he illustrated it exclusively
with closed compounds.

**The only hyphen anywhere near the passage is a typesetting artifact.** The 1952 scan OCRs
the example as `book- keeper`, because the word broke across a line. That is worth naming
because it is the same failure mode a naive tokeniser hits on scanned or hard-wrapped text.

I could not prove that the word "hyphen" appears nowhere in the book's 300-odd pages, because
lending restrictions block the full text and search-inside cannot be scoped to one volume.
The bounded claim is what matters: the printed procedure, in both editions and in both places
it appears, says nothing about hyphenation.

## 2. Where the folklore came from

Search the web and you will be told that the fog index excludes hyphenated words from the
complex-word count. It does not, and Gunning never said it did.

Wikipedia's step 3 reads "Do not include proper nouns, familiar jargon, or compound words",
citing not Gunning but readabilityformulas.com
(https://en.wikipedia.org/wiki/Gunning_fog_index). That page currently says "simple compound
words (like 'sunflower' or 'bookkeeper')", which is faithful
(https://readabilityformulas.com/the-gunnings-fog-index-or-fog-readability-formula/).
Readable.com's explainer drops the exclusions entirely and says only "Complex words are those
containing three or more syllables" (https://readable.com/readability/gunning-fog-index/).

The hyphen version of the rule is a software artifact, not a documentary one. In
py-readability-metrics the Gunning complex-word test is

```python
def is_gunning_complex(t, syllable_count):
    return syllable_count >= 3 and \
        not (self._is_proper_noun(t) or self._is_compound_word(t))

def _is_compound_word(self, token):
    return re.match('.*[-].*', token) is not None
```

(https://github.com/cdimascio/py-readability-metrics, `readability/text/analyzer.py`). Someone
had to operationalise "compound word" in five lines, and "contains a hyphen" is the obvious
cheap proxy. It just happens to catch a disjoint set from the one Gunning named: it exempts
`state-of-the-art` and misses `bookkeeper`, while Gunning's examples were all closed
compounds. Read backwards out of the code, that becomes "the fog index ignores hyphenated
words", which no primary source says.

## 3. Flesch's rule, stated in his own books

Flesch's 1948 paper, "A New Readability Yardstick", *Journal of Applied Psychology*
32(3):221-233, doi:10.1037/h0057532, is paywalled at APA and I could not obtain a scan. His
own books state the counting rules for the same formula, before and after that paper, and
they agree word for word. All three quotes are verbatim from Internet Archive search-inside.

*The Art of Plain Talk* (1946, `artofplaintalk00fles` and three other scans):

> [...] count each word in it up to 100. Count contractions and hyphenated words as one word.
> Count as words numbers or letters separated by space.

*The Art of Readable Writing* (1949, `artofreadablewri0000rudo_i5l1`):

> [...] count each word in it up to 100. Count contractions and hyphenated words as one word.
> Count numbers and [...]

*How to Test Readability* (1951, `howtotestreadabi00fles` and `howtotestreadabi00flesrich`),
the manual Flesch wrote specifically for applying the Reading Ease formula:

> [...] surrounded by white space. Count contractions and hyphenated words as one word. For
> example [...]

Note what the 1951 wording does: it defines a word as a run of characters between whitespace,
then carves out one exception in the other direction. Hyphenated words would already be one
token under a whitespace split. The sentence exists because Flesch expected people to want to
split them, and told them not to.

## 4. Kincaid 1975, and the DoD line

The Navy report that produced Flesch-Kincaid Grade Level is public domain and readable in
full: Kincaid JP, Fishburne RP, Rogers RL, Chissom BS, "Derivation of New Readability Formulas
(Automated Readability Index, Fog Count and Flesch Reading Ease Formula) for Navy Enlisted
Personnel", Research Branch Report 8-75, February 1975, DTIC AD-A006655. The scan is at
https://archive.org/details/DTIC_ADA006655; the OCR text is at
https://archive.org/download/DTIC_ADA006655/DTIC_ADA006655_djvu.txt.

Appendix B is titled "Instructions for Calculating Readability Using the Recalculated ARI, Fog
Count, and Flesch Formulas" and starts on page 33. The Flesch instructions begin on page 39:

> 1. Count the number of words.
>
> Count as a word any numbers, letters, symbols, groups of letters surrounded by white spaces.
> Hyphenated words [and] contractions count as one word. For example, each of the following
> count as one word:
>
> couldn't    F.O.B.    i.e.    32,008    second-grade

The bracketed "and" is an OCR correction; the scan reads "end". `second-grade` is the report's
own worked example, which is about as unambiguous as a primary source gets.

This is the appendix behind the formula the Department of Defense adopted for technical
manuals in 1978 via MIL-M-38784, so it is the closest thing to an official government
tokenisation rule for this formula family. Microsoft Word's documented readability statistics
give the same two formulas but define ASL only as "the number of words divided by the number
of sentences" and say nothing about tokenisation
(https://support.microsoft.com/en-us/office/get-your-document-s-readability-and-level-statistics-85b4969e-e80a-4777-8dd3-f7fc3c8b3fd2).

Worth noting for contrast, since the report also recalculates a fog-family formula: the Fog
Count instructions on pages 36 to 38 never mention hyphenation either, in a document that
spells the rule out explicitly for Flesch three pages later. The one hyphenated example the
Fog Count section does contain is in its Numbers rule, where `eighty-eight` is given a value
of 1, the same as any other single easy word. So even the Navy's fog derivative, when it
touches a hyphenated token at all, treats it as one.

The Navy Fog Count is not Gunning's fog index. It uses a two-syllable hard-word threshold and
the formula `(easy words + 3 x hard words) / sentences`, then subtracts 3 and halves. `gfog`
implements Gunning's, not this. It is cited here only because it is the one place a fog-family
counting procedure was written down by someone other than Gunning.

## 5. What the implementations do

| Tool | `well-known` in the word count | Mechanism |
| --- | --- | --- |
| textstat (Python) | 1 word | `count_words(split_hyphens=False)` by default |
| py-readability-metrics | 1 word | NLTK `TweetTokenizer` keeps the token intact |
| `gunning-fog` crate (Rust) | 2 words | `\b\w+\b` splits on the hyphen |
| quanteda (R) | 1 word | `tokens(split_hyphens = FALSE)` by default |

Measured here: `textstat.lexicon_count("a well-known result")` returns 3. The docstring of
`count_words` says it outright, "By default, English contractions (e.g. "aren't") and
hyphenated words are counted as one word", with a `split_hyphens` flag defaulting to `False`
(https://github.com/textstat/textstat, `textstat/backend/counts/_count_words.py`).
`TweetTokenizer().tokenize("a well-known state-of-the-art result")` returns
`['a', 'well-known', 'state-of-the-art', 'result']`, so py-readability-metrics counts one word
too. quanteda documents the same default
(https://quanteda.io/reference/tokens.html).

The Rust `gunning-fog` crate is the outlier, and not deliberately: its word count is one
regex, `\b\w+\b`, applied to raw text
(https://github.com/astuanax/gunning-fog, `src/lib.rs`). Nothing in it decided anything about
hyphens.

One correction to `existing-readability-tools.md`, which recorded that textstat "strips
hyphens and joins, manufacturing fake tokens". That is true of the *syllable* path, where
`remove_punctuation` turns `well-known` into `wellknown` before counting vowel groups, and its
docstring confirms "Hyphens are always removed". It is not a separate word-count decision:
the join is what makes the word count come out at 1, which is the right answer for the wrong
reason. Both effects come from the same line.

## 6. What the ruling is worth, in grade levels

Measured on this repo's four existing research documents, 12,815 word tokens of
agent-written technical prose after stripping code blocks, tables, list items and URLs. Fog
computed with the naive vowel-group syllable heuristic and a plain 3+ syllable complex-word
rule, no exclusions. Only the delta is meaningful; the absolute numbers inherit the
heuristic's error.

| Document | Words | Hyphenated | Fog, one word | Fog, split | Delta |
| --- | --- | --- | --- | --- | --- |
| existing-readability-tools.md | 2,699 | 3.1% | 11.35 | 10.62 | -0.73 |
| fog-vs-flesch-kincaid.md | 4,179 | 2.5% | 13.02 | 12.36 | -0.67 |
| rust-feasibility.md | 2,797 | 2.0% | 10.49 | 10.07 | -0.42 |
| syllable-counting.md | 3,140 | 3.0% | 10.83 | 10.20 | -0.64 |

**About 0.6 grade levels at document scale.** Splitting lowers the score, because the extra
short easy words dilute the complex-word percentage faster than they raise the sentence
length. Both terms move and they move against each other, which is why this cannot be
reasoned about without measuring it.

At sentence scale it is much larger, which matters because `gfog` reports per-sentence
hotspots. Across the 236 sentences of eight or more words that contain at least one
hyphenated token, the median delta is **-1.49 grade levels**, the 5th percentile is -5.47,
and the worst case measured is -10.63. A single ruling on a punctuation mark moves an
individual sentence more than most real rewrites do.

### The complex-word interaction, which is the bigger problem

Of the 2,197 complex words in the corpus, **290 (13.2%) are hyphenated tokens**. Of those 290,
**190 are compounds whose every part is one or two syllables**: `complex-word`,
`per-sentence`, `vowel-group`, `hand-coded`, `leave-one-out`, `agent-written`, `fog-only`.

These are precisely Gunning's "combinations of short easy words". He excluded `bookkeeper`
and `manpower` from the hard-word count because a compound of two easy words is not hard to
read, and the hyphen changes nothing about that. Keeping the token whole and then judging it
by total syllable count charges the writer three syllables for `per-sentence`, a word made of
`per` and `sentence`.

The one-word ruling is right and the no-compound-detection decision is defensible on its own.
Together they overcount. The cheap fix does not require the general compound problem to be
solved, because the hyphen is doing the segmentation for free:

> A hyphenated token is one word. It is complex only if some hyphen-separated part is three
> or more syllables on its own.

That keeps `state-of-the-art` and `per-sentence` easy, keeps `agent-oriented` complex on the
strength of `oriented`, needs no dictionary and no compound splitter, and lands closer to
Gunning's stated intent than either implementation surveyed above. On this corpus it removes
190 of the 290 hyphenated complex words, moving the complex-word share from 17.1% to 15.6%,
roughly 0.6 grade levels of the same magnitude as the tokenisation question itself.

This is a separate call from the one this note was asked to make, and it belongs to whoever
owns the complex-word rule. Flagging it here because the two decisions interact, and taking
the correct one in isolation makes the score worse.

## 7. Recommendation

For the words-per-sentence term: **one hyphenated token is one word.** Do not split on
hyphens, do not remove them, do not join the parts.

Backed by, in order of weight:

1. Kincaid et al. 1975, p. 39: "Hyphenated words [and] contractions count as one word",
   example `second-grade`. This is the primary source for Flesch-Kincaid and the basis of the
   DoD standard.
2. Flesch, *How to Test Readability* (1951), *The Art of Readable Writing* (1949), *The Art of
   Plain Talk* (1946): "Count contractions and hyphenated words as one word."
3. Gunning, *The Technique of Clear Writing*, 1952 and 1968 editions: silent. No definition of
   a word, no mention of hyphenation, and his one compound rule belongs to the other term and
   is illustrated only with closed compounds.

Do not remove the hyphen before counting, textstat-style. It gives the right word count by
accident and wrecks the syllable count, which `syllable-counting.md` needs intact.

Do keep the hyphen positions, because section 6's complex-word rule needs them, and because
naming the offending word back to the user is the point of the tool. Reporting `wellknown` as
a complex word would be a bug the user cannot act on.

## Sources

Primary:

- Gunning R. *The Technique of Clear Writing*. New York: McGraw-Hill, 1952. Internet Archive
  https://archive.org/details/techniqueofclear0000gunn and
  https://archive.org/details/techniqueofclear0000gunn_y0m0 (lending-restricted; text via
  search-inside only).
- Gunning R. *The Technique of Clear Writing*, revised edition. New York: McGraw-Hill, 1968.
  https://archive.org/details/techniqueofclear00gunn (lending-restricted).
- Flesch R. *The Art of Plain Talk*. Harper, 1946.
  https://archive.org/details/artofplaintalk00fles
- Flesch R. *The Art of Readable Writing*. Harper, 1949. Quoted from scan
  https://archive.org/details/artofreadablewri0000rudo_i5l1
- Flesch R. *How to Test Readability*. Harper, 1951.
  https://archive.org/details/howtotestreadabi00fles
- Flesch R. "A New Readability Yardstick." *Journal of Applied Psychology* 1948;32(3):221-233.
  doi:10.1037/h0057532. Paywalled at APA, not obtained; superseded here by Flesch's own books,
  which state the same counting rules.
- Kincaid JP, Fishburne RP, Rogers RL, Chissom BS. "Derivation of New Readability Formulas
  (Automated Readability Index, Fog Count and Flesch Reading Ease Formula) for Navy Enlisted
  Personnel." Research Branch Report 8-75, Naval Air Station Memphis, February 1975. DTIC
  AD-A006655, doi:10.21236/ADA006655. https://archive.org/details/DTIC_ADA006655

Method for the book quotes: `https://openlibrary.org/search/inside.json?q="<exact phrase>"`,
which returns verbatim OCR snippets from lending-restricted scans and supports quoted phrase
queries. Walking overlapping phrases reconstructs a passage. Every Gunning quote was verified
against all three scans of the book.

Implementations and secondary:

- textstat: https://github.com/textstat/textstat (`backend/counts/_count_words.py`,
  `backend/selections/_list_words.py`, `backend/transformations/_remove_punctuation.py`)
- py-readability-metrics: https://github.com/cdimascio/py-readability-metrics
  (`readability/text/analyzer.py`)
- `gunning-fog` crate: https://github.com/astuanax/gunning-fog
- quanteda `tokens()`: https://quanteda.io/reference/tokens.html
- Microsoft Word readability statistics:
  https://support.microsoft.com/en-us/office/get-your-document-s-readability-and-level-statistics-85b4969e-e80a-4777-8dd3-f7fc3c8b3fd2
- Wikipedia, Gunning fog index: https://en.wikipedia.org/wiki/Gunning_fog_index
- ReadabilityFormulas.com:
  https://readabilityformulas.com/the-gunnings-fog-index-or-fog-readability-formula/
- Readable.com: https://readable.com/readability/gunning-fog-index/

Measurements were run on this machine on 2026-08-28 against the four Markdown files in
`docs/research/`, Python 3.14. The measurement script is in the session scratchpad, not this
repo; the corpus and method are described in section 6 so the numbers can be re-run.
