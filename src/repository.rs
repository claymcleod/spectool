use std::path::Path;
use std::path::PathBuf;

use bon::Builder;
use bon::builder;
use git2::FetchOptions;
use miette::IntoDiagnostic;
use miette::Result;
use tracing::info;

const REPOSITORY_URL: &str = "https://github.com/openwdl/wdl.git";

/// The WDL specification repository.
#[derive(Builder)]
#[builder(builder_type = Builder)]
pub struct Repository {
    /// The local directory.
    ///
    /// An empty local directory signifies that a temporary directory should be created
    /// upon checkout.
    // NOTE: this is not created as a default with the `bon` builder because we don't
    // want to create a new temporary directory with every test.
    local_dir: Option<PathBuf>,

    /// The remote url.
    #[builder(default = REPOSITORY_URL.to_owned())]
    url: String,
}

impl Repository {
    /// Checks out the repository and returns a [`git2::Repository`].
    pub fn checkout(self) -> Result<(git2::Repository, PathBuf)> {
        let path = self.local_dir.unwrap_or_else(|| {
            // SAFETY: on all the platforms we support, we expect a temporary directory
            // to be able to be created.
            let path = tempfile::tempdir()
                .expect("temporary directory to create")
                .into_path()
                .join("wdl");

            info!("created temporary directory: {}", path.display());

            path
        });

        if path.exists() {
            // If the directory already exists, that directory is assumed to be the git
            // repository checked out on a different run.
            info!("using existing git repository: {}", path.display());
            return git2::Repository::open(&path)
                .into_diagnostic()
                .map(|repo| (repo, path));
        }

        info!("using existing git repository: {}", path.display());
        let mut fetch_options = FetchOptions::new();
        fetch_options.depth(1);

        git2::build::RepoBuilder::new()
            .fetch_options(fetch_options)
            .clone(&self.url, &path)
            .into_diagnostic()
            .map(|repo| (repo, path))
    }

    /// Gets a reference to the local directory.
    pub fn local_dir(&self) -> Option<&Path> {
        self.local_dir.as_deref()
    }

    /// Gets a reference to the URL.
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_url() {
        let repo = Repository::builder().build();

        assert!(repo.local_dir.is_none());
        assert_eq!(repo.url(), &*REPOSITORY_URL);
    }
}
