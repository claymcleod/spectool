use std::fs::DirEntry;
use std::path::Path;
use std::path::PathBuf;

use miette::bail;
use miette::Context;
use miette::IntoDiagnostic;
use miette::Result;
use tracing::info;
use tracing::warn;

use crate::conformance;

/// A runner for conformance tests.
pub struct Runner {
    /// The root directory of the conformance test suite.
    root_dir: PathBuf,

    /// The conformance tests to execute.
    tests: conformance::Tests,
}

impl Runner {
    /// Compiles conformance tests.
    pub fn compile<S: AsRef<str>>(root_dir: PathBuf, contents: S, force: bool) -> Result<Self> {
        let contents = contents.as_ref();

        //=========================================//
        // Prepare the conformance tests directory //
        //=========================================//

        info!(
            "preparing conformance tests directory: {}",
            root_dir.display()
        );

        ensure_empty_dir(&root_dir, force)?;

        //==================================//
        // Ensure the data directory exists //
        //==================================//

        let data_dir = root_dir.join("data");
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .into_diagnostic()
                .context("creating the `data` directory")?;
        }

        //================================//
        // Gather and write the resources //
        //================================//

        let resources = conformance::Resources::compile(contents)?;

        for resource in resources.iter() {
            let file_path = data_dir.join(resource.filename());
            if file_path.exists() {
                bail!(
                    "resource with name `{}` was attempted to be written multiple times",
                    file_path.display()
                );
            }

            std::fs::write(file_path, resource.src())
                .into_diagnostic()
                .with_context(|| format!("writing `{}` resource file", resource.filename()))?;
        }

        //===============================//
        // Compile the conformance tests //
        //===============================//

        let mut tests = conformance::Tests::compile(contents)?;

        for test in tests.tests_mut() {
            let file_path = root_dir.join(test.file_name());
            if file_path.exists() {
                bail!(
                    "conformance test with name `{}` was attempted to be written multiple times",
                    file_path.display()
                );
            }

            std::fs::write(&file_path, test.src())
                .into_diagnostic()
                .with_context(|| format!("writing `{}` conformance test", test.file_name()))?;

            test.set_path(file_path);
        }

        Ok(Self { root_dir, tests })
    }

    /// Gets the root directory.
    pub fn root_dir(&self) -> &Path {
        self.root_dir.as_path()
    }

    /// Gets the data directory.
    pub fn data_dir(&self) -> PathBuf {
        self.root_dir.join("data")
    }

    /// Gets the tests within the runner.
    pub fn tests(&self) -> impl Iterator<Item = &conformance::Test> {
        self.tests.tests()
    }
}

/// Ensures that the directory exists and is empty.
fn ensure_empty_dir<P: AsRef<Path>>(path: P, force: bool) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        std::fs::create_dir_all(path)
            .into_diagnostic()
            .context("creating conformance tests directory")?;
    }

    if !path.is_dir() {
        bail!("item at conformance tests directory path is not a directory!");
    }

    let entries = std::fs::read_dir(path)
        .into_diagnostic()
        .context("reading conformance tests directory")?
        .collect::<Result<Vec<DirEntry>, _>>()
        .into_diagnostic()
        .context("collecting the conformance tests directory entries")?;

    if !entries.is_empty() {
        if force {
            warn!(
                "removing {} existing directory entries as `--force` was applied",
                entries.len()
            );

            for entry in entries {
                let path = entry.path();

                if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                        .into_diagnostic()
                        .with_context(|| format!("removing directory: `{}`", path.display()))?;
                } else {
                    std::fs::remove_file(&path)
                        .into_diagnostic()
                        .with_context(|| format!("removing file: `{}`", path.display()))?;
                }
            }
        } else {
            bail!(
                "{count} existing {entries_exist} in {dir}, but `--force` was not provided to overwrite {them}",
                count = entries.len(),
                dir = path.display(),
                entries_exist = {
                    if entries.len() == 1 {
                        "entry exists"
                    } else {
                        "entries exist"
                    }
                },
                them = {
                    if entries.len() == 1 {
                        "it"
                    } else {
                        "them"
                    }
                },
            );
        }
    }

    Ok(())
}
