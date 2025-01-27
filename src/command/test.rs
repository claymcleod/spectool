use std::fs::DirEntry;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use clap::Parser;
use miette::bail;
use miette::Context as _;
use miette::IntoDiagnostic;
use miette::Result;

use crate::conformance::test::Runner;
use crate::conformance::Test;
use crate::shell::substitute;
use crate::Repository;

/// The file name of the specification.
const SPEC_FILE_NAME: &str = "SPEC.md";

/// Performs conformance tests on the WDL specification.
#[derive(Parser, Debug)]
pub struct Args {
    /// A directory that contains the conformance tests.
    #[arg(short, long)]
    conformance_test_dir: Option<PathBuf>,

    /// Whether to force the writing of the conformance tests directory.
    #[arg(short, long, default_value_t = false)]
    force: bool,

    /// A directory that contains the specification repository.
    #[arg(short, long)]
    specification_dir: Option<PathBuf>,

    /// The command to call for each execution.
    ///
    /// * `~{path}` is the path to the file.
    command: String,
}

pub fn main(args: Args) -> Result<()> {
    //=======================================//
    // Checkout the specification repository //
    //=======================================//

    let (_, path) = Repository::builder()
        .maybe_local_dir(args.specification_dir)
        .build()
        .checkout()?;

    //=================================//
    // Read the specification contents //
    //=================================//

    let spec = path.join(SPEC_FILE_NAME);

    if !spec.exists() {
        bail!(
            "the specification does not exist at `{}` in the git repository",
            SPEC_FILE_NAME
        );
    }

    let contents = std::fs::read_to_string(spec).into_diagnostic()?;

    //===============================//
    // Compile the conformance tests //
    //===============================//

    let root_dir = args
        .conformance_test_dir
        .unwrap_or_else(|| tempfile::tempdir().expect("tempdir to create").into_path());

    let runner = Runner::compile(root_dir, contents, args.force)?;

    //===================================//
    // Set up the test working directory //
    //===================================//

    // SAFETY: this should create on all platforms we care about.
    let workdir = tempfile::tempdir().expect("tempdir to create").into_path();

    //===============//
    // Run the tests //
    //===============//

    for test in runner.tests() {
        // (1) Recreate the directory to ensure it's empty.
        // SAFETY: we expect to be able to remove and recreate the directory on all
        // platforms we care about within this subcommand.
        std::fs::remove_dir_all(&workdir).unwrap();
        std::fs::create_dir_all(&workdir).unwrap();

        // (2) Copy the entries in the `data` directory.
        copy_directory(&runner.data_dir(), &workdir).unwrap();

        // (3) Create the inputs file.
        let input_file = create_input_json(test, &workdir).unwrap();

        // (4) Substitute the command.
        let command = substitute()
            .command(args.command.clone())
            .path(test.path().unwrap().to_path_buf())
            .input(input_file)
            .call();

        // (5) Run the command;
        execute(command).unwrap();
    }

    Ok(())
}

/// Copies the contents of a directory to another directory
fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    let entries = std::fs::read_dir(source)
        .into_diagnostic()
        .context("reading the data directory")?
        .collect::<Result<Vec<DirEntry>, _>>()
        .into_diagnostic()
        .context("collecting the data entries")?;

    for source in entries {
        assert!(source.path().is_file(), "only files are supported");
        let destination = destination.join(source.file_name().to_string_lossy().to_string());
        std::fs::copy(source.path(), &destination).into_diagnostic()?;
    }

    Ok(())
}

/// Creates an `input.json` file.
fn create_input_json(test: &Test, work_dir: &Path) -> Result<PathBuf> {
    let input = match test.input() {
        Some(value) => serde_json::to_string_pretty(value)
            .into_diagnostic()
            .context("serializing input file")?,
        None => Default::default(),
    };

    let input_file_path = work_dir.join("inputs.json");
    std::fs::write(&input_file_path, input)
        .into_diagnostic()
        .context("writing `inputs.json` file")?;

    Ok(input_file_path)
}

/// Executes the engine running command.
fn execute(command: String) -> Result<()> {
    let output = Command::new("bash")
        .args(["-c", &command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .into_diagnostic()
        .context("running engine command")?;

    dbg!(output);

    Ok(())
}
