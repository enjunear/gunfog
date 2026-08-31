//! Syllable counting and the complex-word rule: words in, the complex
//! words the fog formula counts out.
//!
//! The counter is the 29-rule byte-comparison set from
//! `docs/research/syllable-counting.md` §4: a vowel-letter-group base
//! estimate with silent-`e` and consonant+`le` corrections, adjusted by 29
//! tuned rules, each applied at most once per word. No regex engine, no
//! word list. Measured there at 96.1% held-out accuracy on the 3+ boundary;
//! `tests/cmudict_accuracy.rs` re-checks that figure against a local
//! CMUdict.
//!
//! A complex word has 3+ syllables after Gunning's exclusions (proper
//! names, `-ed`/`-es` inflations, hyphenated words judged per part). The
//! exclusions are applied here, inside [`complex_words`], so an excused
//! word can never leak into a `complex:` list downstream.

use crate::segment::{Sentence, Word};

/// Rules matched with `str::contains`, and the count adjustment each makes.
const CONTAINS_RULES: [(&str, i32); 10] = [
    ("ia", 1),
    ("io", 1),
    ("eo", 1),
    ("iu", 1),
    ("creat", 1),
    ("dien", 1),
    ("cious", -1),
    ("tia", -1),
    ("giu", -1),
    ("cial", -1),
];

/// Rules matched with `str::starts_with`.
const PREFIX_RULES: [(&str, i32); 2] = [("mc", 1), ("jua", -1)];

/// Rules matched with `str::ends_with`.
const SUFFIX_RULES: [(&str, i32); 10] = [
    ("ier", 1),
    ("ee", 1),
    ("ie", 1),
    ("ism", 1),
    ("asm", 1),
    ("lle", -1),
    ("tes", -1),
    ("nes", -1),
    ("ion", -1),
    ("ions", -1),
];

/// The letter class vowel runs are built from: the five vowels plus `y`.
///
/// Author: Claude Fable 5
fn is_vowel_or_y(byte: u8) -> bool {
    matches!(byte, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
}

/// The strict `[aeiou]` class several tuned rules test, `y` excluded.
///
/// Author: Claude Fable 5
fn is_vowel(byte: u8) -> bool {
    matches!(byte, b'a' | b'e' | b'i' | b'o' | b'u')
}

/// Estimates the syllable count of one word.
///
/// Anything that is not an ASCII letter (apostrophes, digits, accented
/// letters) is dropped before counting; a token with no ASCII letters
/// counts zero. Any word with letters counts at least one.
///
/// ```
/// use gunfog::syllable::syllables;
///
/// assert_eq!(syllables("fog"), 1);
/// assert_eq!(syllables("table"), 2);
/// assert_eq!(syllables("beautiful"), 3);
/// assert_eq!(syllables("42"), 0);
/// ```
///
/// Author: Claude Fable 5
pub fn syllables(word: &str) -> usize {
    let normalised: String = word
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|letter| letter.to_ascii_lowercase())
        .collect();
    if normalised.is_empty() {
        return 0;
    }
    let bytes = normalised.as_bytes();

    // Base estimate: maximal runs of vowel letters.
    let mut count: i32 = 0;
    let mut in_run = false;
    for &byte in bytes {
        let vowel = is_vowel_or_y(byte);
        if vowel && !in_run {
            count += 1;
        }
        in_run = vowel;
    }

    // Silent final `e`, except when a consonant+`le` ending keeps it.
    if bytes.ends_with(b"e")
        && !(bytes.len() >= 3 && bytes.ends_with(b"le") && !is_vowel_or_y(bytes[bytes.len() - 3]))
    {
        count -= 1;
    }

    count += adjustments(&normalised);
    count.max(1) as usize
}

/// The 29 tuned rules. Each fires at most once however often its pattern
/// occurs.
///
/// Author: Claude Fable 5
fn adjustments(word: &str) -> i32 {
    let bytes = word.as_bytes();
    let len = bytes.len();
    let mut delta = 0;

    for (pattern, adjustment) in CONTAINS_RULES {
        if word.contains(pattern) {
            delta += adjustment;
        }
    }
    for (pattern, adjustment) in PREFIX_RULES {
        if word.starts_with(pattern) {
            delta += adjustment;
        }
    }
    for (pattern, adjustment) in SUFFIX_RULES {
        if word.ends_with(pattern) {
            delta += adjustment;
        }
    }

    // The seven character-class windows.
    // -1 [^td]ed$
    if len >= 3 && bytes.ends_with(b"ed") && !matches!(bytes[len - 3], b't' | b'd') {
        delta -= 1;
    }
    // +1 [aeiou]y[aeiou]
    if bytes
        .windows(3)
        .any(|w| is_vowel(w[0]) && w[1] == b'y' && is_vowel(w[2]))
    {
        delta += 1;
    }
    // -1 .ely$
    if len >= 4 && bytes.ends_with(b"ely") {
        delta -= 1;
    }
    // +1 [^gq]ua[^auieo]
    if bytes
        .windows(4)
        .any(|w| !matches!(w[0], b'g' | b'q') && w[1] == b'u' && w[2] == b'a' && !is_vowel(w[3]))
    {
        delta += 1;
    }
    // +1 [^aeiou]y[ae]
    if bytes
        .windows(3)
        .any(|w| !is_vowel(w[0]) && w[1] == b'y' && matches!(w[2], b'a' | b'e'))
    {
        delta += 1;
    }
    // +1 [^l]lien
    if bytes.windows(5).any(|w| w[0] != b'l' && &w[1..] == b"lien") {
        delta += 1;
    }
    // -1 awe($|d|so)
    if bytes.windows(3).enumerate().any(|(i, w)| {
        w == b"awe"
            && match bytes.get(i + 3) {
                None => true,
                Some(b'd') => true,
                Some(b's') => bytes.get(i + 4) == Some(&b'o'),
                Some(_) => false,
            }
    }) {
        delta -= 1;
    }

    delta
}

/// The words of a sentence that count as complex, in sentence order.
///
/// Gunning's exclusions are applied here, so a word this function omits
/// never reaches a `complex:` list: proper names (capitalised, not the
/// sentence's first word, and not led by a placeholder's capitalised
/// stand-in), words 3-syllable only by an `-ed`/`-es` ending
/// (`-ing` deliberately not excused), hyphenated words unless a
/// hyphen-separated part is 3+ syllables on its own, and words tokenisation
/// marked never-complex (numbers, placeholder words).
///
/// ```
/// use gunfog::prose::extract;
/// use gunfog::segment::sentences;
/// use gunfog::syllable::complex_words;
///
/// let md = "Claude underestimated the beautiful, well-tuned heuristics.";
/// let sentence = &sentences(md, &extract(md))[0];
/// let complex: Vec<&str> = complex_words(sentence)
///     .iter()
///     .map(|word| word.text.as_str())
///     .collect();
/// // "Claude" opens the sentence, so the proper-name exclusion cannot
/// // excuse it, but its two syllables keep it simple anyway.
/// assert_eq!(complex, ["underestimated", "beautiful", "heuristics"]);
/// ```
///
/// Author: Claude Fable 5
pub fn complex_words(sentence: &Sentence) -> Vec<&Word> {
    sentence
        .words
        .iter()
        .enumerate()
        .filter(|(index, word)| is_complex(word, *index == 0))
        .map(|(_, word)| word)
        .collect()
}

/// Applies the complex-word rule to one word.
///
/// Author: Claude Fable 5
fn is_complex(word: &Word, sentence_initial: bool) -> bool {
    if word.never_complex {
        return false;
    }
    // A capitalised word not at sentence start is a proper name. A name
    // opening a sentence slips through and gets counted; accepted cost.
    // A placeholder's capital is segmentation, not evidence of a name.
    if !sentence_initial
        && !word.placeholder_led
        && word.text.chars().next().is_some_and(char::is_uppercase)
    {
        return false;
    }
    // A hyphenated word is judged per part on syllables alone. The
    // `-ed`/`-es` excusal below is for inflection lifting a whole word
    // over the line; `docs/research/hyphenated-words.md` §6 keeps
    // `agent-oriented` complex on the strength of `oriented`.
    if word.text.contains('-') {
        return word.text.split('-').any(|part| syllables(part) >= 3);
    }
    if syllables(&word.text) < 3 {
        return false;
    }
    let lower = word.text.to_ascii_lowercase();
    if (lower.ends_with("ed") || lower.ends_with("es"))
        // The matched suffix is two ASCII bytes, so the slice is in bounds
        // and on a character boundary whatever precedes it.
        && syllables(&word.text[..word.text.len() - 2]) < 3
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prose::extract;
    use crate::segment::sentences;

    /// The complex words the whole pipeline finds in one markdown string.
    ///
    /// Author: Claude Fable 5
    fn complex_in(md: &str) -> Vec<String> {
        sentences(md, &extract(md))
            .iter()
            .flat_map(|sentence| {
                complex_words(sentence)
                    .into_iter()
                    .map(|word| word.text.clone())
            })
            .collect()
    }

    /// Author: Claude Fable 5
    #[test]
    fn counts_words_either_side_of_the_boundary() {
        // Under three.
        assert_eq!(syllables("fog"), 1);
        assert_eq!(syllables("the"), 1);
        assert_eq!(syllables("make"), 1);
        assert_eq!(syllables("see"), 1);
        assert_eq!(syllables("table"), 2);
        assert_eq!(syllables("simple"), 2);
        assert_eq!(syllables("belle"), 1);
        assert_eq!(syllables("nation"), 2);
        assert_eq!(syllables("movie"), 2);
        assert_eq!(syllables("flawed"), 1);
        // Three or more.
        assert_eq!(syllables("beautiful"), 3);
        assert_eq!(syllables("important"), 3);
        assert_eq!(syllables("employee"), 3);
        assert_eq!(syllables("creation"), 3);
        assert_eq!(syllables("easier"), 3);
        assert_eq!(syllables("saying"), 2);
        assert_eq!(syllables("complicated"), 4);
    }

    /// Author: Claude Fable 5
    #[test]
    fn strips_non_letters_before_counting() {
        assert_eq!(syllables("doesn't"), syllables("doesnt"));
        assert_eq!(syllables("Fog"), 1);
        assert_eq!(syllables("42"), 0);
        assert_eq!(syllables(""), 0);
        // A recorded limit, not a goal: accented vowels are dropped, not
        // treated as vowels, so the word undercounts.
        assert_eq!(syllables("résumé"), 1);
    }

    /// Author: Claude Fable 5
    #[test]
    fn three_syllables_is_complex() {
        assert_eq!(
            complex_in("the beautiful fog rolled in over the simple harbour"),
            ["beautiful"]
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn proper_name_mid_sentence_is_excused() {
        assert_eq!(
            complex_in("we visited Antarctica in winter"),
            Vec::<String>::new()
        );
        // The same letters uncapitalised stay complex.
        assert_eq!(
            complex_in("we visited antarctica in winter"),
            ["antarctica"]
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn proper_name_opening_a_sentence_is_counted() {
        // The accepted cost named in SPEC.md: sentence-initial capitals
        // cannot be told from ordinary sentence case.
        assert_eq!(complex_in("Antarctica is cold"), ["Antarctica"]);
    }

    /// Author: Claude Fable 5
    #[test]
    fn ed_and_es_inflations_are_excused() {
        // 3 syllables only because of the ending.
        assert_eq!(
            complex_in("the fog created a problem"),
            Vec::<String>::new()
        );
        assert_eq!(complex_in("he trespasses at night"), Vec::<String>::new());
        // 3+ syllables even without the ending stays complex.
        assert_eq!(complex_in("a complicated day"), ["complicated"]);
        assert_eq!(complex_in("all the properties sold"), ["properties"]);
    }

    /// Author: Claude Fable 5
    #[test]
    fn ing_is_not_excused() {
        assert_eq!(complex_in("the fog was thickening slowly"), ["thickening"]);
    }

    /// Author: Claude Fable 5
    #[test]
    fn hyphenated_word_judged_per_part() {
        // No part reaches 3 syllables alone.
        assert_eq!(
            complex_in("a well-known short-term fix"),
            Vec::<String>::new()
        );
        // One part does.
        assert_eq!(complex_in("a self-important tone"), ["self-important"]);
        // 3+ syllables as a whole is not enough: no part of
        // over-the-counter reaches 3 alone, so it stays simple.
        assert_eq!(complex_in("an over-the-counter cure"), Vec::<String>::new());
        // The part test is syllables alone, with no -ed/-es excusal:
        // the research note's own example stays complex on the strength
        // of its second part, and so does a 3-syllable -ed part.
        assert_eq!(complex_in("an agent-oriented design"), ["agent-oriented"]);
        assert_eq!(complex_in("the re-created scene"), ["re-created"]);
    }

    /// The stand-in's leading capital is a segmentation device and must
    /// not reach the proper-name exclusion: a compound led by a
    /// placeholder is still judged on its real parts.
    ///
    /// Author: Claude Opus 5
    #[test]
    fn a_placeholder_led_compound_is_not_a_proper_name() {
        assert_eq!(
            complex_in("a `foo`-oriented interpretation"),
            ["X-oriented", "interpretation"]
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn never_complex_words_stay_excluded() {
        // The inline code span becomes a one-syllable placeholder and the
        // number can never be complex, whatever the counter would say.
        assert_eq!(
            complex_in("run `impossibly-multisyllabic-identifier` on 12345678 files"),
            Vec::<String>::new()
        );
    }
}
