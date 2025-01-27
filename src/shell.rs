use std::path::PathBuf;

use bon::builder;

/// Resolves a command with the supported replacements.
#[builder]
pub fn substitute(mut command: String, path: PathBuf, input: PathBuf) -> String {
    command = command.replace("~{path}", &path.display().to_string());
    command = command.replace("~{input}", &input.display().to_string());
    command
}
