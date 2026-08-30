//! The CLI contract from `SPEC.md`: exactly one input per call, the two
//! flags, byte-exact output rendering, and the grep/diff exit codes. 0 is
//! a scored document at or under target; 1 is a document read and judged
//! (over target or under the floor); 2 never scored anything at all.

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_gunfog");

/// Runs the binary with `args`, feeding `stdin` and capturing both streams.
///
/// Author: Claude Opus 5
fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("gunfog binary should be runnable");
    let mut pipe = child.stdin.take().expect("stdin was piped");
    match pipe.write_all(stdin.as_bytes()) {
        Ok(()) => {}
        // A run that rejects its arguments exits before reading stdin. That
        // broken pipe is the verdict under test, not a failure to feed it.
        Err(err) if err.kind() == ErrorKind::BrokenPipe => {}
        Err(err) => panic!("stdin should accept the input: {err}"),
    }
    drop(pipe);
    child.wait_with_output().expect("gunfog should exit")
}

/// Author: Claude Opus 5
fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

/// Author: Claude Opus 5
fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

/// Writes `body` to a uniquely named file under the integration-test temp dir
/// cargo provides, and returns the path.
///
/// Author: Claude Opus 5
fn markdown_file(name: &str, body: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, body).expect("the cargo temp dir should be writable");
    path
}

/// One generator sentence: a capitalised opener (2 syllables, so counted
/// but never complex), `complex` copies of the 3-syllable `beautiful`,
/// then 1-syllable `day` filler up to `words` words.
///
/// Author: Claude Fable 5
fn sentence(opener: &str, complex: usize, words: usize) -> String {
    let mut tokens = vec![opener];
    tokens.extend(std::iter::repeat_n("beautiful", complex));
    tokens.extend(std::iter::repeat_n("day", words - complex - 1));
    format!("{}.", tokens.join(" "))
}

/// A document that scores fog 4.0: ten 10-word sentences, no complex
/// words, so 0.4 x (10 + 0).
///
/// Author: Claude Fable 5
fn passing_doc() -> String {
    std::iter::repeat_n("One day the fog rolled in from the cold sea.", 10)
        .collect::<Vec<_>>()
        .join(" ")
}

/// A document that scores fog 12.678: 115 words, 5 sentences, 10 complex,
/// 0.4 x (115/5 + 100 x 10/115). One paragraph per sentence, so `--file`
/// hotspots land on lines 3, 5, 7, 9, 11.
///
/// Contributions: the 40-word sentence carries +3.0449 (document minus it
/// scores 0.4 x (75/4 + 100 x 4/75) = 9.6333), the 30-word +0.8841, the
/// rest negative. Against the default target 10 the first alone covers the
/// 2.678 gap; against target 8 both are wanted and never cover the 4.678
/// gap, so `--limit 1` truncates with 4.678 - 3.0449 = 1.63 remaining.
///
/// Author: Claude Fable 5
fn foggy_doc() -> String {
    format!(
        "# Golden fixture\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n",
        sentence("Alpha", 6, 40),
        sentence("Bravo", 3, 30),
        sentence("Charlie", 1, 25),
        sentence("Delta", 0, 10),
        sentence("Echo", 0, 10),
    )
}

const ALPHA_LINE: &str = "+3.04 40w \"Alpha beautiful beautiful beautiful beautiful beautiful beautiful day…\" \
complex: beautiful";

/// Author: Claude Fable 5
#[test]
fn at_or_under_target_prints_the_score_line_alone_and_passes() {
    let output = run(&[&passing_doc()], "");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stdout_of(&output), "fog: 4.0 (target 10)\n");
    assert_eq!(stderr_of(&output), "");
}

/// Author: Claude Fable 5
#[test]
fn over_target_prints_hotspots_with_line_numbers_for_file_input() {
    let path = markdown_file("foggy.md", &foggy_doc());
    let output = run(&["--file", path.to_str().expect("path is UTF-8")], "");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let expected = format!(
        "fog: 12.7 (target 10)\n{}\n",
        ALPHA_LINE.replacen("40w ", "40w L3 ", 1)
    );
    assert_eq!(stdout_of(&output), expected);
}

/// Author: Claude Fable 5
#[test]
fn stdin_input_prints_no_line_numbers() {
    let output = run(&[], &foggy_doc());
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        stdout_of(&output),
        format!("fog: 12.7 (target 10)\n{ALPHA_LINE}\n")
    );
}

/// Author: Claude Fable 5
#[test]
fn a_binding_limit_appends_the_truncation_tail() {
    let output = run(&["--target", "8", "--limit", "1"], &foggy_doc());
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        stdout_of(&output),
        format!("fog: 12.7 (target 8)\n{ALPHA_LINE}\n+1 more, 1.6 grades remaining\n")
    );
}

/// Author: Claude Fable 5
#[test]
fn a_raised_target_turns_the_same_document_passing() {
    let output = run(&["--target", "13"], &foggy_doc());
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stdout_of(&output), "fog: 12.7 (target 13)\n");
}

/// Author: Claude Fable 5
#[test]
fn limit_zero_prints_the_score_line_alone_even_over_target() {
    let output = run(&["--limit", "0"], &foggy_doc());
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(stdout_of(&output), "fog: 12.7 (target 10)\n");
}

/// Author: Claude Fable 5
#[test]
fn short_input_refuses_naming_complex_words() {
    let output = run(
        &["The paradigm was beautiful. A paradigm stays beautiful."],
        "",
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        stdout_of(&output),
        "no score: 8 words (min 100)\ncomplex: paradigm, beautiful\n"
    );
}

/// Author: Claude Fable 5
#[test]
fn refusal_with_limit_zero_drops_the_complex_line() {
    let output = run(&["--limit", "0", "The paradigm was beautiful."], "");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(stdout_of(&output), "no score: 4 words (min 100)\n");
}

/// Author: Claude Fable 5
#[test]
fn zero_prose_refuses_with_a_zero_count() {
    let output = run(&["# Heading only"], "");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(stdout_of(&output), "no score: 0 words (min 100)\n");
}

/// Author: Claude Opus 5
/// Author: Claude Fable 5
#[test]
fn file_flag_is_the_input() {
    let path = markdown_file("input.md", &format!("# Heading\n\n{}\n", passing_doc()));
    let output = run(&["--file", path.to_str().expect("path is UTF-8")], "");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stdout_of(&output), "fog: 4.0 (target 10)\n");
}

/// Author: Claude Opus 5
#[test]
fn two_inputs_at_once_are_rejected() {
    let path = markdown_file("rejected.md", "the fog rolled in\n");
    let output = run(
        &["some text", "--file", path.to_str().expect("path is UTF-8")],
        "stdin this run will never read",
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr_of(&output).contains("cannot be used with"),
        "{output:?}"
    );
}

/// Author: Claude Opus 5
#[test]
fn a_missing_file_names_the_path() {
    let output = run(&["--file", "no/such/file.md"], "never read either");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr_of(&output).contains("no/such/file.md"), "{output:?}");
}

/// Exit 2 prints its reason to stderr and no stdout line at all.
///
/// Author: Claude Fable 5
#[test]
fn exit_two_prints_nothing_to_stdout() {
    let output = run(&["--file", "no/such/file.md"], "");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(stdout_of(&output), "");
}
