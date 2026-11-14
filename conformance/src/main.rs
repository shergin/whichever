//! The command line of the conformance matrix.
//!
//! ```text
//! cargo run -p whichever-conformance --release -- <command>
//!
//! run [--suite NAME]... [--filter TEXT] [--bless] [--whichever PATH] [--jobs N]
//!     [--shape] [--random N [--seed S]]
//! list [--suite NAME]...
//! show <id>
//! axes
//! ```
//!
//! `run` exits 0 when nothing diverged and the ledger agrees, 1 on a
//! divergence or a ledger mismatch, and 2 on a usage or harness error.

use std::path::PathBuf;
use std::process::ExitCode;

use whichever_conformance::cell::Cell;
use whichever_conformance::ledger::Files;
use whichever_conformance::render;
use whichever_conformance::run::{self, Options, RunError, Status};

const USAGE: &str = "\
usage: cargo run -p whichever-conformance --release -- <command>

commands:
  run [--suite NAME]... [--filter TEXT] [--bless] [--whichever PATH] [--jobs N]
      [--shape] [--random N [--seed S]]
  list [--suite NAME]...
  show <id>
  axes";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match command(&arguments) {
        Ok(code) => code,
        Err(RunError::Usage(message)) => {
            eprintln!("error: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
        Err(RunError::Harness(message)) => {
            eprintln!("harness error: {message}");
            ExitCode::from(2)
        }
    }
}

fn command(arguments: &[String]) -> Result<ExitCode, RunError> {
    let Some((name, rest)) = arguments.split_first() else {
        return Err(RunError::Usage(String::from("no command")));
    };
    match name.as_str() {
        "run" => {
            let options = options(rest)?;
            let status = run::run(&options)?;
            Ok(match status {
                Status::Clean => ExitCode::SUCCESS,
                Status::Diverged => ExitCode::from(1),
                Status::ModelBroken => ExitCode::from(2),
            })
        }
        "list" => {
            let options = options(rest)?;
            list(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        "show" => {
            let [id] = rest else {
                return Err(RunError::Usage(String::from("`show` takes one id")));
            };
            let cell = Cell::parse(id).map_err(|error| RunError::Usage(error.to_string()))?;
            print!("{}", render::render(&cell).source);
            Ok(ExitCode::SUCCESS)
        }
        "axes" => {
            if !rest.is_empty() {
                return Err(RunError::Usage(String::from("`axes` takes no argument")));
            }
            for axis in Cell::AXES {
                println!("{}: {}", axis.name, axis.summary.trim());
                for (index, word) in (axis.words)().iter().enumerate() {
                    let neutral = if index == 0 { "  (neutral)" } else { "" };
                    println!("  {word}{neutral}");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        other => Err(RunError::Usage(format!("no command `{other}`"))),
    }
}

fn options(arguments: &[String]) -> Result<Options, RunError> {
    let mut options = Options::default();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        let mut value = |flag: &str| {
            rest.next()
                .cloned()
                .ok_or_else(|| RunError::Usage(format!("`{flag}` takes a value")))
        };
        match argument.as_str() {
            "--suite" => options.suites.push(value("--suite")?),
            "--filter" => options.filter = Some(value("--filter")?),
            "--bless" => options.bless = true,
            "--shape" => options.shape = true,
            "--random" => {
                let count = value("--random")?;
                let count = count
                    .parse::<usize>()
                    .ok()
                    .filter(|count| *count > 0)
                    .ok_or_else(|| {
                        RunError::Usage(format!("`--random {count}` is no positive number"))
                    })?;
                options.random = Some(count);
            }
            "--seed" => {
                let seed = value("--seed")?;
                options.seed = seed
                    .parse::<u64>()
                    .map_err(|_| RunError::Usage(format!("`--seed {seed}` is no number")))?;
            }
            "--whichever" => options.whichever = Some(PathBuf::from(value("--whichever")?)),
            "--jobs" => {
                let jobs = value("--jobs")?;
                let jobs = jobs
                    .parse::<usize>()
                    .ok()
                    .filter(|jobs| *jobs > 0)
                    .ok_or_else(|| {
                        RunError::Usage(format!("`--jobs {jobs}` is no positive number"))
                    })?;
                options.jobs = Some(jobs);
            }
            other => return Err(RunError::Usage(format!("unknown option `{other}`"))),
        }
    }
    Ok(options)
}

fn list(options: &Options) -> Result<(), RunError> {
    let files = Files::at(&run::repository().join("conformance"));
    let (selections, _) = run::suites(&files)?;
    let names: Vec<&str> = selections
        .iter()
        .map(|selection| selection.suite.as_str())
        .collect();
    for suite in &options.suites {
        if !names.contains(&suite.as_str()) {
            return Err(RunError::Usage(format!(
                "no suite `{suite}`; the suites are {}",
                names.join(", ")
            )));
        }
    }
    for selection in &selections {
        if !options.suites.is_empty() && !options.suites.contains(&selection.suite) {
            continue;
        }
        println!(
            "suite {} ({} cells)",
            selection.suite,
            selection.cells.len()
        );
        for cell in &selection.cells {
            println!("  {}\t{}", cell.id(), cell.verdict().describe());
        }
    }
    Ok(())
}
