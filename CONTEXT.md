# gunfog

A CLI that scores prose with the Gunning fog index and reports hotspots so a coding agent can revise its own writing.

## Language

**Prose**:
The text that remains after the constructs gunfog never scores (headings, code blocks, tables, image alt text, raw HTML, task-list markers) are removed. The fog formula only ever sees prose.

**Sentence**:
The scoring unit. A span of prose bounded by UAX #29 sentence boundaries, paragraph ends, and hard breaks. A list item's start and end are always sentence boundaries, and normal segmentation applies inside the item. The position of every other block-level construct is a boundary too, whether gunfog removes its content (heading, code block, table, HTML block), it has none (thematic break), or its content is scored (blockquote, footnote definition).

**Placeholder word**:
A single simple word standing in for an inline code span or bare URL, so the sentence keeps its grammatical slot and length without the token ever counting as complex. Within a token a placeholder is a boundary: a word that joins a placeholder to real text is judged and named by its remainder fragments (`` `foo` ``-oriented is the fragment `-oriented`; re-`` `foo` ``created is the fragments `re-` and `created`, never a fused `re-created`). The token is still one word, complex when any fragment is; stand-in text never reaches output. The stand-in is capitalised, so a placeholder after a sentence terminator opens a sentence instead of being swallowed by the one before it.

**Remainder fragment**:
One contiguous run of real text left in a token after its placeholder stand-ins are stripped, joiners kept. Complexity is judged per fragment and a `complex:` list names fragments, so a printed name is always findable in the source by substring search. A word without a placeholder is one fragment; a bare placeholder has none.
_Avoid_: remainder (alone)

**Complex word**:
A word of three or more syllables, after Gunning's own exclusions (proper names, -ed/-es inflations). A hyphenated word is complex only if one of its hyphen-separated parts is 3+ syllables on its own.
_Avoid_: hard word, difficult word

**Word**:
One token of prose. Hyphenated compounds, contractions, and numbers each count as one word, hyphen kept.

**Contribution**:
How much a sentence raises the document's fog score: the drop in the score if the sentence were removed. Every sentence has one; it can be negative.

**Hotspot**:
A sentence the report singles out, by contribution, as driving the document's fog score above the target. Only a document scoring over target has hotspots.

**Excerpt**:
The quoted snippet that identifies a hotspot sentence: its first ~8 words of extracted prose, one excerpt word per counted word, with a placeholder word shown as the construct it replaced, as written in the source (whitespace a construct carries collapses to single spaces, so that one word can span several space-separated parts). Markup that extraction removes never appears in it. What the collapsing leaves is sanitised. The text between the outer quotes is opaque: it is not parseable by splitting on quote characters.

**Complex list**:
The names printed after `complex:`, beside a hotspot's excerpt and on the refusal line under the floor. Each name is one complex remainder fragment, sanitised. Names print in document order, deduplicated case-insensitively, first spelling kept. A hotspot's list is capped at three names: the three with the most syllables, an earlier name winning a tie for the last slot, and a trailing `…` when the cap hid a name. A name is ranked on its hyphen-separated parts added together, so the whole compound counts. Complexity still turns on one part reaching three syllables on its own. The refusal's list is uncapped, and deduplicated across the whole document rather than within one sentence.
_Avoid_: complex words (for the printed names)

**Sanitising**:
Replacing each Unicode control (`Cc`) or format (`Cf`) character with one U+FFFD (`�`), one replacement per character. It applies wherever the report quotes the document: the excerpt, a hotspot's complex list, and the refusal's complex list. A scored document cannot write escape sequences to the terminal, reorder a line with a bidi override, or hide names with zero-width characters.

**Target**:
The fog score threshold (default 10) that sets hotspot flagging and the exit code.

**Floor**:
The minimum prose length (100 words, exact) a document needs to receive a fog score. Under the floor gunfog reports no score and the run fails, though complex words are still named.
