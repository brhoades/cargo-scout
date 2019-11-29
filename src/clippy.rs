use serde::Deserialize;
use std::process::Command;

#[derive(Deserialize, PartialEq, Debug)]
pub struct LintCode {
    pub code: String,
    pub explanation: String,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct LintSpan {
    pub file_name: String,
    /// The line where the lint should be reported
    ///
    /// GitHub provides a line_start and a line_end.
    /// We should use the line_start in case of multi-line lints.
    /// (Why?)
    pub line_start: usize,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Lint {
    /// The lint message
    ///
    /// Example:
    ///
    /// unused variable: `count`
    pub package_id: String,
    pub src_path: Option<String>,
    pub message: Option<Message>,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Message {
    pub rendered: String,
    pub spans: Vec<Span>,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Span {
    pub file_name: String,
    pub line_start: i32,
    pub line_end: i32,
}

pub struct Linter {
    verbose: bool,
}

impl Linter {
    pub fn new() -> Self {
        Self { verbose: false }
    }

    pub fn set_verbose(&mut self, verbose: bool) -> &mut Self {
        self.verbose = verbose;
        self
    }

    pub fn get_lints(&self) -> Result<Vec<Lint>, crate::error::Error> {
        self.clippy().map(|output| lints(&output))
    }

    fn clippy(&self) -> Result<String, crate::error::Error> {
        let clippy_pedantic_output = if self.verbose {
            Command::new("cargo")
                .args(&[
                    "clippy",
                    "--verbose",
                    "--message-format",
                    "json",
                    "--",
                    "-W",
                    "clippy::pedantic",
                ])
                .envs(std::env::vars())
                .env("RUST_BACKTRACE", "full")
                .output()
                .expect("failed to run clippy pedantic")
        } else {
            Command::new("cargo")
                .args(&[
                    "clippy",
                    "--message-format",
                    "json",
                    "--",
                    "-W",
                    "clippy::pedantic",
                ])
                .envs(std::env::vars())
                .output()
                .expect("failed to run clippy pedantic")
        };
        if self.verbose {
            println!(
                "{}",
                String::from_utf8(clippy_pedantic_output.stdout.clone())?
            );
        }
        if clippy_pedantic_output.status.success() {
            Ok(String::from_utf8(clippy_pedantic_output.stdout)?)
        } else if self.verbose {
            println!("Clippy run failed");
            println!("cleaning and building with full backtrace");
            let _ = Command::new("cargo")
                .args(&["clean"])
                .envs(std::env::vars())
                .env("RUST_BACKTRACE", "full")
                .output()
                .expect("failed to start cargo clean");
            let build = Command::new("cargo")
                .args(&["build"])
                .envs(std::env::vars())
                .env("RUST_BACKTRACE", "full")
                .output()
                .expect("failed to start cargo build");
            if build.status.success() {
                Err(String::from_utf8(build.stdout)?.into())
            } else {
                println!("{}", String::from_utf8(build.stdout.clone())?);
                Err(String::from_utf8(build.stderr)?.into())
            }
        } else {
            Err(String::from_utf8(clippy_pedantic_output.stderr)?.into())
        }
    }
}

pub fn lints(clippy_output: &str) -> Vec<Lint> {
    clippy_output
        .lines()
        .filter(|l| l.starts_with('{'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|lint: &Lint| {
            if let Some(m) = &lint.message {
                !m.spans.is_empty()
            } else {
                false
            }
        })
        .collect::<Vec<Lint>>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_set_verbose() {
        use crate::clippy::Linter;

        let mut linter = Linter::new();
        assert_eq!(false, linter.verbose);

        let l2 = linter.set_verbose(true);
        assert_eq!(true, l2.verbose);

        let l3 = l2.set_verbose(false);
        assert_eq!(false, l3.verbose);
    }
    #[test]
    fn test_lints() {
        use crate::clippy::{lints, Lint, Message, Span};
        let expected_lints = vec![Lint {
            package_id: "cargo-scout".to_string(),
            src_path: Some("test/foo/bar.rs".to_string()),
            message: Some(Message {
                rendered: "this is a test lint".to_string(),
                spans: vec![Span {
                    file_name: "test/foo/baz.rs".to_string(),
                    line_start: 10,
                    line_end: 12,
                }],
            }),
        }];

        let clippy_output = r#"{"package_id": "cargo-scout","src_path": "test/foo/bar.rs","message": { "rendered": "this is a test lint","spans": [{"file_name": "test/foo/baz.rs","line_start": 10,"line_end": 12}]}}"#;

        assert_eq!(expected_lints, lints(clippy_output));
    }
}
