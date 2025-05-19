use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use assert_cmd::{cargo::CommandCargoExt, assert::OutputAssertExt};

const CARGO_TARGET_TMPDIR: &'static str = env!("CARGO_TARGET_TMPDIR");

static GIT: LazyLock<PathBuf> = LazyLock::new(|| {
	which::which("git")
		.ok()
		.or_else(|| env::var_os("GIT").map(PathBuf::from))
		.expect("cannot find `git` executable in $PATH or $GIT environment variable")
});

fn with_dir<R, F>(directory: &Path, f: F) -> R
where
	F: FnOnce(&Path) -> R
{
	let current_dir = env::current_dir().expect("cannot get current working directory");

	env::set_current_dir(directory)
		.expect(&format!("cannot cd into {}", directory.display()));

	let res = f(directory);

	env::set_current_dir(&current_dir)
		.expect(&format!("cannot cd back to original directory {}", current_dir.display()));

	res
}

#[test]
fn basic()
{
	let git = GIT.as_path();

	let tempdir = tempfile::Builder::new()
		.tempdir_in(CARGO_TARGET_TMPDIR)
		.expect(&format!("cannot create temporary directory in {} for test", CARGO_TARGET_TMPDIR));

	with_dir(tempdir.path(), |_dir| {
		Command::new(git)
			.arg("init")
			.assert()
			.success();

		Command::new(git)
			.args(&["commit", "--allow-empty", "-m", "initial commit"])
			.assert()
			.success();

		let initial_commit = Command::new(&*GIT)
			.args(&["rev-parse", "@"])
			.assert()
			.success()
			.get_output()
			.to_owned();

		Command::new(git)
			.args(&["branch", "initial"])
			.assert()
			.success();

		let initial_branch_rev = Command::new(git)
			.args(&["rev-parse", "initial"])
			.assert()
			.success()
			.get_output()
			.to_owned();

		assert_eq!(initial_commit, initial_branch_rev);

		Command::new(git)
			.args(&["commit", "--allow-empty", "-m", "second-commit"])
			.assert()
			.success();

		let second_commit = Command::new(git)
			.args(&["rev-parse", "@"])
			.assert()
			.success()
			.get_output()
			.to_owned();

		Command::cargo_bin("git-point")
			.unwrap()
			.arg("initial")
			.arg("@")
			.assert()
			.success();

		let new_initial_branch_rev = Command::new(git)
			.args(&["rev-parse", "initial"])
			.assert()
			.success()
			.get_output()
			.to_owned();

		assert_eq!(new_initial_branch_rev, second_commit);
	});
}
