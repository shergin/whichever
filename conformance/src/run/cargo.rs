//! Nested cargo: the command with the outer cargo's environment removed,
//! the diagnostics of `cargo check` read from its JSON messages, and the
//! results of `cargo test` read from its text.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// Returns a cargo command run in `workspace` with its target directory
/// at `target`, using the toolchain the outer cargo ran with and none of
/// its environment.
pub fn command(workspace: &Path, target: &Path) -> Command {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let mut command = Command::new(cargo);
    command.current_dir(workspace);
    for (key, _) in std::env::vars_os() {
        let Some(key) = key.to_str() else {
            continue;
        };
        if leaks(key) {
            command.env_remove(key);
        }
    }
    command.env("CARGO_TARGET_DIR", target);
    command
}

/// Returns whether a variable of the outer cargo's environment would change
/// what the nested cargo builds.
fn leaks(key: &str) -> bool {
    const FLAGS: &[&str] = &[
        "RUSTFLAGS",
        "RUSTDOCFLAGS",
        "RUSTC",
        "RUSTDOC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "OUT_DIR",
        "MAKEFLAGS",
        "MFLAGS",
    ];
    if FLAGS.contains(&key) {
        return true;
    }
    if key.starts_with("__CARGO") {
        return true;
    }
    key.starts_with("CARGO_") && key != "CARGO_HOME"
}

/// The level of a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// An error.
    Error,
    /// A warning.
    Warning,
}

/// A span in a cell's file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    /// The cell's module, `c_` and twelve hex digits.
    pub module: String,
    /// The line, counted from 1.
    pub line: usize,
}

/// A diagnostic of rustc, attributed to a cell.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    /// Its level.
    pub level: Level,
    /// The error code or the lint's name.
    pub code: Option<String>,
    /// The message's first line.
    pub message: String,
    /// The rendered text, for the report.
    pub rendered: String,
    /// The first span in a cell's file, from the primary span outward
    /// through the macro backtrace.
    pub site: Option<Site>,
    /// Every span in a cell's file along the backtrace, the site first.
    pub chain: Vec<Site>,
    /// Whether a `compile_error!` in the backtrace raised it.
    pub from_compile_error: bool,
}

/// Returns the module a file of a cell is, or `None` for any other file.
fn cell_module(file: &str) -> Option<String> {
    let name = Path::new(file).file_name()?.to_str()?;
    let module = name.strip_suffix(".rs")?;
    let digits = module.strip_prefix("c_")?;
    if digits.len() != 12 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(String::from(module))
}

/// Returns whether a message is cargo's or rustc's summary, which no cell
/// owns.
fn is_summary(message: &str) -> bool {
    message.starts_with("aborting due to")
        || message.ends_with("warnings emitted")
        || message.ends_with("warning emitted")
        || message.starts_with("For more information about")
        || message.starts_with("Some errors have detailed explanations")
}

/// Parses the compiler messages of cargo's JSON output. A message whose
/// spans are none of a cell's is returned with no site; a summary is
/// dropped.
pub fn diagnostics(output: &str) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for line in output.lines() {
        if !line.starts_with('{') {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value["reason"] != "compiler-message" {
            continue;
        }
        let message = &value["message"];
        let level = match message["level"].as_str() {
            Some("error") | Some("error: internal compiler error") => Level::Error,
            Some("warning") => Level::Warning,
            _ => continue,
        };
        let text = message["message"].as_str().unwrap_or_default();
        let spans = message["spans"].as_array().cloned().unwrap_or_default();
        if spans.is_empty() && is_summary(text) {
            continue;
        }
        let code = message["code"]["code"].as_str().map(String::from);
        let primary = spans
            .iter()
            .find(|span| span["is_primary"] == true)
            .or_else(|| spans.first());
        let mut chain = Vec::new();
        let mut from_compile_error = false;
        let mut span = primary;
        while let Some(current) = span {
            if let Some(module) = current["file_name"].as_str().and_then(cell_module) {
                let line = current["line_start"].as_u64().unwrap_or(0) as usize;
                chain.push(Site { module, line });
            }
            let expansion = &current["expansion"];
            if expansion["macro_decl_name"] == "compile_error!" {
                from_compile_error = true;
            }
            span = expansion.get("span");
        }
        found.push(Diagnostic {
            level,
            code,
            message: String::from(text.lines().next().unwrap_or_default()),
            rendered: String::from(message["rendered"].as_str().unwrap_or(text)),
            site: chain.first().cloned(),
            chain,
            from_compile_error,
        });
    }
    found
}

/// The result of one test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestResult {
    /// It passed.
    Passed,
    /// It failed, with the first line of the panic's message.
    Failed(String),
}

/// Parses the text of `cargo test`: the result of every test, by its path
/// in its crate.
pub fn test_results(output: &str) -> BTreeMap<String, TestResult> {
    let mut results = BTreeMap::new();
    let lines: Vec<&str> = output.lines().collect();
    for line in &lines {
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        let Some((name, result)) = rest.rsplit_once(" ... ") else {
            continue;
        };
        let result = match result.trim() {
            "ok" => TestResult::Passed,
            "FAILED" => TestResult::Failed(String::new()),
            _ => continue,
        };
        results.insert(String::from(name.trim()), result);
    }
    for (index, line) in lines.iter().enumerate() {
        let Some(name) = line
            .strip_prefix("---- ")
            .and_then(|rest| rest.strip_suffix(" stdout ----"))
        else {
            continue;
        };
        let message = lines[index + 1..]
            .iter()
            .take_while(|line| !line.starts_with("---- ") && **line != "failures:")
            .skip_while(|line| !line.contains("panicked at"))
            .nth(1)
            .map(|line| line.trim().to_string())
            .unwrap_or_else(|| String::from("no panic message"));
        results.insert(String::from(name), TestResult::Failed(message));
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diagnostic_is_attributed_through_the_backtrace() {
        let message = r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types","code":{"code":"E0308"},"rendered":"error[E0308]","spans":[{"file_name":"/x/whichever/macros/src/lib.rs","line_start":3,"is_primary":true,"expansion":{"macro_decl_name":"Probe!","span":{"file_name":"pass-000/src/c_0123456789ab.rs","line_start":40,"is_primary":false,"expansion":null}}}]}}"#;
        let found = diagnostics(message);
        assert_eq!(found.len(), 1);
        let diagnostic = &found[0];
        assert_eq!(diagnostic.level, Level::Error);
        assert_eq!(diagnostic.code.as_deref(), Some("E0308"));
        assert_eq!(
            diagnostic.site,
            Some(Site {
                module: String::from("c_0123456789ab"),
                line: 40
            })
        );
        assert!(!diagnostic.from_compile_error);
    }

    #[test]
    fn summaries_are_dropped_and_other_lines_ignored() {
        let output = "Compiling x\n{\"reason\":\"compiler-message\",\"message\":{\"level\":\"error\",\"message\":\"aborting due to 2 previous errors\",\"code\":null,\"spans\":[]}}\n{\"reason\":\"build-finished\",\"success\":false}\n";
        assert!(diagnostics(output).is_empty());
    }

    #[test]
    fn test_results_are_read_with_the_panic_message() {
        let output = "\
running 2 tests
test c_0123456789ab::law::hand ... ok
test c_0123456789ab::law::subject ... FAILED

failures:

---- c_0123456789ab::law::subject stdout ----

thread 'c_0123456789ab::law::subject' panicked at src/c_0123456789ab.rs:10:5:
assertion `left == right` failed: m on A with x = 2
  left: \"1\"

failures:
    c_0123456789ab::law::subject
";
        let results = test_results(output);
        assert_eq!(results["c_0123456789ab::law::hand"], TestResult::Passed);
        assert_eq!(
            results["c_0123456789ab::law::subject"],
            TestResult::Failed(String::from(
                "assertion `left == right` failed: m on A with x = 2"
            ))
        );
    }

    #[test]
    fn only_cell_files_are_cells() {
        assert_eq!(
            cell_module("pass-000/src/c_0123456789ab.rs").as_deref(),
            Some("c_0123456789ab")
        );
        assert_eq!(cell_module("pass-000/src/support.rs"), None);
        assert_eq!(cell_module("pass-000/src/c_01.rs"), None);
        assert!(leaks("CARGO_TARGET_DIR") && leaks("RUSTFLAGS") && !leaks("CARGO_HOME"));
        assert!(!leaks("PATH") && !leaks("RUSTUP_TOOLCHAIN"));
    }
}
