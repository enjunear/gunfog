#![forbid(unsafe_code)]

//! CLI wiring. All logic lives in the library.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use gunfog::prose::extract;
use gunfog::report::render;
use gunfog::score::analyse;
use gunfog::segment::sentences;

/// Score prose with the Gunning fog index.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Markdown file to score. Omit to read from stdin.
    #[arg(long, value_name = "PATH")]
    file: Option<PathBuf>,

    /// Fog score threshold. Sets hotspot flagging and the exit code.
    #[arg(long, value_name = "N", default_value_t = 10)]
    target: usize,

    /// Maximum hotspot lines. 0 prints the score line alone.
    #[arg(long, value_name = "N", default_value_t = 10)]
    limit: usize,
}

/// Author: Claude Opus 5
/// Author: Claude Fable 5
fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        // 1, per the grep/diff convention: a document was read and judged
        // over target or under the floor.
        Ok(passes) => {
            if passes {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(err) => {
            eprintln!("gunfog: {err}");
            // 2, not 1. A 1 means gunfog read a document and judged it; this
            // path never got that far. clap already exits 2 on a bad
            // invocation, so both no-score failures share one code.
            ExitCode::from(2)
        }
    }
}

/// Runs the pipeline over the chosen input, prints the report, and returns
/// the exit-code judgement.
///
/// Author: Claude Opus 5
/// Author: Claude Fable 5
fn run(cli: &Cli) -> std::io::Result<bool> {
    let input = read_input(cli.file.as_deref())?;
    let found = sentences(&input, &extract(&input));
    let analysis = analyse(&found, cli.target as f64, cli.limit);
    // Line numbers are source lines, which only --file input can honour.
    print!(
        "{}",
        render(
            &input,
            &found,
            &analysis,
            cli.target,
            cli.limit,
            cli.file.is_some()
        )
    );
    Ok(analysis.passes())
}

/// Resolves the one input for this call: `--file`, or stdin without it.
///
/// Author: Claude Opus 5
/// Author: Claude Fable 5
fn read_input(file: Option<&Path>) -> std::io::Result<String> {
    match file {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|err| std::io::Error::new(err.kind(), format!("{}: {err}", path.display()))),
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}
