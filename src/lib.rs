#![forbid(unsafe_code)]

//! Scores prose with the Gunning fog index and names the sentences driving the
//! score.
//!
//! The pipeline described in `SPEC.md`: markdown prose extraction ([`prose`]),
//! sentence segmentation with word tokenisation ([`segment`]), syllable
//! counting with the complex-word rule ([`syllable`]), scoring with hotspot
//! attribution and the short-text floor ([`score`]), and report rendering
//! ([`report`]).

pub mod prose;
pub mod report;
pub mod score;
pub mod segment;
pub mod syllable;
