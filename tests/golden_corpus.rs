//! The hand-counted golden corpus: five verbatim passages from this repo's
//! research notes under `tests/corpus/`, each asserted down to its sentence
//! word counts, its complex-word list, and its printed score.
//!
//! The hand counts follow `SPEC.md` exactly: prose extraction, placeholder
//! words for inline code spans, UAX #29 segmentation with the list-item and
//! paragraph boundaries, and the 29-rule syllable counter. Where that rule
//! counter disagrees with dictionary syllables the divergence is named in
//! the sample's test rather than hidden; the counting worksheet with every
//! per-word judgement is in the merge request that landed this file.

use gunfog::prose::extract;
use gunfog::report::render;
use gunfog::score::analyse;
use gunfog::segment::sentences;
use gunfog::syllable::complex_words;

/// Asserts one corpus sample's full shape: per-sentence word counts,
/// the complex words in document order, and the printed score line.
///
/// Author: Claude Fable 5
fn assert_sample(source: &str, sentence_words: &[usize], complex: &[&str], score_line: &str) {
    let found = sentences(source, &extract(source));
    let words: Vec<usize> = found.iter().map(|s| s.words.len()).collect();
    assert_eq!(words, sentence_words, "per-sentence word counts");
    let named: Vec<&str> = found
        .iter()
        .flat_map(|s| complex_words(s).into_iter().map(|w| w.text.as_str()))
        .collect();
    assert_eq!(named, complex, "complex words in document order");
    let analysis = analyse(&found, 10.0, 10);
    let report = render(&found, &analysis, 10, 10, false);
    assert!(
        report.starts_with(score_line),
        "expected {score_line:?}, report was:\n{report}"
    );
}

/// `docs/research/hyphenated-words.md`, the sourcing preamble. 115 words,
/// 6 sentences, 10 complex: fog 11.14, printed 11.1.
///
/// `Every` reads as 2 syllables in fast speech, but CMUdict — the
/// counter's reference — gives 3 (`EH1 V ER0 IY0`), and the rule counter
/// agrees, so it lands in the complex list (sentence-initial, so the
/// proper-name exclusion cannot excuse it either way).
///
/// Author: Claude Fable 5
#[test]
fn gunning_snippets_sample() {
    assert_sample(
        include_str!("corpus/gunning-snippets.md"),
        &[20, 18, 21, 29, 11, 16],
        &[
            "Companion",
            "hyphenation",
            "implementations",
            "editions",
            "lending-restricted",
            "Every",
            "verbatim",
            "edition",
            "revision",
            "Measurements",
        ],
        "fog: 11.1 (target 10)\n",
    );
}

/// `docs/research/naming-collisions.md`, the verdict paragraph. 114 words,
/// 5 sentences, 18 complex: fog 15.44, printed 15.4.
///
/// The opening sentence ends at "with zero hits of any kind." and the next
/// one opens with the `gfog` code span. That break exists because the
/// placeholder's stand-in is capitalised; with a lowercase stand-in the
/// two read as one 54-word sentence.
///
/// Two judgement calls a reader might dispute, both aligned with the
/// rules: `every` (twice) is 3 syllables per CMUdict (`EH1 V ER0 IY0`)
/// though speech often compresses it to 2; `namespaces` reads as
/// namespace + s lifting a 2-syllable word to 3, but the `-es` excusal
/// tests the truncated stem `namespac`, which keeps its middle `e` audible
/// and stays 3, so the word stays complex.
///
/// Author: Claude Fable 5
#[test]
fn naming_verdict_sample() {
    assert_sample(
        include_str!("corpus/naming-verdict.md"),
        &[11, 43, 24, 26, 10],
        &[
            "everywhere",
            "every",
            "registry",
            "distribution",
            "occupations",
            "username",
            "unrelated",
            "repository",
            "gradient-free-optimization",
            "library",
            "publishing",
            "formula",
            "namespaces",
            "every",
            "general-web",
            "literal",
            "collision",
            "deciding",
        ],
        "fog: 15.4 (target 10)\n",
    );
}

/// `docs/research/fog-vs-flesch-kincaid.md`, the recommendation and the
/// caveat. 117 words, 7 sentences, 14 complex: fog 11.47, printed 11.5.
///
/// One real rule-counter miss: `carrying` is 3 syllables per CMUdict
/// (`K AE1 R IY0 IH0 NG`; `-ing` is deliberately never excused), but the
/// counter reads `yi` as one vowel group and counts 2, so it never
/// reaches the list.
///
/// Author: Claude Fable 5
#[test]
fn formula_recommendation_sample() {
    assert_sample(
        include_str!("corpus/formula-recommendation.md"),
        &[18, 18, 21, 26, 13, 9, 12],
        &[
            "Recommendation",
            "actually",
            "segmentation",
            "attribution",
            "explicit",
            "policy",
            "minimum",
            "secondary",
            "syllable",
            "formulas",
            "difficulty",
            "correlated",
            "formulas",
            "thermometer",
        ],
        "fog: 11.5 (target 10)\n",
    );
}

/// `docs/research/syllable-counting.md`, finding 1. 111 words, 6
/// sentences, 13 complex: fog 12.08, printed 12.1.
///
/// The bold list marker `**1.**` segments as its own one-word sentence
/// (digit, period, space, capital is a UAX #29 boundary), and the sentence
/// ending `calls**.` runs on into `npm ...` because a lowercase follower
/// suppresses the break. Both are the documented segmentation rules
/// applied to real text, not artefacts to be papered over. `Every` is the
/// same CMU-backed 3-syllable count as in `gunning_snippets_sample`.
///
/// Author: Claude Fable 5
#[test]
fn boundary_finding_sample() {
    assert_sample(
        include_str!("corpus/boundary-finding.md"),
        &[1, 17, 16, 27, 16, 34],
        &[
            "boundary",
            "easier",
            "syllable",
            "nobody",
            "Every",
            "accuracy",
            "boundary",
            "accuracy",
            "syllable",
            "comparable",
            "deliberately",
            "absolute",
            "dictionary",
        ],
        "fog: 12.1 (target 10)\n",
    );
}

/// `docs/research/existing-readability-tools.md`, the textstat survey
/// bullets. 155 words, 12 sentences, 10 complex: fog 7.75, printed 7.7.
/// The one under-target sample, so the report is the score line alone.
///
/// Every list item's start and end are sentence boundaries, and each of
/// the twenty inline code spans counts as one placeholder word. Two of
/// those spans open a sentence and carry its boundary on the strength of
/// the capitalised stand-in: the third bullet breaks after "the canonical
/// one." and again before "`following` and `interesting` are not". One
/// rule-counter miss: `syllables` (3 per CMUdict, twice in the passage) —
/// the plural defeats the consonant+`le` correction that counts the
/// singular correctly, so the counter reads 2 and the word never reaches
/// the list.
///
/// Author: Claude Fable 5
#[test]
fn textstat_survey_sample() {
    let source = include_str!("corpus/textstat-survey.md");
    assert_sample(
        source,
        &[2, 17, 3, 8, 21, 14, 14, 15, 10, 24, 8, 19],
        &[
            "inspecting",
            "distribution",
            "canonical",
            "lowercases",
            "dependency",
            "inherit",
            "actual",
            "primitive",
            "ecosystem",
            "positions",
        ],
        "fog: 7.7 (target 10)\n",
    );
    // Under target: the whole report is the score line.
    let found = sentences(source, &extract(source));
    let analysis = analyse(&found, 10.0, 10);
    assert_eq!(
        render(&found, &analysis, 10, 10, false),
        "fog: 7.7 (target 10)\n"
    );
}
