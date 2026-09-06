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
/// first spelling kept, mirroring the refusal line.
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
            let names: Vec<&str> = complex
                .iter()
                .filter(|word| seen.insert(word.text.to_lowercase()))
                .map(|word| word.text.as_str())
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
/// and the limit allows a second line.
///
/// Author: Claude Fable 5
fn refusal_text(refusal: &Refusal, limit: usize) -> String {
    let mut out = format!("no score: {} words (min {FLOOR})\n", refusal.words);
    if limit != 0 && !refusal.complex.is_empty() {
        out.push_str("complex: ");
        out.push_str(&refusal.complex.join(", "));
        out.push('\n');
    }
    out
}

/// The sentence's first [`EXCERPT_WORDS`] words in their shown form,
/// single-spaced, `…` appended when the sentence goes on: one excerpt
/// word per counted word, so the excerpt stays consistent with the
/// `<N>w` count. Whitespace a shown construct carries collapses to
/// single spaces, and each control character that collapsing does not
/// remove is replaced with one `\u{FFFD}`, so a scored document cannot
/// write escape sequences into the caller's terminal.
///
/// Author: Claude Fable 5
fn excerpt(words: &[Word]) -> String {
    let mut out: String = words
        .iter()
        .take(EXCERPT_WORDS)
        .flat_map(|word| word.shown.split_whitespace())
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .map(|c| if c.is_control() { '\u{FFFD}' } else { c })
        .collect();
    if words.len() > EXCERPT_WORDS {
        out.push('…');
    }
    out
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

    /// Author: Claude Fable 5
    #[test]
    fn rendered_report_carries_no_control_bytes() {
        // One ESC sits bare in prose, where tokenisation already drops it;
        // the other hides inside a code span, whose source bytes the
        // excerpt shows, so it must surface as the replacement character.
        // The "day" padding lifts the document over the 100-word floor.
        let md = format!(
            "The \x1b[31mcounterfactual implementation runs `red\x1b[0m` and recalculates {}sea. Bravo {}sea.",
            "day ".repeat(100),
            "day ".repeat(8)
        );
        let found = sentences(&md, &extract(&md));
        let analysis = analyse(&found, 10.0, 10);
        let report = render(&found, &analysis, 10, 10, false);
        assert!(
            !report.chars().any(|c| c.is_control() && c != '\n'),
            "control characters leaked into: {report}"
        );
        assert!(
            report.contains(
                "The 31mcounterfactual implementation runs `red\u{FFFD}[0m` and recalculates day…"
            ),
            "excerpt should drop the bare ESC token-wise and replace the code span's: {report}"
        );
        // Tokenisation splits on the ESC byte, so the complex: list names
        // the token with the colour code's digits attached but never the
        // ESC itself.
        assert!(
            report.contains("complex: 31mcounterfactual"),
            "complex list should split at the ESC byte: {report}"
        );
    }
}
