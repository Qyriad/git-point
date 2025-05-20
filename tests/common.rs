use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};

use assert_cmd::cargo::CommandCargoExt;

fn maybe_quote<S: AsRef<str>>(s: S) -> String
{
	let s = s.as_ref();
	if s.contains(" ") {
		format!("\"{s}\"")
	} else {
		s.to_string()
	}
}

fn join_words<S, I>(args: I) -> String
where
	S: AsRef<str>,
	I: Iterator<Item = S>,
{
	let words: Vec<String> = args
		.map(maybe_quote)
		.collect();

	words.join(" ")
}

#[derive(Debug)]
pub struct CommandWrapper
{
	pub command: Command,
	pub name: String,
}

impl CommandWrapper
{
	pub fn cargo_bin(name: &str) -> Self
	{
		let command = Command::cargo_bin(name).unwrap();
		let name = name.to_string();

		Self { command, name }
	}

	pub fn new<P: AsRef<Path>>(name: &str, path: P) -> Self
	{
		let command = Command::new(path.as_ref());
		let name = name.to_string();
		Self { command, name }
	}

	pub fn arg<S>(mut self, arg: S) -> Self
	where
		S: AsRef<OsStr>,
	{
		self.command.arg(arg);
		self
	}

	pub fn args<S, I>(mut self, args: I) -> Self
	where
		S: AsRef<OsStr>,
		I: IntoIterator<Item = S>,
	{
		self.command.args(args);
		self
	}

	pub fn assert_spawn(&mut self) -> ChildWrapper
	{
		let args: Vec<String> = self.command
			.get_args()
			.map(|arg| arg.to_string_lossy().to_string())
			.collect();

		let child = self.command.spawn().unwrap_or_else(|e| {
			panic!("error executing '{} {}': {e}", self.name, join_words(args.iter()));
		});

		ChildWrapper {
			child,
			name: self.name.clone(),
			args,
		}
	}

	pub fn assert_spawn_exit_ok(mut self)
	{
		let child = self.assert_spawn();
		child.assert_exit_ok();
	}

	/// This one *does* handle piping stdout/stderr.
	///
	/// Yeah the name is way too long. Whatever.
	pub fn assert_spawn_exit_ok_with_output(mut self) -> Output
	{
		self.command.stdout(Stdio::piped());
		self.command.stderr(Stdio::piped());

		let child = self.assert_spawn();

		child.assert_exit_ok_with_output()
	}
}

#[derive(Debug)]
pub struct ChildWrapper
{
	pub child: Child,
	pub name: String,
	pub args: Vec<String>,
}

impl ChildWrapper
{
	pub fn assert_exit_ok(mut self)
	{
		let status = self.child.wait().unwrap_or_else(|e| {
			panic!(
				"error waiting for command '{} {}': {e} (killed by signal?)",
				self.name,
				join_words(self.args.iter()),
			);
		});

		if !status.success() {
			panic!(
				"command '{} {}' exited with non-zero code {}",
				self.name,
				join_words(self.args.iter()),
				status,
			);
		}
	}

	/// Does *not* setup stdout and stderr piping beforehand. You have to do that yourself.
	pub fn assert_exit_ok_with_output(self) -> Output
	{
		let output = self.child.wait_with_output().unwrap_or_else(|e| {
			let words: Vec<String> = self.args.iter().map(maybe_quote).collect();
			let joined = words.join(" ");
			panic!("error waiting for command '{} {}': {e} (killed by signal?)", self.name, joined);
		});

		if !output.status.success() {
			panic!(
				"command '{} {}' exited with non-zero code {}",
				self.name,
				join_words(self.args.iter()),
				output.status,
			);
		}

		output
	}
}
