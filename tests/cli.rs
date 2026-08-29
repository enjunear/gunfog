//! The CLI contract from `SPEC.md`. Exactly one input per call, plus the two
//! flags.
//!
//! Two things below certify the stub rather than the contract. The word counts
//! come from the stub scorer and exist only to prove each input route reaches
//! it. And every success assertion rests on stubbed exit codes: `SPEC.md` gives
//! exit 1 to anything under the 100-word floor, and these inputs run 4 to 9
//! words, so all of them exit 1 once the floor lands. Work item 15 authorises
//! the stub and work item 20 rewrites both. The exit code 2 asserted further
//! down is not stubbed: `SPEC.md` fixes 2 as the code for a run that never
//! scored anything, and both cases asserting it are that.

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

/// Author: Claude Opus 5
#[test]
fn inline_argument_is_the_input() {
    let output = run(&["the fog rolled in"], "");
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("4 words"), "{output:?}");
}

/// Author: Claude Opus 5
#[test]
fn file_flag_is_the_input() {
    let path = markdown_file("input.md", "# Heading\n\nthe fog rolled in from the sea\n");
    let output = run(&["--file", path.to_str().expect("path is UTF-8")], "");
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("9 words"), "{output:?}");
}

/// Author: Claude Opus 5
#[test]
fn stdin_is_the_input_when_neither_is_given() {
    let output = run(&[], "the fog rolled in from the sea");
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("7 words"), "{output:?}");
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

/// Author: Claude Opus 5
#[test]
fn target_and_limit_default_to_ten() {
    let output = run(&["the fog rolled in"], "");
    assert!(stdout_of(&output).contains("(target 10)"), "{output:?}");
    assert!(stdout_of(&output).contains("limit 10"), "{output:?}");
}

/// Author: Claude Opus 5
#[test]
fn target_and_limit_are_overridable() {
    let output = run(&["--target", "14", "--limit", "0", "the fog rolled in"], "");
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("(target 14)"), "{output:?}");
    assert!(stdout_of(&output).contains("limit 0"), "{output:?}");
}
