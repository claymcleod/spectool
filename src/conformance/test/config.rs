use serde::Deserialize;
use serde::Serialize;

/// A configuration for a conformance test.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// An array of the test's dependencies.
    #[serde(default)]
    dependencies: Vec<String>,

    /// Output keys to ignore when testing.
    #[serde(default)]
    exclude_output: Vec<String>,

    /// Whether or not the test is expected to fail.
    #[serde(default)]
    fail: bool,

    /// The expected return code.
    #[serde(default)]
    return_code: usize,

    /// A set of tags.
    #[serde(default)]
    tags: Vec<String>,

    /// The target of the conformance test.
    target: Option<String>,
}
