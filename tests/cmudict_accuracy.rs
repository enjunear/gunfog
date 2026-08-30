//! Dev-time accuracy check: the 29-rule counter against CMUdict on the 3+
//! boundary, the figure `docs/research/syllable-counting.md` §4 measured at
//! 96.09% on its held-out half.
//!
//! CMUdict is deliberately not shipped, so the test is ignored by default.
//! To run it, point `CMUDICT` at a copy of `cmudict.dict` (the cmusphinx
//! format: `headword PH0 N EH1 M Z`, variants suffixed `(2)`):
//!
//! ```text
//! curl -LO https://raw.githubusercontent.com/cmusphinx/cmudict/master/cmudict.dict
//! CMUDICT=cmudict.dict cargo test --test cmudict_accuracy -- --ignored
//! ```
//!
//! The gate is 96.0% over the whole dictionary, a touch under the 96.15%
//! this port measured on cmusphinx master (2026-08-30) and the research
//! note's 96.09% held-out figure: the note's number came from one half of
//! a private 50/50 split of a vendored snapshot, so it cannot be matched
//! exactly. Under the gate means the port has diverged from the note's
//! rule semantics.

use gunfog::syllable::syllables;

/// Author: Claude Fable 5
#[test]
#[ignore = "needs a local CMUdict; see the module comment"]
fn three_plus_boundary_accuracy_holds() {
    let path = std::env::var("CMUDICT").expect("set CMUDICT to a cmudict.dict path");
    let data = std::fs::read_to_string(&path).expect("read CMUdict");

    let mut total = 0u64;
    let mut agree = 0u64;
    for line in data.lines() {
        let mut fields = line.split_whitespace();
        let Some(headword) = fields.next() else {
            continue;
        };
        // Alphabetic headwords only, which also drops the `(2)` variant
        // entries so only the first pronunciation counts, as the research
        // note measured.
        if headword.is_empty() || !headword.bytes().all(|b| b.is_ascii_lowercase()) {
            continue;
        }
        // Syllables = phones carrying a stress digit. A `#` starts a
        // trailing comment in the cmusphinx format.
        let mut gold = 0usize;
        for phone in fields {
            if phone.starts_with('#') {
                break;
            }
            if phone.bytes().any(|b| b.is_ascii_digit()) {
                gold += 1;
            }
        }
        if gold == 0 {
            continue;
        }
        total += 1;
        if (syllables(headword) >= 3) == (gold >= 3) {
            agree += 1;
        }
    }

    assert!(
        total > 100_000,
        "CMUdict looks truncated: {total} usable headwords"
    );
    let accuracy = agree as f64 / total as f64;
    println!(
        "3+ boundary accuracy: {:.2}% over {total} headwords",
        accuracy * 100.0
    );
    assert!(
        accuracy >= 0.960,
        "3+ boundary accuracy {:.2}% is materially short of the research note's 96.09%",
        accuracy * 100.0
    );
}
