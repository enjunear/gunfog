//! Calibration against Gunning's own worked example: the Maugham passage
//! from *The Technique of Clear Writing*, revised edition 1968, pp. 38-39.
//!
//! Gunning counts the passage as 118 words, 8 sentences, and 15 hard words
//! (12.7%), and publishes a Fog Index of 10.9. The exact formula on his own
//! counts gives 11.0: he rounds the average sentence length to 14.5 before
//! adding, where 118/8 is 14.75, and 0.4 x (14.75 + 1500/118) = 10.98. The
//! 0.1 delta is his intermediate rounding, not a counting disagreement, and
//! is accepted per `SPEC.md` "Scoring".

use gunfog::prose::extract;
use gunfog::report::render;
use gunfog::score::{Analysis, analyse};
use gunfog::segment::sentences;
use gunfog::syllable::complex_words;

/// The passage as the 1968 edition prints it, with one deliberate change:
/// Gunning notes that "the third sentence is actually three complete
/// thoughts linked by a comma, in one instance, and a semicolon in the
/// other. These should be counted as separate sentences." gunfog segments
/// what is written, not what should have been, so this test encodes his
/// sentence boundaries as periods (`Hume. And`, `you. But`) to feed the
/// formula the same 8 sentences he counted.
const MAUGHAM: &str = "\
I have never had much patience with the writers who claim from the reader an
effort to understand their meaning. You have only to go to the great
philosophers to see that it is possible to express with lucidity the most
subtle reflections. You may find it difficult to understand the thought of
Hume. And if you have no philosophical training its implications will
doubtless escape you. But no one with any education at all can fail to
understand exactly what the meaning of each sentence is. Few people have
written English with more grace than Berkeley. There are two sorts of
obscurity you will find in writers. One is due to negligence and the other to
willfulness.
";

/// Gunning's published word count per sentence: "The number of words in the
/// sentences of this passage is as follows: 20-23-11-13-20-10-11-10."
const GUNNING_SENTENCE_WORDS: [usize; 8] = [20, 23, 11, 13, 20, 10, 11, 10];

/// The pipeline reproduces Gunning's segmentation and word counts exactly.
///
/// Author: Claude Fable 5
#[test]
fn segmentation_matches_gunnings_published_counts() {
    let found = sentences(MAUGHAM, &extract(MAUGHAM));
    let words: Vec<usize> = found.iter().map(|s| s.words.len()).collect();
    assert_eq!(words, GUNNING_SENTENCE_WORDS);
    assert_eq!(words.iter().sum::<usize>(), 118);
}

/// The full pipeline on the passage: one known divergence from Gunning's
/// hand count, documented rather than hidden.
///
/// Gunning italicises 15 hard words. The rule-based syllable counter
/// (`docs/research/syllable-counting.md` §4, 96.1% held-out accuracy)
/// counts `people` as 3 syllables where it has 2, so gunfog finds those 15
/// plus `people` and scores the passage 11.3 instead of 11.0. `SPEC.md`
/// "Syllable counting" accepts rule-counter error and names the CMU-derived
/// word list as the additive first-check layer if golden tests show worse.
///
/// Author: Claude Fable 5
#[test]
fn pipeline_scores_the_passage_within_the_documented_divergence() {
    let found = sentences(MAUGHAM, &extract(MAUGHAM));
    let complex: Vec<&str> = found
        .iter()
        .flat_map(|s| complex_words(s).into_iter().map(|w| w.text.as_str()))
        .collect();
    // Gunning's 15, in document order, with `people` the counter's one miss.
    assert_eq!(
        complex,
        [
            "understand",
            "philosophers",
            "possible",
            "lucidity",
            "reflections",
            "difficult",
            "understand",
            "philosophical",
            "implications",
            "education",
            "understand",
            "exactly",
            "people", // 2 syllables; the rule counter's known miss
            "obscurity",
            "negligence",
            "willfulness",
        ]
    );
    let analysis = analyse(&found, 10.0, 10);
    let report = render(&found, &analysis, 10, 10, false);
    assert!(
        report.starts_with("fog: 11.3 (target 10)\n"),
        "report was:\n{report}"
    );
}

/// The formula on Gunning's own counts scores 11.0 against his published
/// 10.9.
///
/// Built as a synthetic document shaped exactly like his hand count: his
/// per-sentence word counts, 15 complex words distributed as he italicised
/// them (1-4-2-2-3-0-1-2), fed through the public pipeline. `beautiful`
/// carries three syllables, `day` one; `Day` opens each sentence so the
/// proper-name exclusion cannot excuse the filler.
///
/// Author: Claude Fable 5
#[test]
fn formula_scores_gunnings_counts_eleven_point_zero() {
    const COMPLEX_PER_SENTENCE: [usize; 8] = [1, 4, 2, 2, 3, 0, 1, 2];
    let doc: String = GUNNING_SENTENCE_WORDS
        .iter()
        .zip(COMPLEX_PER_SENTENCE)
        .map(|(&words, complex)| {
            let mut tokens = vec!["Day"];
            tokens.extend(std::iter::repeat_n("beautiful", complex));
            tokens.extend(std::iter::repeat_n("day", words - complex - 1));
            format!("{}.", tokens.join(" "))
        })
        .collect::<Vec<_>>()
        .join(" ");

    let found = sentences(&doc, &extract(&doc));
    assert_eq!(found.len(), 8);
    assert_eq!(found.iter().map(|s| s.words.len()).sum::<usize>(), 118);
    assert_eq!(
        found.iter().map(|s| complex_words(s).len()).sum::<usize>(),
        15
    );

    let analysis = analyse(&found, 10.0, 10);
    let Analysis::Scored(scored) = &analysis else {
        panic!("118 words must score: {analysis:?}");
    };
    // 0.4 x (118/8 + 100 x 15/118) = 10.9847...
    assert!((scored.fog - 10.984_745_762_711_864).abs() < 1e-9);
    let report = render(&found, &analysis, 10, 10, false);
    assert!(
        report.starts_with("fog: 11.0 (target 10)\n"),
        "report was:\n{report}"
    );
}
