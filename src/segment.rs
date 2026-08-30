//! Sentence segmentation and word tokenisation: the prose stream in,
//! sentences of word tokens out.
//!
//! Between the unconditional boundaries the extraction stage emits (list
//! item edges, hard breaks, paragraph ends), segmentation is UAX #29 via
//! `unicode-segmentation`, followed by a merge pass over the fixed
//! abbreviation list in `SPEC.md`. A soft break becomes a space before
//! segmentation. Tokenisation is UAX #29 word bounds with a merge pass
//! joining hyphenated compounds; contractions and number/version strings
//! are single words as UAX #29 already leaves them.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::prose::ProseEvent;

/// The stand-in text a placeholder word contributes to segmentation: one
/// simple word, so the sentence keeps its grammatical slot and length.
const PLACEHOLDER_WORD: &str = "x";

/// The abbreviations whose trailing period never ends a sentence, exactly
/// as fixed in `SPEC.md`. `etc.` is deliberately absent: it legitimately
/// ends sentences.
const ABBREVIATIONS: [&str; 10] = [
    "Mr.", "Mrs.", "Ms.", "Dr.", "Prof.", "St.", "e.g.", "i.e.", "cf.", "vs.",
];

/// One word of a sentence.
///
/// Author: Claude Fable 5
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// The word as written: a hyphenated compound keeps its hyphens, a
    /// contraction stays whole. A placeholder word carries its stand-in
    /// text instead, which never reaches output.
    pub text: String,
    /// Number/version strings and placeholder words can never be complex,
    /// whatever the syllable counter would make of their text.
    pub never_complex: bool,
}

/// One sentence of prose: the scoring unit.
///
/// Author: Claude Fable 5
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    /// The sentence's word tokens, in order. Never empty: a candidate
    /// sentence with no words is dropped.
    pub words: Vec<Word>,
    /// The sentence's byte extent in the markdown source, whitespace
    /// trimmed, so the report stage can quote it as written (a placeholder
    /// maps back to the construct it replaced).
    pub span: Range<usize>,
    /// The 1-based source line the sentence starts on.
    pub line: usize,
}

/// A run of assembled block text tied back to the markdown source: where
/// its text sits in the block, its byte extent in the source, and whether
/// it is a placeholder word.
struct Piece {
    text: Range<usize>,
    source: Range<usize>,
    placeholder: bool,
}

/// Segments the prose stream into sentences of word tokens.
///
/// `source` is the markdown the events were extracted from; the spans they
/// carry index into it and become each sentence's source span and line
/// number.
///
/// ```
/// use gunfog::prose::extract;
/// use gunfog::segment::sentences;
///
/// let md = "Ask Dr. Smith.\n\nHe'll know.";
/// let found = sentences(md, &extract(md));
/// assert_eq!(found.len(), 2);
/// assert_eq!(found[0].words.len(), 3); // Ask, Dr, Smith
/// assert_eq!(&md[found[1].span.clone()], "He'll know.");
/// assert_eq!(found[1].line, 3);
/// ```
///
/// Author: Claude Fable 5
pub fn sentences(source: &str, events: &[ProseEvent]) -> Vec<Sentence> {
    let mut out = Vec::new();
    // Sentences start at ascending source positions, so one forward pass
    // over the source turns positions into line numbers. A text event's
    // source span never contains a newline (soft and hard breaks are
    // their own events), which is what lets a piece-mapped position name
    // a line.
    let mut scanned = 0usize;
    let mut line = 1usize;
    for block in events.split(|event| matches!(event, ProseEvent::Boundary)) {
        let (text, pieces) = assemble(block);
        for range in sentence_ranges(&text) {
            let tokens = token_ranges(&text, range.clone());
            if tokens.is_empty() {
                continue;
            }
            let sentence = &text[range.clone()];
            let trimmed_start = range.start + (sentence.len() - sentence.trim_start().len());
            let trimmed_end = range.start + sentence.trim_end().len();
            let span = source_start(&pieces, trimmed_start)..source_end(&pieces, trimmed_end);
            line += source[scanned..span.start]
                .bytes()
                .filter(|&byte| byte == b'\n')
                .count();
            scanned = span.start;
            // Each lookup scans the block's pieces, so a block is quadratic
            // in its pieces. Blocks reset at every boundary and a real
            // paragraph holds few pieces; only a single paragraph flooded
            // with thousands of code spans ever notices.
            let words = tokens
                .iter()
                .map(|token| Word {
                    text: text[token.clone()].to_string(),
                    never_complex: is_number(&text[token.clone()])
                        || pieces.iter().any(|piece| {
                            piece.placeholder
                                && piece.text.start <= token.start
                                && token.end <= piece.text.end
                        }),
                })
                .collect();
            out.push(Sentence { words, span, line });
        }
    }
    out
}

/// Maps a start position in the block text to its position in the source:
/// the offset into the piece that carries it. Exact because a piece is
/// either carried verbatim or is a whole decoded entity or placeholder,
/// where a start can only sit at offset zero.
///
/// Author: Claude Fable 5
fn source_start(pieces: &[Piece], position: usize) -> usize {
    let piece = pieces
        .iter()
        .rev()
        .find(|piece| piece.text.start <= position)
        .expect("a trimmed sentence starts inside a piece");
    piece.source.start + (position - piece.text.start)
}

/// Maps an end position in the block text to its position in the source. An
/// end at a piece's edge takes the piece's source end, so a sentence ending
/// in a placeholder or entity covers the whole construct as written.
///
/// Author: Claude Fable 5
fn source_end(pieces: &[Piece], position: usize) -> usize {
    let piece = pieces
        .iter()
        .rev()
        .find(|piece| piece.text.start < position)
        .expect("a trimmed sentence ends inside a piece");
    if position >= piece.text.end {
        piece.source.end
    } else {
        piece.source.start + (position - piece.text.start)
    }
}

/// Whether the token is a number or version string: after an optional
/// leading `v` or `V`, a numeric core that starts with a digit and holds no
/// letters (`42`, `1.2.3`, `v1.2.3`). A token with no letters at all
/// (`2024-01-15`, `555-1234`) is a number outright. A hyphenated suffix is
/// a version identifier only when the core is dotted, so `1.2.3-beta` and
/// `2.0-rc1` qualify while `3-dimensional` keeps its word part eligible
/// for complexity. The dotted-core rule excuses any suffix (`10.5-inch`,
/// `4.7-magnitude`): an accepted trade-off, since telling a unit from a
/// prerelease tag would need a dictionary.
///
/// Author: Claude Fable 5
fn is_number(token: &str) -> bool {
    let rest = token.strip_prefix(['v', 'V']).unwrap_or(token);
    let (core, suffix) = match rest.split_once('-') {
        Some((core, suffix)) => (core, Some(suffix)),
        None => (rest, None),
    };
    core.starts_with(|c: char| c.is_ascii_digit())
        && !core.contains(char::is_alphabetic)
        && (suffix.is_none() || core.contains('.') || !rest.contains(char::is_alphabetic))
}

/// Flattens one between-boundaries block into contiguous text: text runs as
/// carried, a soft break as a space, a placeholder as its stand-in word,
/// with a [`Piece`] per run tying it back to the source.
///
/// Author: Claude Fable 5
fn assemble(block: &[ProseEvent]) -> (String, Vec<Piece>) {
    let mut text = String::new();
    let mut pieces = Vec::new();
    for event in block {
        match event {
            ProseEvent::Text { text: run, span } => {
                pieces.push(Piece {
                    text: text.len()..text.len() + run.len(),
                    source: span.clone(),
                    placeholder: false,
                });
                text.push_str(run);
            }
            ProseEvent::Placeholder { span } => {
                pieces.push(Piece {
                    text: text.len()..text.len() + PLACEHOLDER_WORD.len(),
                    source: span.clone(),
                    placeholder: true,
                });
                text.push_str(PLACEHOLDER_WORD);
            }
            ProseEvent::SoftBreak => text.push(' '),
            ProseEvent::Boundary => unreachable!("blocks are split on boundaries"),
        }
    }
    (text, pieces)
}

/// UAX #29 sentence segments of the block text, with a break undone
/// whenever the sentence before it ends in a listed abbreviation. UAX #29
/// only breaks there when a capitalised word follows, so that condition
/// needs no separate check.
///
/// Author: Claude Fable 5
fn sentence_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    for (start, segment) in text.split_sentence_bound_indices() {
        let end = start + segment.len();
        match ranges.last_mut() {
            Some(prev) if ends_with_abbreviation(&text[prev.clone()]) => prev.end = end,
            _ => ranges.push(start..end),
        }
    }
    ranges
}

/// Whether the sentence's last token is a listed abbreviation: the entry
/// itself, at the very end ignoring trailing whitespace, preceded by
/// nothing alphanumeric.
///
/// Author: Claude Fable 5
fn ends_with_abbreviation(sentence: &str) -> bool {
    let trimmed = sentence.trim_end();
    ABBREVIATIONS.iter().any(|abbreviation| {
        trimmed.ends_with(abbreviation)
            && trimmed[..trimmed.len() - abbreviation.len()]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric())
    })
}

/// The sentence's word tokens as ranges into the block text: UAX #29 word
/// bounds, keeping only segments with an alphanumeric character, then
/// joining segments whose gap is exactly one ASCII hyphen so a hyphenated
/// compound is one token. UAX #29 already keeps contractions and
/// number/version strings whole.
///
/// Author: Claude Fable 5
fn token_ranges(text: &str, span: Range<usize>) -> Vec<Range<usize>> {
    let mut tokens: Vec<Range<usize>> = Vec::new();
    for (offset, segment) in text[span.clone()].split_word_bound_indices() {
        if !segment.chars().any(char::is_alphanumeric) {
            continue;
        }
        let start = span.start + offset;
        let end = start + segment.len();
        match tokens.last_mut() {
            Some(prev) if &text[prev.end..start] == "-" => prev.end = end,
            _ => tokens.push(start..end),
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::{ABBREVIATIONS, Sentence, sentences};
    use crate::prose::extract;

    /// Runs the pipeline up to this stage on one markdown string.
    ///
    /// Author: Claude Fable 5
    fn parse(md: &str) -> Vec<Sentence> {
        sentences(md, &extract(md))
    }

    /// Each sentence's token texts, for tests about segmentation and token
    /// shape rather than flags or lines.
    ///
    /// Author: Claude Fable 5
    fn token_texts(md: &str) -> Vec<Vec<String>> {
        parse(md)
            .into_iter()
            .map(|sentence| sentence.words.into_iter().map(|word| word.text).collect())
            .collect()
    }

    /// Author: Claude Fable 5
    #[test]
    fn a_paragraph_splits_into_its_sentences() {
        assert_eq!(
            token_texts("The fog rolled in. It was thick."),
            vec![
                vec!["The", "fog", "rolled", "in"],
                vec!["It", "was", "thick"],
            ],
        );
    }

    /// The measured failure the list rule fixes: without item boundaries
    /// the three unpunctuated bullets fuse into one sentence.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn each_list_item_is_its_own_sentence() {
        let found = parse("- alpha beta\n- gamma delta\n- epsilon zeta");
        assert_eq!(found.len(), 3);
        assert_eq!(
            found.iter().map(|s| s.line).collect::<Vec<_>>(),
            vec![1, 2, 3],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn one_bullet_can_hold_several_sentences() {
        assert_eq!(
            token_texts("- First here. Second here."),
            vec![vec!["First", "here"], vec!["Second", "here"]],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn nested_items_are_boundaries_too() {
        assert_eq!(
            token_texts("- outer text\n  - inner text"),
            vec![vec!["outer", "text"], vec!["inner", "text"]],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn a_soft_break_is_a_space_not_a_boundary() {
        assert_eq!(
            token_texts("one two\nthree four."),
            vec![vec!["one", "two", "three", "four"]],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn a_hard_break_is_a_boundary_without_punctuation() {
        assert_eq!(
            token_texts("one two  \nthree four"),
            vec![vec!["one", "two"], vec!["three", "four"]],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn a_paragraph_end_is_a_boundary_without_punctuation() {
        assert_eq!(
            token_texts("no terminal punctuation\n\nnext paragraph"),
            vec![
                vec!["no", "terminal", "punctuation"],
                vec!["next", "paragraph"],
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn dr_smith_stays_one_sentence() {
        assert_eq!(
            token_texts("Ask Dr. Smith about it."),
            vec![vec!["Ask", "Dr", "Smith", "about", "it"]],
        );
    }

    /// Every entry of the fixed abbreviation list merges the break UAX #29
    /// inserts before a capitalised word.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn every_listed_abbreviation_merges() {
        for abbreviation in ABBREVIATIONS {
            let md = format!("We saw {abbreviation} Smith today.");
            assert_eq!(
                parse(&md).len(),
                1,
                "{abbreviation} should not end a sentence"
            );
        }
    }

    /// `etc.` is deliberately not on the list; it legitimately ends
    /// sentences, and this break is also the premise the merge tests rest
    /// on (UAX #29 does split at abbreviation-period before a capital).
    ///
    /// Author: Claude Fable 5
    #[test]
    fn etc_still_ends_a_sentence() {
        assert_eq!(parse("We packed maps, ropes, etc. Then we left.").len(), 2);
    }

    /// The abbreviation must be a token of its own: a longer word that
    /// merely ends with one does not merge.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn an_abbreviation_inside_a_longer_word_does_not_merge() {
        assert_eq!(parse("He tested libcf. Results improved.").len(), 2);
    }

    /// Author: Claude Fable 5
    #[test]
    fn hyphenated_compounds_are_one_word_hyphen_kept() {
        assert_eq!(
            token_texts("A well-known state-of-the-art fix."),
            vec![vec!["A", "well-known", "state-of-the-art", "fix"]],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn contractions_are_one_word() {
        assert_eq!(
            token_texts("Don't stop; it can't fail."),
            vec![vec!["Don't", "stop", "it", "can't", "fail"]],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn numbers_and_version_strings_are_one_never_complex_word() {
        let found = parse("Version v1.2.3 shipped 42 fixes.");
        assert_eq!(found.len(), 1);
        let flags: Vec<(&str, bool)> = found[0]
            .words
            .iter()
            .map(|word| (word.text.as_str(), word.never_complex))
            .collect();
        assert_eq!(
            flags,
            vec![
                ("Version", false),
                ("v1.2.3", true),
                ("shipped", false),
                ("42", true),
                ("fixes", false),
            ],
        );
    }

    /// A hyphenated compound with a numeric part is not a number: its word
    /// parts stay eligible for complexity.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn a_numeric_part_does_not_excuse_a_hyphenated_compound() {
        let found = parse("A 3-dimensional interface.");
        assert_eq!(found[0].words[1].text, "3-dimensional");
        assert!(!found[0].words[1].never_complex);
    }

    /// A hyphenated suffix on a dotted numeric core is a version
    /// identifier, so prerelease and build forms are never complex too.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn a_version_with_a_prerelease_part_is_never_complex() {
        let found = parse("Releases 1.2.3-beta and 2.0-rc1 shipped.");
        let flagged: Vec<(&str, bool)> = found[0]
            .words
            .iter()
            .map(|word| (word.text.as_str(), word.never_complex))
            .collect();
        assert_eq!(
            flagged,
            vec![
                ("Releases", false),
                ("1.2.3-beta", true),
                ("and", false),
                ("2.0-rc1", true),
                ("shipped", false),
            ],
        );
    }

    /// An all-digit hyphenated token (a date, a phone number) is a
    /// number even though its core is undotted.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn an_all_digit_hyphenated_token_is_never_complex() {
        let found = parse("Released on 2024-01-15 today.");
        assert_eq!(found[0].words[2].text, "2024-01-15");
        assert!(found[0].words[2].never_complex);
    }

    /// Author: Claude Fable 5
    #[test]
    fn a_placeholder_counts_as_one_never_complex_word() {
        let found = parse("Run `cargo test --locked` now.");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].words.len(), 3);
        assert!(found[0].words[1].never_complex);
        assert!(!found[0].words[0].never_complex);
    }

    /// A placeholder joined to other text (across a hyphen or flush
    /// against it) does not spread never-complex onto the compound: in
    /// `x-oriented` the real part stays eligible, and the bare-suffix join
    /// `xs` is likewise unexcused, with the stand-in contributing its own
    /// one syllable as the placeholder contract says.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn a_placeholder_compound_keeps_its_real_parts_eligible() {
        let found = parse("A `foo`-oriented interpretation.");
        assert_eq!(found[0].words[1].text, "x-oriented");
        assert!(!found[0].words[1].never_complex);

        let found = parse("Many `cargo test`s were run.");
        assert_eq!(found[0].words[1].text, "xs");
        assert!(!found[0].words[1].never_complex);
    }

    /// A sentence's line is the line it starts on, taken from the spans the
    /// extraction stage carried through; constructs removed before scoring
    /// (the heading here) still push later lines down.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn sentences_carry_their_source_line() {
        let lines: Vec<usize> = parse("# Title\n\nOne sentence\ncontinues here. Then another.")
            .into_iter()
            .map(|sentence| sentence.line)
            .collect();
        assert_eq!(lines, vec![3, 4]);
    }

    /// A sentence's span quotes it as written in the source: trailing
    /// punctuation included, a placeholder mapped back to the code span it
    /// replaced, an entity left encoded, a soft break kept as the newline.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn sentences_carry_their_source_span() {
        let md = "First one here. Run `cargo test`\nplus fish &amp; chips!";
        let quoted: Vec<&str> = parse(md)
            .into_iter()
            .map(|sentence| &md[sentence.span.start..sentence.span.end])
            .collect();
        assert_eq!(
            quoted,
            vec![
                "First one here.",
                "Run `cargo test`\nplus fish &amp; chips!"
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn input_with_no_prose_yields_no_sentences() {
        assert_eq!(parse("```\nlet x = 1;\n```"), vec![]);
    }
}
