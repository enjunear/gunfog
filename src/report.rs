//! Report rendering: an [`Analysis`] and its sentences in, the CLI's
//! stdout text out.
//!
//! Every printed character must earn its place in an agent's context
//! window, so the report is compact plain text: a score line, hotspot
//! lines only when the document scores over target, a tail line only when
//! the limit truncates, and a two-line refusal under the floor. Exit
//! codes are the caller's job; [`render`] only builds the text.

use crate::score::{Analysis, FLOOR, Refusal, Scored, printed};
use crate::segment::{Sentence, Word};
use crate::syllable::complex_words;
use std::collections::HashSet;

/// How many leading words a hotspot excerpt shows.
const EXCERPT_WORDS: usize = 8;

/// Renders an analysis as the report text, one trailing newline per line.
///
/// Each hotspot quotes its sentence's leading words as segmentation shows
/// them, so the excerpt is extracted prose with placeholders swapped back
/// for their constructs. `with_lines` prints `L<N>` source line numbers,
/// which only `--file` input can honour. `limit` 0 means score only: the
/// refusal's `complex:` line is dropped (the scored path already carries
/// no hotspots).
///
/// ```
/// use gunfog::prose::extract;
/// use gunfog::report::render;
/// use gunfog::score::analyse;
/// use gunfog::segment::sentences;
///
/// let md = "Too short to score, though the paradigm is beautiful.";
/// let found = sentences(md, &extract(md));
/// let analysis = analyse(&found, 10.0, 10);
/// assert_eq!(
///     render(&found, &analysis, 10, 10, false),
///     "no score: 9 words (min 100)\ncomplex: paradigm, beautiful\n"
/// );
/// ```
///
/// Author: Claude Fable 5
pub fn render(
    sentences: &[Sentence],
    analysis: &Analysis,
    target: usize,
    limit: usize,
    with_lines: bool,
) -> String {
    match analysis {
        Analysis::Scored(scored) => scored_text(sentences, scored, target, with_lines),
        Analysis::Refused(refusal) => refusal_text(refusal, limit),
    }
}

/// The score line, the hotspot lines in document order, and the tail.
///
/// Each hotspot's `complex:` list is deduplicated case-insensitively,
/// first spelling kept, mirroring the refusal line, and each word is
/// [`sanitise`]d on its way out.
///
/// Author: Claude Fable 5
fn scored_text(sentences: &[Sentence], scored: &Scored, target: usize, with_lines: bool) -> String {
    let mut out = format!("fog: {:.1} (target {target})\n", printed(scored.fog));
    for hotspot in &scored.hotspots {
        let sentence = &sentences[hotspot.sentence_index];
        out.push_str(&format!(
            "{:+.2} {}w ",
            hotspot.contribution,
            sentence.words.len()
        ));
        if with_lines {
            out.push_str(&format!("L{} ", sentence.line));
        }
        out.push('"');
        out.push_str(&excerpt(&sentence.words));
        out.push('"');
        let complex = complex_words(sentence);
        if !complex.is_empty() {
            let mut seen = HashSet::new();
            let names: Vec<String> = complex
                .iter()
                .filter(|word| seen.insert(word.text.to_lowercase()))
                .map(|word| sanitise(&word.text))
                .collect();
            out.push_str(" complex: ");
            out.push_str(&names.join(", "));
        }
        out.push('\n');
    }
    if let Some(truncation) = &scored.truncation {
        out.push_str(&format!(
            "+{} more, {:.1} grades remaining\n",
            truncation.hidden, truncation.remaining
        ));
    }
    out
}

/// The refusal: the word count always, the complex words when any exist
/// and the limit allows a second line. Each word is [`sanitise`]d on its
/// way out.
///
/// Author: Claude Fable 5
fn refusal_text(refusal: &Refusal, limit: usize) -> String {
    let mut out = format!("no score: {} words (min {FLOOR})\n", refusal.words);
    if limit != 0 && !refusal.complex.is_empty() {
        let names: Vec<String> = refusal.complex.iter().map(|word| sanitise(word)).collect();
        out.push_str("complex: ");
        out.push_str(&names.join(", "));
        out.push('\n');
    }
    out
}

/// The sentence's first [`EXCERPT_WORDS`] words in their shown form,
/// single-spaced, `…` appended when the sentence goes on: one excerpt
/// word per counted word, so the excerpt stays consistent with the
/// `<N>w` count. Whitespace a shown construct carries collapses to
/// single spaces, and what collapsing does not remove is [`sanitise`]d.
///
/// Author: Claude Fable 5
fn excerpt(words: &[Word]) -> String {
    let mut out = sanitise(
        &words
            .iter()
            .take(EXCERPT_WORDS)
            .flat_map(|word| word.shown.split_whitespace())
            .collect::<Vec<&str>>()
            .join(" "),
    );
    if words.len() > EXCERPT_WORDS {
        out.push('…');
    }
    out
}

/// Replaces each Unicode control (`Cc`) or format (`Cf`) character in
/// document-derived text with one `\u{FFFD}`, one replacement per
/// character. Every stretch of the report that quotes the document (the
/// excerpt and both `complex:` print sites) passes through here, so
/// output safety never rests on what tokenisation happens to split: a
/// scored document cannot write escape sequences into the caller's
/// terminal, reorder a line with a bidi override, or hide words with
/// zero-width characters.
///
/// Author: Claude Fable 5
fn sanitise(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() || is_format(c) {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .collect()
}

/// Whether the character is in Unicode general category `Cf` (Format):
/// the bidi overrides and isolates, the zero-width characters, the soft
/// hyphen, and the rest of the class. The standard library has no
/// category query, so the ranges are transcribed from UnicodeData.txt
/// 16.0.0.
///
/// Author: Claude Fable 5
fn is_format(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prose::extract;
    use crate::score::analyse;
    use crate::segment::sentences;

    /// Author: Claude Fable 5
    #[test]
    fn hotspot_complex_dedupe_is_per_hotspot_case_insensitive_keeping_first_spelling() {
        // Both long sentences open with "Beautiful" (sentence-initial, so
        // the proper-name exclusion cannot excuse it) and repeat it
        // lowercase. Within a hotspot the repeat folds into the opener,
        // but the word still prints on every hotspot that has it. The
        // "day" padding lifts the document over the 100-word floor.
        let md = format!(
            "Beautiful beautiful {}sea. Beautiful beautiful {}sea. Bravo {}sea.",
            "day ".repeat(57),
            "day ".repeat(37),
            "day ".repeat(8)
        );
        let found = sentences(&md, &extract(&md));
        let analysis = analyse(&found, 10.0, 10);
        let report = render(&found, &analysis, 10, 10, false);
        let lines: Vec<&str> = report.lines().collect();
        assert_eq!(lines.len(), 3, "score line and two hotspots:\n{report}");
        for hotspot in &lines[1..] {
            assert!(
                hotspot.ends_with("complex: Beautiful"),
                "expected the first spelling alone, line was: {hotspot}"
            );
        }
    }

    /// The first sentence's words, for excerpt tests: the excerpt's
    /// input is the word tokens, one excerpt word per counted word.
    ///
    /// Author: Claude Fable 5
    fn words_of(md: &str) -> Vec<Word> {
        sentences(md, &extract(md)).remove(0).words
    }

    /// Author: Claude Fable 5
    #[test]
    fn excerpt_shows_eight_words_and_marks_truncation() {
        assert_eq!(excerpt(&words_of("one two three")), "one two three");
        assert_eq!(
            excerpt(&words_of("one two three four five six seven eight")),
            "one two three four five six seven eight"
        );
        assert_eq!(
            excerpt(&words_of("one two three four five six seven eight nine")),
            "one two three four five six seven eight…"
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn excerpt_single_spaces_soft_wrapped_source() {
        assert_eq!(
            excerpt(&words_of("wrapped\nacross   lines")),
            "wrapped across lines"
        );
    }

    /// The excerpt is extracted prose, so markup the extraction removes
    /// (emphasis markers, link targets) can never appear in it.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn excerpt_renders_extracted_prose_not_source_markup() {
        assert_eq!(
            excerpt(&words_of(
                "The domain was **military technical** [orders](https://eric.ed.gov/?id=ED205915) then."
            )),
            "The domain was military technical orders then"
        );
    }

    /// A placeholder word appears as the construct it replaced, as
    /// written in the source: a code span with its backticks, a bare URL
    /// as the URL. The stand-in text never appears.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn excerpt_shows_a_placeholder_as_the_construct_it_replaced() {
        assert_eq!(
            excerpt(&words_of("Run `cargo test` before every push.")),
            "Run `cargo test` before every push"
        );
        assert_eq!(
            excerpt(&words_of("See https://example.com/x for details.")),
            "See https://example.com/x for details"
        );
        assert_eq!(
            excerpt(&words_of("A `foo`-oriented interpretation.")),
            "A `foo`-oriented interpretation"
        );
    }

    /// Whitespace inside a shown construct collapses to single spaces, so
    /// a code span wrapping across a line stays one excerpt word apart.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn excerpt_collapses_whitespace_inside_a_construct() {
        assert_eq!(excerpt(&words_of("Run `a   b` now.")), "Run `a b` now");
    }

    /// Inner quote characters pass through unescaped: the excerpt field
    /// is not parseable by splitting on quotes, and the contract says so
    /// rather than spending tokens escaping a case extraction makes rare.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn excerpt_passes_inner_quotes_unescaped() {
        assert_eq!(
            excerpt(&words_of("Run `echo \"hi\"` now.")),
            "Run `echo \"hi\"` now"
        );
    }

    /// Control characters can only reach an excerpt through a shown
    /// construct's source bytes; each one becomes one replacement.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn excerpt_replaces_control_characters_one_for_one() {
        assert_eq!(
            excerpt(&words_of("Run `\x1b[31m` now.")),
            "Run `\u{FFFD}[31m` now"
        );
        assert_eq!(
            excerpt(&words_of("Run `\x1b\x1b` and `\x00` now.")),
            "Run `\u{FFFD}\u{FFFD}` and `\u{FFFD}` now"
        );
    }

    /// Format characters reach the report two ways: most have
    /// Word_Break Format, which UAX #29 rule WB4 keeps mid-token, and
    /// any of them can hide in a shown construct's source bytes (the
    /// zero-width space is excluded from WB4, so the code span is its
    /// only route). Each becomes one replacement.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn excerpt_replaces_format_characters_one_for_one() {
        assert_eq!(
            excerpt(&words_of("The coun\u{202E}terfactual case.")),
            "The coun\u{FFFD}terfactual case"
        );
        assert_eq!(
            excerpt(&words_of("The soft\u{00AD}hyphen stays mid-word.")),
            "The soft\u{FFFD}hyphen stays mid-word"
        );
        assert_eq!(
            excerpt(&words_of("Run `a\u{200B}b` now.")),
            "Run `a\u{FFFD}b` now"
        );
        assert_eq!(
            excerpt(&words_of("A dou\u{202E}\u{202E}bled override.")),
            "A dou\u{FFFD}\u{FFFD}bled override"
        );
    }

    /// Under the floor the `complex:` line is the whole actionable
    /// output, and it quotes document text like any other print site.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn refusal_complex_line_replaces_format_characters() {
        let md = "The coun\u{202E}terfactual implementation.";
        let found = sentences(md, &extract(md));
        let analysis = analyse(&found, 10.0, 10);
        let report = render(&found, &analysis, 10, 10, false);
        assert!(
            report.contains("complex: coun\u{FFFD}terfactual, implementation"),
            "refusal line should replace the override: {report}"
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn rendered_report_carries_no_control_or_format_characters() {
        // The ESC hides inside a code span, whose source bytes the
        // excerpt shows; the bidi override sits inside a complex word,
        // where tokenisation keeps it (UAX #29 rule WB4), so it reaches
        // both the excerpt and the complex: list. The "day" padding
        // lifts the document over the 100-word floor.
        let md = format!(
            "The coun\u{202E}terfactual implementation runs `red\x1b[0m` and recalculates {}sea. Bravo {}sea.",
            "day ".repeat(100),
            "day ".repeat(8)
        );
        let found = sentences(&md, &extract(&md));
        let analysis = analyse(&found, 10.0, 10);
        let report = render(&found, &analysis, 10, 10, false);
        assert!(
            !report
                .chars()
                .any(|c| (c.is_control() && c != '\n') || is_format(c)),
            "control or format characters leaked into: {report}"
        );
        assert!(
            report.contains(
                "The coun\u{FFFD}terfactual implementation runs `red\u{FFFD}[0m` and recalculates day…"
            ),
            "excerpt should replace the override and the code span's ESC: {report}"
        );
        assert!(
            report.contains("complex: coun\u{FFFD}terfactual"),
            "the complex: list should carry the replacement, not the raw override: {report}"
        );
    }
}
