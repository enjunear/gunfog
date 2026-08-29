#![forbid(unsafe_code)]

//! Scores prose with the Gunning fog index and names the sentences driving the
//! score.
//!
//! The pipeline described in `SPEC.md` (markdown prose extraction, sentence
//! segmentation, word tokenisation, syllable counting, scoring, hotspot
//! attribution, report rendering) is not built yet. [`word_count`] is the stub
//! the CLI skeleton runs against.

/// Counts whitespace-separated tokens.
///
/// A stand-in for the real word tokeniser. It does not strip markdown and it
/// does not apply the tokenisation rules in `SPEC.md`, so a hyphenated
/// compound, a contraction and a bare word all count as one either way, but an
/// inline code span counts as however many chunks it happens to contain.
///
/// ```
/// assert_eq!(gunfog::word_count("the fog rolled in"), 4);
/// assert_eq!(gunfog::word_count(""), 0);
/// ```
///
/// Author: Claude Opus 5
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}
