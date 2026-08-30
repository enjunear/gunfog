//! Scoring, hotspot attribution, and the short-text floor: sentences in, a
//! structured result out.
//!
//! The document fog score is Gunning's formula over the whole prose. Each
//! sentence's contribution is the leave-one-out counterfactual
//! `Fog(doc) - Fog(doc minus this sentence)`, computed O(1) per sentence
//! from document running totals; scoring each sentence on its own is the
//! wrong signal (see `docs/research/fog-vs-flesch-kincaid.md`). Below the
//! 100-word floor no score exists and [`analyse`] refuses, still naming the
//! complex words. Rendering the result as text is the report stage's job.

use std::collections::HashSet;

use crate::segment::Sentence;
use crate::syllable::complex_words;

/// The floor: the minimum prose length that receives a fog score, exact
/// and fixed. Exactly 100 words scores; 99 refuses.
pub(crate) const FLOOR: usize = 100;

/// The outcome of scoring a document: a fog score, or a refusal when the
/// prose is under the floor.
///
/// Author: Claude Fable 5
#[derive(Debug, Clone, PartialEq)]
pub enum Analysis {
    /// The document reached the floor and was scored.
    Scored(Scored),
    /// The document was under the floor; no fog score exists.
    Refused(Refusal),
}

impl Analysis {
    /// The exit-code judgement: `true` exactly when the run exits 0.
    ///
    /// A scored document passes at or under target; a refusal never
    /// passes. The caller still exits 2 for a run that never produced an
    /// `Analysis` at all.
    ///
    /// Author: Claude Fable 5
    pub fn passes(&self) -> bool {
        match self {
            Analysis::Scored(scored) => !scored.over_target,
            Analysis::Refused(_) => false,
        }
    }
}

/// A scored document: the fog score and the hotspots that drive it.
///
/// Author: Claude Fable 5
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    /// The document fog score, unrounded.
    pub fog: f64,
    /// Whether the score exceeds the target, judged on the score as the
    /// report prints it (one decimal), so the score line and the exit
    /// code can never disagree. Only an over-target document has hotspots.
    pub over_target: bool,
    /// The selected hotspots, in document order. Empty when the document
    /// is at or under target, and when `--limit 0` asked for score only.
    pub hotspots: Vec<Hotspot>,
    /// Set only when the limit truncated the selection; carries the tail
    /// line's numbers.
    pub truncation: Option<Truncation>,
}

/// One selected hotspot.
///
/// Author: Claude Fable 5
#[derive(Debug, Clone, PartialEq)]
pub struct Hotspot {
    /// Index of the sentence in the slice given to [`analyse`].
    pub sentence_index: usize,
    /// The sentence's leave-one-out contribution, signed. Positive here by
    /// construction: only positive contributors are selected.
    pub contribution: f64,
}

/// What the limit cut off: the numbers for the `+N more, X.X grades
/// remaining` tail line.
///
/// Author: Claude Fable 5
#[derive(Debug, Clone, PartialEq)]
pub struct Truncation {
    /// How many more sentences the uncapped selection would have shown.
    pub hidden: usize,
    /// The gap left uncovered by the shown hotspots' contributions, in
    /// grades.
    pub remaining: f64,
}

/// A document under the floor: no score, but the complex words are still
/// named.
///
/// Author: Claude Fable 5
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    /// The prose word count that fell short. Zero-prose input lands here
    /// too, with a count of 0.
    pub words: usize,
    /// Every complex word in the document, deduplicated case-insensitively
    /// keeping the first spelling, in document order.
    pub complex: Vec<String>,
}

/// Scores a document and selects its hotspots.
///
/// Selection takes positive contributors in descending contribution until
/// their cumulative contribution covers the gap (score minus target),
/// capped by `limit`; the result is ordered back into document order.
/// Contributions do not sum exactly (each removal shifts the base for the
/// others), so gap coverage is approximate. A `limit` of 0 means score
/// only: no hotspots and no truncation tail, whatever the score.
///
/// ```
/// use gunfog::prose::extract;
/// use gunfog::score::{analyse, Analysis};
/// use gunfog::segment::sentences;
///
/// let md = "Too short to score, though the paradigm is beautiful.";
/// match analyse(&sentences(md, &extract(md)), 10.0, 10) {
///     Analysis::Refused(refusal) => {
///         assert_eq!(refusal.words, 9);
///         assert_eq!(refusal.complex, ["paradigm", "beautiful"]);
///     }
///     Analysis::Scored(_) => unreachable!(),
/// }
/// ```
///
/// Author: Claude Fable 5
pub fn analyse(sentences: &[Sentence], target: f64, limit: usize) -> Analysis {
    let totals = Totals::of(sentences);
    if totals.words < FLOOR {
        return Analysis::Refused(Refusal {
            words: totals.words,
            complex: named_complex(sentences),
        });
    }
    let over_target = printed(totals.fog) > target;
    if !over_target || limit == 0 {
        return Analysis::Scored(Scored {
            fog: totals.fog,
            over_target,
            hotspots: Vec::new(),
            truncation: None,
        });
    }

    let contributions = totals.contributions();
    let mut ranked: Vec<usize> = (0..sentences.len())
        .filter(|&i| contributions[i] > 0.0)
        .collect();
    // Stable sort: equal contributors keep document order.
    ranked.sort_by(|&a, &b| contributions[b].total_cmp(&contributions[a]));

    let gap = totals.fog - target;
    let mut covered = 0.0;
    let mut wanted = 0;
    for &i in &ranked {
        if covered >= gap {
            break;
        }
        covered += contributions[i];
        wanted += 1;
    }

    let shown = wanted.min(limit);
    let truncation = (wanted > limit).then(|| Truncation {
        hidden: wanted - limit,
        remaining: gap
            - ranked[..shown]
                .iter()
                .map(|&i| contributions[i])
                .sum::<f64>(),
    });
    let mut chosen = ranked[..shown].to_vec();
    chosen.sort_unstable();
    Analysis::Scored(Scored {
        fog: totals.fog,
        over_target,
        hotspots: chosen
            .into_iter()
            .map(|sentence_index| Hotspot {
                sentence_index,
                contribution: contributions[sentence_index],
            })
            .collect(),
        truncation,
    })
}

/// The document running totals the score and every contribution are
/// computed from.
///
/// Author: Claude Fable 5
struct Totals {
    /// Per-sentence (word count, complex-word count), in document order.
    counts: Vec<(usize, usize)>,
    /// Total prose words.
    words: usize,
    /// Total complex words.
    complex: usize,
    /// The document fog score, unrounded.
    fog: f64,
}

impl Totals {
    /// One pass over the sentences, counting as it goes.
    ///
    /// Author: Claude Fable 5
    fn of(sentences: &[Sentence]) -> Totals {
        let counts: Vec<(usize, usize)> = sentences
            .iter()
            .map(|sentence| (sentence.words.len(), complex_words(sentence).len()))
            .collect();
        let words = counts.iter().map(|&(words, _)| words).sum();
        let complex = counts.iter().map(|&(_, complex)| complex).sum();
        let fog = fog(words, counts.len(), complex);
        Totals {
            counts,
            words,
            complex,
            fog,
        }
    }

    /// Every sentence's leave-one-out contribution, signed, in document
    /// order, O(1) each from the totals. There is no sentence-length
    /// floor.
    ///
    /// Author: Claude Fable 5
    fn contributions(&self) -> Vec<f64> {
        self.counts
            .iter()
            .map(|&(words, complex)| {
                self.fog
                    - fog(
                        self.words - words,
                        self.counts.len() - 1,
                        self.complex - complex,
                    )
            })
            .collect()
    }
}

/// Gunning's formula. Zero sentences means zero prose: the fog of nothing
/// is 0, which makes a one-sentence document's contribution its whole
/// score.
///
/// Author: Claude Fable 5
fn fog(words: usize, sentences: usize, complex: usize) -> f64 {
    if sentences == 0 {
        return 0.0;
    }
    // A sentence is never empty, so words > 0 whenever sentences > 0.
    let words = words as f64;
    0.4 * (words / sentences as f64 + 100.0 * complex as f64 / words)
}

/// The score as the report prints it: rounded to one decimal.
///
/// Author: Claude Fable 5
pub(crate) fn printed(fog: f64) -> f64 {
    (fog * 10.0).round() / 10.0
}

/// The refusal's complex-word list: document order, deduplicated
/// case-insensitively, first spelling kept.
///
/// Author: Claude Fable 5
fn named_complex(sentences: &[Sentence]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut named = Vec::new();
    for sentence in sentences {
        for word in complex_words(sentence) {
            if seen.insert(word.text.to_lowercase()) {
                named.push(word.text.clone());
            }
        }
    }
    named
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prose::extract;
    use crate::segment::sentences;
    use std::iter::repeat_n;

    /// Builds a one-paragraph document from per-sentence (word count,
    /// complex count) specs: `Day` opens each sentence, `beautiful` is the
    /// complex word, `day` pads the rest.
    ///
    /// Author: Claude Fable 5
    fn doc(spec: &[(usize, usize)]) -> String {
        spec.iter()
            .map(|&(words, complex)| {
                assert!(words > complex);
                let mut tokens = vec!["Day"];
                tokens.extend(repeat_n("beautiful", complex));
                tokens.extend(repeat_n("day", words - complex - 1));
                format!("{}.", tokens.join(" "))
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Parses a spec'd document and checks the generator produced exactly
    /// the sentences asked for.
    ///
    /// Author: Claude Fable 5
    fn parsed(spec: &[(usize, usize)]) -> Vec<Sentence> {
        let md = doc(spec);
        let found = sentences(&md, &extract(&md));
        let shape: Vec<(usize, usize)> = found
            .iter()
            .map(|s| (s.words.len(), complex_words(s).len()))
            .collect();
        assert_eq!(shape, spec, "generator and pipeline disagree");
        found
    }

    /// The formula recomputed from scratch, independently of the running
    /// totals the module under test uses.
    ///
    /// Author: Claude Fable 5
    fn brute_fog(sentences: &[Sentence]) -> f64 {
        if sentences.is_empty() {
            return 0.0;
        }
        let words: usize = sentences.iter().map(|s| s.words.len()).sum();
        let complex: usize = sentences.iter().map(|s| complex_words(s).len()).sum();
        0.4 * (words as f64 / sentences.len() as f64 + 100.0 * complex as f64 / words as f64)
    }

    /// Leave-one-out by actually leaving each sentence out and rescoring.
    ///
    /// Author: Claude Fable 5
    fn brute_contributions(sentences: &[Sentence]) -> Vec<f64> {
        (0..sentences.len())
            .map(|i| {
                let mut rest = sentences.to_vec();
                rest.remove(i);
                brute_fog(sentences) - brute_fog(&rest)
            })
            .collect()
    }

    /// Selection re-derived from the brute-force contributions: the chosen
    /// indices in document order, and the tail's (hidden, remaining) when
    /// the limit binds.
    ///
    /// Author: Claude Fable 5
    fn brute_selection(
        found: &[Sentence],
        target: f64,
        limit: usize,
    ) -> (Vec<usize>, Option<(usize, f64)>) {
        let brute = brute_contributions(found);
        let mut ranked: Vec<usize> = (0..found.len()).filter(|&i| brute[i] > 0.0).collect();
        ranked.sort_by(|&a, &b| brute[b].total_cmp(&brute[a]));
        let gap = brute_fog(found) - target;
        let mut covered = 0.0;
        let mut wanted = 0;
        for &i in &ranked {
            if covered >= gap {
                break;
            }
            covered += brute[i];
            wanted += 1;
        }
        let shown = wanted.min(limit);
        let truncation = (wanted > limit).then(|| {
            let shown_sum: f64 = ranked[..shown].iter().map(|&i| brute[i]).sum();
            (wanted - limit, gap - shown_sum)
        });
        let mut chosen = ranked[..shown].to_vec();
        chosen.sort_unstable();
        (chosen, truncation)
    }

    /// Deterministic pseudo-random step (an LCG), so the sweep varies its
    /// documents without a clock or an RNG dependency.
    ///
    /// Author: Claude Fable 5
    fn lcg(state: &mut u64, bound: usize) -> usize {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*state >> 33) as usize) % bound
    }

    /// Author: Claude Fable 5
    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    /// Author: Claude Fable 5
    fn scored(analysis: Analysis) -> Scored {
        match analysis {
            Analysis::Scored(scored) => scored,
            Analysis::Refused(refusal) => panic!("refused: {refusal:?}"),
        }
    }

    /// Author: Claude Fable 5
    fn refused(analysis: Analysis) -> Refusal {
        match analysis {
            Analysis::Refused(refusal) => refusal,
            Analysis::Scored(scored) => panic!("scored: {scored:?}"),
        }
    }

    /// Author: Claude Fable 5
    #[test]
    fn contribution_matches_brute_force_recompute() {
        let found = parsed(&[(12, 2), (30, 0), (8, 1), (25, 3), (15, 0)]);
        let fast = Totals::of(&found).contributions();
        let brute = brute_contributions(&found);
        assert_eq!(fast.len(), brute.len());
        for (f, b) in fast.iter().zip(&brute) {
            assert_close(*f, *b);
        }
    }

    /// Author: Claude Fable 5
    #[test]
    fn single_sentence_contribution_is_the_whole_score() {
        let found = parsed(&[(12, 1)]);
        let fast = Totals::of(&found).contributions();
        assert_close(fast[0], brute_fog(&found));
    }

    /// Author: Claude Fable 5
    #[test]
    fn fog_matches_hand_computation() {
        // 10 sentences, 100 words, 5 complex:
        // 0.4 x (100/10 + 100 x 5/100) = 0.4 x 15 = 6.0.
        let spec: Vec<(usize, usize)> = (0..10).map(|i| (10, usize::from(i < 5))).collect();
        let result = scored(analyse(&parsed(&spec), 10.0, 10));
        assert_close(result.fog, 6.0);
        assert!(!result.over_target);
        assert!(result.hotspots.is_empty());
        assert_eq!(result.truncation, None);
    }

    /// Author: Claude Fable 5
    #[test]
    fn under_target_passes_with_no_hotspots() {
        let spec: Vec<(usize, usize)> = repeat_n((10, 1), 10).collect();
        let analysis = analyse(&parsed(&spec), 10.0, 10);
        assert!(analysis.passes());
        assert!(scored(analysis).hotspots.is_empty());
    }

    /// Author: Claude Fable 5
    #[test]
    fn over_target_is_judged_on_the_printed_one_decimal_score() {
        // Eight 25-word + one 26-word sentence: 226 words, 9 sentences,
        // raw fog 0.4 x 226/9 = 10.044, printed 10.0: the run passes.
        let mut spec = vec![(25, 0); 8];
        spec.push((26, 0));
        let analysis = analyse(&parsed(&spec), 10.0, 10);
        assert!(analysis.passes());
        let result = scored(analysis);
        assert!(result.fog > 10.0);
        assert!(!result.over_target);
        assert!(result.hotspots.is_empty());

        // Six 25-word + two 26-word: 202 words, 8 sentences, raw fog
        // 0.4 x 25.25 = 10.1: over.
        let mut spec = vec![(25, 0); 6];
        spec.extend([(26, 0); 2]);
        assert!(!analyse(&parsed(&spec), 10.0, 10).passes());
    }

    /// Author: Claude Fable 5
    #[test]
    fn selection_matches_brute_force_ranking_and_covers_the_gap() {
        let found = parsed(&[(40, 6), (10, 0), (35, 5), (10, 0), (15, 1)]);
        let target = 10.0;
        let result = scored(analyse(&found, target, 10));
        assert!(result.over_target);

        let (expected, truncation) = brute_selection(&found, target, 10);
        let chosen: Vec<usize> = result.hotspots.iter().map(|h| h.sentence_index).collect();
        assert_eq!(chosen, expected);
        assert_eq!(chosen, [0, 2], "document order");
        let brute = brute_contributions(&found);
        for hotspot in &result.hotspots {
            assert_close(hotspot.contribution, brute[hotspot.sentence_index]);
        }
        let cumulative: f64 = result.hotspots.iter().map(|h| h.contribution).sum();
        assert!(cumulative >= result.fog - target);
        assert_eq!(truncation, None);
        assert_eq!(result.truncation, None);
        assert!(!Analysis::Scored(result).passes());
    }

    /// Author: Claude Fable 5
    #[test]
    fn diffuse_fog_binds_on_the_limit_and_reports_the_tail() {
        // Twelve alternating sentences: the six (30, 2) ones each carry a
        // small positive contribution that never sums to the gap, so the
        // uncapped selection takes all six and a limit of 2 truncates.
        let spec: Vec<(usize, usize)> = (0..12)
            .map(|i| if i % 2 == 0 { (30, 2) } else { (20, 1) })
            .collect();
        let found = parsed(&spec);
        let target = 10.0;
        let result = scored(analyse(&found, target, 2));

        let chosen: Vec<usize> = result.hotspots.iter().map(|h| h.sentence_index).collect();
        assert_eq!(chosen, [0, 2], "equal contributors keep document order");
        let truncation = result.truncation.expect("limit must truncate");
        assert_eq!(truncation.hidden, 4);
        let gap = result.fog - target;
        let shown: f64 = result.hotspots.iter().map(|h| h.contribution).sum();
        assert_close(truncation.remaining, gap - shown);
        assert!(truncation.remaining > 0.0);
    }

    /// Author: Claude Fable 5
    #[test]
    fn sweep_matches_brute_force_across_documents_targets_and_limits() {
        let mut state: u64 = 0x5eed;
        for _ in 0..25 {
            let spec: Vec<(usize, usize)> = (0..3 + lcg(&mut state, 10))
                .map(|_| {
                    let words = 5 + lcg(&mut state, 36);
                    (words, lcg(&mut state, 4))
                })
                .collect();
            let found = parsed(&spec);
            let brute = brute_contributions(&found);
            for (fast, slow) in Totals::of(&found).contributions().iter().zip(&brute) {
                assert_close(*fast, *slow);
            }
            for target in [6.0, 10.0, 14.0] {
                for limit in [1, 2, 10] {
                    let Analysis::Scored(result) = analyse(&found, target, limit) else {
                        continue; // under the floor: covered elsewhere
                    };
                    if !result.over_target {
                        assert!(result.hotspots.is_empty());
                        continue;
                    }
                    let (expected, truncation) = brute_selection(&found, target, limit);
                    let chosen: Vec<usize> =
                        result.hotspots.iter().map(|h| h.sentence_index).collect();
                    assert_eq!(chosen, expected);
                    match (result.truncation, truncation) {
                        (None, None) => {}
                        (Some(got), Some((hidden, remaining))) => {
                            assert_eq!(got.hidden, hidden);
                            assert_close(got.remaining, remaining);
                        }
                        other => panic!("truncation mismatch: {other:?}"),
                    }
                }
            }
        }
    }

    /// Author: Claude Fable 5
    #[test]
    fn limit_zero_scores_only() {
        let found = parsed(&[(40, 6), (35, 5), (30, 4)]);
        let result = scored(analyse(&found, 10.0, 0));
        assert!(result.over_target);
        assert!(result.hotspots.is_empty());
        assert_eq!(result.truncation, None);
    }

    /// Author: Claude Fable 5
    #[test]
    fn floor_boundaries_at_99_100_101_words() {
        let mut spec = vec![(10, 0); 9];
        spec.push((9, 0));
        let refusal = refused(analyse(&parsed(&spec), 10.0, 10));
        assert_eq!(refusal.words, 99);

        let spec = vec![(10, 0); 10];
        assert!(matches!(
            analyse(&parsed(&spec), 10.0, 10),
            Analysis::Scored(_)
        ));

        let mut spec = vec![(10, 0); 10];
        spec.push((1, 0));
        assert!(matches!(
            analyse(&parsed(&spec), 10.0, 10),
            Analysis::Scored(_)
        ));
    }

    /// Author: Claude Fable 5
    #[test]
    fn refusal_fails_and_names_complex_words_deduped_in_document_order() {
        let md = "The paradigm was beautiful. A paradigm stays beautiful.";
        let analysis = analyse(&sentences(md, &extract(md)), 10.0, 10);
        assert!(!analysis.passes());
        let refusal = refused(analysis);
        assert_eq!(refusal.words, 8);
        assert_eq!(refusal.complex, ["paradigm", "beautiful"]);
    }

    /// Author: Claude Fable 5
    #[test]
    fn refusal_dedup_is_case_insensitive_keeping_first_spelling() {
        // Sentence-initial, so the proper-name exclusion cannot excuse it.
        let md = "Beautiful things stay beautiful.";
        let refusal = refused(analyse(&sentences(md, &extract(md)), 10.0, 10));
        assert_eq!(refusal.complex, ["Beautiful"]);
    }

    /// Author: Claude Fable 5
    #[test]
    fn zero_prose_refuses_down_the_same_path() {
        for md in ["", "```\nlet code = 1;\n```", "# Heading only"] {
            let analysis = analyse(&sentences(md, &extract(md)), 10.0, 10);
            assert!(!analysis.passes());
            let refusal = refused(analysis);
            assert_eq!(refusal.words, 0);
            assert!(refusal.complex.is_empty());
        }
    }

    /// Author: Claude Fable 5
    #[test]
    fn refusal_with_limit_zero_still_carries_the_word_count() {
        let md = "The paradigm was beautiful.";
        let refusal = refused(analyse(&sentences(md, &extract(md)), 10.0, 0));
        assert_eq!(refusal.words, 4);
        assert_eq!(refusal.complex, ["paradigm", "beautiful"]);
    }
}
