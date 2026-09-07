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
use crate::syllable::{complex_fragments, syllables};
use std::cmp::Reverse;
use std::collections::HashSet;

/// How many leading words a hotspot excerpt shows.
const EXCERPT_WORDS: usize = 8;

/// How many names a hotspot's `complex:` list prints. A longer list is a
/// transcript of the sentence's polysyllables rather than a rewriting
/// instruction, and it is most of what the report costs: scoring
/// `docs/research/fog-vs-flesch-kincaid.md` at target 10 prints 1724
/// bytes uncapped against 1227 capped.
const COMPLEX_NAMES: usize = 3;

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
/// first spelling kept, mirroring the refusal line, then capped by
/// [`complex_list`]; each name is [`sanitise`]d on its way out.
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
        let complex = complex_fragments(sentence);
        if !complex.is_empty() {
            let mut seen = HashSet::new();
            let unique: Vec<&str> = complex
                .iter()
                .copied()
                .filter(|fragment| seen.insert(fragment.to_lowercase()))
                .collect();
            out.push_str(" complex: ");
            out.push_str(&complex_list(&unique));
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

/// The refusal: the word count always, the complex names when any exist
/// and the limit allows a second line. Each name is [`sanitise`]d on its
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

/// The `complex:` list a hotspot prints, from its deduplicated complex
/// names: at most [`COMPLEX_NAMES`] of them, the ones with the most
/// syllables, joined in document order with `…` appended when the cap
/// hid something.
///
/// Ranking and printing are separate orders on purpose. The cap keeps the
/// words hardest for a reader, which is where a rewrite pays best; the
/// list still reads in document order, so each name lands where the
/// excerpt shows it. The sort is stable, so equal syllable counts keep
/// document order and the earlier word wins the last slot.
///
/// Author: Claude Fable 5
fn complex_list(unique: &[&str]) -> String {
    let mut kept: Vec<usize> = (0..unique.len()).collect();
    kept.sort_by_key(|&index| Reverse(name_syllables(unique[index])));
    kept.truncate(COMPLEX_NAMES);
    kept.sort_unstable();
    let mut out = kept
        .iter()
        .map(|&index| sanitise(unique[index]))
        .collect::<Vec<String>>()
        .join(", ");
    if unique.len() > COMPLEX_NAMES {
        out.push('…');
    }
    out
}

/// What one printed name costs a reader: its hyphen-separated parts'
/// syllables added together, so the whole compound counts.
///
/// [`syllables`] drops the hyphen before counting, which fuses the vowels
/// either side of it: `co-operation` comes back 4 where a reader meets 5,
/// and `re-examine` 3 against 4. Splitting first counts each part on its
/// own and adds them, which is also the split the complex-word rule
/// already makes. A name without a hyphen is one part and unaffected.
///
/// Author: Claude Fable 5
fn name_syllables(name: &str) -> usize {
    name.split('-').map(syllables).sum()
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

    /// The one hotspot line of a document opening with `opener`, padded
    /// with one-syllable filler past the 100-word floor and closed by a
    /// short simple sentence that never makes the hotspot list.
    ///
    /// Author: Claude Fable 5
    fn hotspot_of(opener: &str) -> String {
        let md = format!(
            "{opener} {}sea. Bravo {}sea.",
            "day ".repeat(90),
            "day ".repeat(8)
        );
        let found = sentences(&md, &extract(&md));
        let analysis = analyse(&found, 10.0, 10);
        let report = render(&found, &analysis, 10, 10, false);
        report
            .lines()
            .nth(1)
            .expect("the padded first sentence should be a hotspot")
            .to_string()
    }

    /// Syllable counts: beautiful 3, counterfactual 5, implementation 5,
    /// recalculates 4. Document order opens with the least demanding
    /// word, so a list that kept document order alone would name it.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn hotspot_complex_list_names_the_three_most_demanding_words() {
        let hotspot = hotspot_of("The beautiful counterfactual implementation recalculates");
        assert!(
            hotspot.ends_with("complex: counterfactual, implementation, recalculates…"),
            "line was: {hotspot}"
        );
    }

    /// At or under the cap the list is what it has always been: every
    /// complex word, in document order, with nothing marking a
    /// truncation that did not happen.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn hotspot_complex_list_under_the_cap_keeps_document_order_and_no_marker() {
        assert!(
            hotspot_of("The beautiful counterfactual")
                .ends_with("complex: beautiful, counterfactual"),
            "two words should print in document order, unmarked"
        );
        assert!(
            hotspot_of("The beautiful counterfactual implementation")
                .ends_with("complex: beautiful, counterfactual, implementation"),
            "exactly three words hide nothing, so nothing marks them"
        );
    }

    /// Syllable counts: counterfactual 5, then attribution, recalculates
    /// and segmentation all 4. Three words compete for two slots and the
    /// earlier two win.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn hotspot_complex_cap_breaks_syllable_ties_by_document_order() {
        let hotspot = hotspot_of("The counterfactual attribution recalculates segmentation");
        assert!(
            hotspot.ends_with("complex: counterfactual, attribution, recalculates…"),
            "line was: {hotspot}"
        );
    }

    /// A hyphenated name is ranked on the whole compound, not on the part
    /// that made it complex. The complex-word rule splits on the hyphen
    /// so a compound of short parts is not counted as one long word, but
    /// a reader still meets every syllable: `well-documented` is five
    /// against `segmentation`'s four, so it takes the last slot.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn hotspot_complex_cap_ranks_a_hyphenated_name_on_the_whole_compound() {
        let hotspot = hotspot_of("The attribution recalculates segmentation well-documented");
        assert!(
            hotspot.ends_with("complex: attribution, recalculates, well-documented…"),
            "line was: {hotspot}"
        );
    }

    /// The parts are added, not counted through the whole string, so the
    /// hyphen's neighbouring vowels stay apart. `co-operation` is five
    /// syllables and outranks `segmentation`'s four; counting the string
    /// whole would fuse `co` into `operation` and score it four, leaving
    /// `segmentation` in the list on document order instead.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn hotspot_complex_cap_adds_a_hyphenated_name_rather_than_fusing_it() {
        let hotspot = hotspot_of("The attribution recalculates segmentation co-operation");
        assert!(
            hotspot.ends_with("complex: attribution, recalculates, co-operation…"),
            "line was: {hotspot}"
        );
    }

    /// A repeat is folded away before the cap counts, so it neither
    /// spends a slot nor marks the list truncated.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn hotspot_complex_dedupe_happens_before_the_cap() {
        let hotspot = hotspot_of("The beautiful beautiful counterfactual implementation");
        assert!(
            hotspot.ends_with("complex: beautiful, counterfactual, implementation"),
            "line was: {hotspot}"
        );
    }

    /// Under the floor the `complex:` line is the whole actionable
    /// output, so the cap does not apply to it.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn refusal_complex_line_is_uncapped() {
        let md = "The beautiful counterfactual implementation recalculates documentation.";
        let found = sentences(md, &extract(md));
        let analysis = analyse(&found, 10.0, 10);
        assert_eq!(
            render(&found, &analysis, 10, 10, false),
            "no score: 6 words (min 100)\n\
             complex: beautiful, counterfactual, implementation, recalculates, documentation\n"
        );
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
