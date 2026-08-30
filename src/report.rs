//! Report rendering: an [`Analysis`] and its sentences in, the CLI's
//! stdout text out.
//!
//! Every printed character must earn its place in an agent's context
//! window, so the report is compact plain text: a score line, hotspot
//! lines only when the document scores over target, a tail line only when
//! the limit truncates, and a two-line refusal under the floor. Exit
//! codes are the caller's job; [`render`] only builds the text.

use crate::score::{Analysis, FLOOR, Refusal, Scored, printed};
use crate::segment::Sentence;
use crate::syllable::complex_words;

/// How many leading source words a hotspot excerpt shows.
const EXCERPT_WORDS: usize = 8;

/// Renders an analysis as the report text, one trailing newline per line.
///
/// `source` is the markdown the sentences were segmented from; each
/// hotspot quotes its sentence from it as written. `with_lines` prints
/// `L<N>` source line numbers, which only `--file` input can honour.
/// `limit` 0 means score only: the refusal's `complex:` line is dropped
/// (the scored path already carries no hotspots).
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
///     render(md, &found, &analysis, 10, 10, false),
///     "no score: 9 words (min 100)\ncomplex: paradigm, beautiful\n"
/// );
/// ```
///
/// Author: Claude Fable 5
pub fn render(
    source: &str,
    sentences: &[Sentence],
    analysis: &Analysis,
    target: usize,
    limit: usize,
    with_lines: bool,
) -> String {
    match analysis {
        Analysis::Scored(scored) => scored_text(source, sentences, scored, target, with_lines),
        Analysis::Refused(refusal) => refusal_text(refusal, limit),
    }
}

/// The score line, the hotspot lines in document order, and the tail.
///
/// Author: Claude Fable 5
fn scored_text(
    source: &str,
    sentences: &[Sentence],
    scored: &Scored,
    target: usize,
    with_lines: bool,
) -> String {
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
        out.push_str(&excerpt(&source[sentence.span.clone()]));
        out.push('"');
        let complex = complex_words(sentence);
        if !complex.is_empty() {
            let names: Vec<&str> = complex.iter().map(|word| word.text.as_str()).collect();
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

/// The sentence's first [`EXCERPT_WORDS`] whitespace-separated words as
/// written in the source, single-spaced, `…` appended when the sentence
/// goes on.
///
/// Author: Claude Fable 5
fn excerpt(sentence: &str) -> String {
    let mut words = sentence.split_whitespace();
    let shown: Vec<&str> = words.by_ref().take(EXCERPT_WORDS).collect();
    let mut out = shown.join(" ");
    if words.next().is_some() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Author: Claude Fable 5
    #[test]
    fn excerpt_shows_eight_words_and_marks_truncation() {
        assert_eq!(excerpt("one two three"), "one two three");
        assert_eq!(
            excerpt("one two three four five six seven eight"),
            "one two three four five six seven eight"
        );
        assert_eq!(
            excerpt("one two three four five six seven eight nine"),
            "one two three four five six seven eight…"
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn excerpt_single_spaces_soft_wrapped_source() {
        assert_eq!(excerpt("wrapped\nacross   lines"), "wrapped across lines");
    }
}
