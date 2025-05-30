use std::env;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use common::CommandWrapper;

mod common;

const CARGO_TARGET_TMPDIR: &str = env!("CARGO_TARGET_TMPDIR");

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
		.unwrap_or_else(|e| panic!("cannot cd into {}: {e}", directory.display()));

	let res = f(directory);

	env::set_current_dir(&current_dir)
		.unwrap_or_else(|e| panic!("cannot cd back to original directory {}: {e}", current_dir.display()));

	res
}

fn setup_git(gitcmd: &dyn Fn() -> CommandWrapper)
{
	gitcmd()
		.args(["init", "--initial-branch=main"])
		.assert_spawn_exit_ok();

	gitcmd()
		.arg("config")
		.args(["user.name", "dummy"])
		.assert_spawn_exit_ok();

	gitcmd()
		.arg("config")
		.args(["user.email", "dummy@example.com"])
		.assert_spawn_exit_ok();
}

#[test]
fn basic()
{
	let git = GIT.as_path();

	let gitcmd = || CommandWrapper::new("git", git);
	let gitpointcmd = || CommandWrapper::cargo_bin("git-point");

	let tempdir = tempfile::Builder::new()
		.tempdir_in(CARGO_TARGET_TMPDIR)
		.unwrap_or_else(|e| panic!("cannot create temporary directory in {} for test: {e}", CARGO_TARGET_TMPDIR));

	with_dir(tempdir.path(), |_dir| {

		setup_git(&gitcmd);

		gitcmd()
			.args(["commit", "--allow-empty", "-m", "initial commit"])
			.assert_spawn_exit_ok();

		let initial_commit = gitcmd()
			.args(["rev-parse", "@"])
			.assert_spawn_exit_ok_with_output();

		gitcmd()
			.args(["branch", "initial"])
			.assert_spawn_exit_ok();

		let initial_branch_rev = gitcmd()
			.args(["rev-parse", "initial"])
			.assert_spawn_exit_ok_with_output();

		assert_eq!(initial_commit, initial_branch_rev);

		gitcmd()
			.args(["commit", "--allow-empty", "-m", "second-commit"])
			.assert_spawn_exit_ok();

		let second_commit = gitcmd()
			.args(["rev-parse", "@"])
			.assert_spawn_exit_ok_with_output();

		gitpointcmd()
			.args(["initial", "@"])
			.assert_spawn_exit_ok();

		let new_initial_branch_rev = gitcmd()
			.args(["rev-parse", "initial"])
			.assert_spawn_exit_ok_with_output();

		assert_eq!(new_initial_branch_rev, second_commit);
	});
}
