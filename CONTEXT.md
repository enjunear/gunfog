# gunfog

A CLI that scores prose with the Gunning fog index and reports hotspots so a coding agent can revise its own writing.

## Language

**Prose**:
The text that remains after the constructs gunfog never scores (headings, code blocks, tables, image alt text, raw HTML, task-list markers) are removed. The fog formula only ever sees prose.

**Sentence**:
The scoring unit. A span of prose bounded by UAX #29 sentence boundaries, paragraph ends, and hard breaks. A list item's start and end are always sentence boundaries, and normal segmentation applies inside the item.

**Placeholder word**:
A single simple word standing in for an inline code span or bare URL, so the sentence keeps its grammatical slot and length without the token ever counting as complex. A word that joins a placeholder to real text is judged and named by its real remainder alone, joiners kept (`` `foo` ``-oriented is the word `-oriented`); stand-in text never reaches output. The stand-in is capitalised, so a placeholder after a sentence terminator opens a sentence instead of being swallowed by the one before it.

**Complex word**:
A word of three or more syllables, after Gunning's own exclusions (proper names, -ed/-es inflations). A hyphenated word is complex only if one of its hyphen-separated parts is 3+ syllables on its own.
_Avoid_: hard word, difficult word

**Word**:
One token of prose. Hyphenated compounds, contractions, and numbers each count as one word, hyphen kept.

**Contribution**:
How much a sentence raises the document's fog score: the drop in the score if the sentence were removed. Every sentence has one; it can be negative.

**Hotspot**:
A sentence the report singles out, by contribution, as driving the document's fog score above the target. Only a document scoring over target has hotspots.

**Target**:
The fog score threshold (default 10) that sets hotspot flagging and the exit code.

**Floor**:
The minimum prose length (100 words, exact) a document needs to receive a fog score. Under the floor gunfog reports no score and the run fails, though complex words are still named.
