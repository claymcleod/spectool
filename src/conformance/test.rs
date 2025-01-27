use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

use bon::builder;
use bon::Builder;
use miette::miette;
use miette::Context;
use miette::IntoDiagnostic;
use miette::Result;
use regex::Captures;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde_json::Value;

mod config;
pub mod runner;

pub use config::Config;
pub use runner::Runner;

/// The regex for a WDL conformance test within the specification.
static CONFORMANCE_TEST_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    const PATTERN: &str = concat!(
        "(?is)", // Turn on `i` and `s` options.
        r"<details>\s*",
        r"<summary>\s*",
        r"Example: (.+?)\s*```wdl(.+?)```\s*",
        r"</summary>\s*",
        r"(?:<p>\s*",
        r"(?:Example input:\s*```json(.*?)```)?\s*",
        r"(?:Example output:\s*```json(.*?)```)?\s*",
        r"(?:Test config:\s*```json(.*?)```)?\s*",
        r"</p>\s*",
        r")?",
        r"</details>"
    );

    Regex::new(PATTERN).unwrap()
});

/// A conformance test.
#[derive(Builder, Debug)]
#[builder(builder_type = Builder)]
pub struct Test {
    /// The path to the test, if has been written.
    path: Option<PathBuf>,

    /// The file name of the test.
    file_name: String,

    /// The source.
    src: String,

    /// The input.
    input: Option<Value>,

    /// The output.
    output: Option<Value>,

    /// The configuration.
    config: Config,
}

impl Test {
    /// The path to the test.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file name of the test.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The source of the test.
    pub fn src(&self) -> &str {
        &self.src
    }

    /// The input of the test.
    pub fn input(&self) -> Option<&Value> {
        self.input.as_ref()
    }

    /// The output of the test.
    pub fn output(&self) -> Option<&Value> {
        self.output.as_ref()
    }

    /// The configuration of the test.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Sets the path for the test.
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }
}

/// A set of conformance tests.
pub struct Tests(Vec<Test>);

impl Tests {
    /// Turns a markdown specification into a set of conformance tests.
    pub fn compile<S: AsRef<str>>(contents: S) -> Result<Self> {
        let contents = contents.as_ref();

        let tests = CONFORMANCE_TEST_REGEX
            .captures_iter(contents)
            .map(build_conformance_test)
            .collect::<Result<Vec<Test>, _>>()?;

        Ok(Self(tests))
    }

    /// Returns a reference to each conformance test.
    pub fn tests(&self) -> impl Iterator<Item = &Test> {
        self.0.iter()
    }

    /// Returns a mutable reference to each conformance test.
    pub fn tests_mut(&mut self) -> impl Iterator<Item = &mut Test> {
        self.0.iter_mut()
    }

    /// Consumes `self` and returns the conformance tests.
    pub fn into_tests(self) -> impl Iterator<Item = Test> {
        self.0.into_iter()
    }
}

/// Builds a conformance test from a set of captures.
fn build_conformance_test(captures: Captures<'_>) -> Result<Test> {
    let file_name = required_string(&captures, 1, "filename")?;
    let src = required_string(&captures, 2, "source")?;
    let input = optional_json_group(&captures, 3);
    let output = optional_json_group(&captures, 4);
    let config = optional_group::<Config>(&captures, 5)?.unwrap_or_default();

    Ok(Test::builder()
        .file_name(file_name)
        .src(src)
        .maybe_input(input)
        .maybe_output(output)
        .config(config)
        .build())
}

/// Parses a _required_ group within a test.
fn required_string(captures: &Captures, index: usize, name: &str) -> Result<String> {
    captures
        .get(index)
        .ok_or_else(|| {
            miette!(
                "unable to parse {} from test:\n\n{}",
                name,
                captures.get(0).unwrap().as_str()
            )
        })
        .map(|v| v.as_str().to_owned())
}

/// Parses an _optional_ group within a test.
fn optional_json_group(captures: &Captures, index: usize) -> Option<Value> {
    captures.get(index).and_then(|v| v.as_str().parse().ok())
}

/// Parses an _optional_ group within a test.
fn optional_group<D>(captures: &Captures, index: usize) -> Result<Option<D>>
where
    D: DeserializeOwned,
{
    captures
        .get(index)
        .map(|m| {
            serde_json::from_str::<D>(m.as_str())
                .into_diagnostic()
                .with_context(|| {
                    format!(
                        "parsing configuration:\n\n{}",
                        captures.get(0).unwrap().as_str()
                    )
                })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_matches() {
        let example = r#"
        <details>
            <summary>
                Example: hello.wdl

                ```wdl
                version 1.2

                task hello_task {
                    input {
                    File infile
                    String pattern
                    }

                    command <<<
                    grep -E '~{pattern}' '~{infile}'
                    >>>

                    requirements {
                    container: "ubuntu:latest"
                    }

                    output {
                    Array[String] matches = read_lines(stdout())
                    }
                }

                workflow hello {
                    input {
                    File infile
                    String pattern
                    }

                    call hello_task {
                    infile, pattern
                    }

                    output {
                    Array[String] matches = hello_task.matches
                    }
                }
                ```
            </summary>
            <p>
            Example input:

            ```json
            {
                "hello.infile": "greetings.txt",
                "hello.pattern": "hello.*"
            }
            ```

            Example output:

            ```json
            {
                "hello.matches": ["hello world", "hello nurse"]
            }
            ```
            </p>
        </details>"#;

        let captures = CONFORMANCE_TEST_REGEX
            .find_iter(example)
            .collect::<Vec<_>>();
        assert_eq!(captures.len(), 1);
    }
}
