use std::env;
use std::error::Error as StdError;
use std::path::PathBuf;

use bstr::{BString, ByteSlice};
use clap::{Parser, ValueEnum, ArgAction};

use gix::refs::transaction::Change;
use gix::refs::transaction::LogChange;
use gix::refs::transaction::PreviousValue;
use gix::refs::transaction::RefEdit;
use gix::refs::transaction::RefLog;
use gix::refs::Target;
use gix::Id;
use gix::Reference;
use gix::Repository;

#[allow(unused)]
use log::{trace, debug, warn, info, error};

use tap::TapFallible;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash)]
#[derive(ValueEnum)]
enum NewRefKind
{
	Branch,
	Tag,
}

#[derive(Debug, Clone, PartialEq, Hash)]
#[derive(Parser)]
#[command(version, author, about)]
struct GitPointCmd
{
	/// ref to update
	pub from: String,

	/// revision to point <FROM> to
	pub to: String,

	/// create a new ref of <KIND> instead of updating an existing one
	#[arg(short, long, action = ArgAction::Set, value_name = "KIND")]
	pub new: Option<NewRefKind>,
}

/// The ref we will mutate.
#[derive(Clone, PartialEq, Hash)]
struct VictimRef<'id>
{
	/// The original, requested revision (`git rev-parse`able).
	revspec: BString,

	/// The short form of the ref, e.g. `main`.
	short: BString,

	/// The fully resolved ID.
	resolved_id: Id<'id>,

	/// The first line of the commit message.
	summary: BString,
}

impl<'id> VictimRef<'id>
{
	/// Constructs a [FromRev] from a [Reference].
	pub fn from(revspec: BString, reference: &'id Reference) -> Result<Self, Box<dyn StdError>>
	{
		let peeled = reference.clone().into_fully_peeled_id()
			.tap_err(|e| error!("while peeling {}: {}", reference.name().as_bstr(), e))?;
		let id = peeled.detach();

		let commit = peeled
			.object()
			.tap_err(|e| error!("while finding object {}: {}", id.to_hex(), e))?
			.try_into_commit()
			.tap_err(|e| error!("{} is not a commit: {}", id.to_hex(), e))?;

		let commit_summary = commit
			.message_raw()
			.tap_err(|e| error!("while getting message of commit {}: {}", id.to_hex(), e))?
			.lines()
			.next()
			.unwrap_or(b"<empty msg>");

		Ok(Self {
			revspec,
			short: reference.name().shorten().to_owned(),
			resolved_id: peeled,
			summary: BString::from(commit_summary.to_vec()),
		})
	}
}

fn main() -> Result<(), Box<dyn StdError>>
{
	env_logger::builder()
		// Default to INFO rather than WARN, but let the user override it.
		.filter_level(log::LevelFilter::Info)
		.parse_default_env()
		.init();

	let args = GitPointCmd::parse();

	let cwd: PathBuf = env::current_dir()
		.expect("cannot open current directory");

	let repo: Repository = gix::open(&cwd)
		.expect(&format!("cannot open Git repo in {}", cwd.display()));

	let ref_to_update = match &args.new {
		Some(kind) => {
			todo!();
		},
		None => {
			repo.find_reference(&args.from)
				.expect(&format!("cannot get ref {}", &args.from))
		},
	};

	let victim = VictimRef::from(BString::from(args.from.clone()), &ref_to_update)?;

	let target_id = repo.rev_parse_single(args.to.as_str())
		.map_err(|e| {
			error!("error parsing revision {}", args.to.as_str());
			e
		})
		.unwrap();
	let target_obj = target_id
		.object()
		.unwrap();
	let target_commit = target_obj
		.clone()
		.into_commit();
	let target_commit_summary = target_commit
		.message_raw()
		.unwrap()
		.lines()
		.next()
		.unwrap();

	eprintln!(
		"Ref \x1b[34m{}\x1b[0m resolved to \x1b[33m{}\x1b[0m ({})",
		args.to.as_str(),
		target_id.shorten_or_id(),
		target_commit_summary.as_bstr()
	);

	let reflog_msg = format!(
		"git-point: updating {} from {} to {}",
		ref_to_update.name().as_bstr(),
		victim.resolved_id,
		target_obj.id,
	);

	let transaction = RefEdit {
		change: Change::Update {
			log: LogChange {
				mode: RefLog::AndReference,
				force_create_reflog: false,
				message: BString::from(reflog_msg),
			},
			expected: PreviousValue::MustExistAndMatch(Target::Peeled(victim.resolved_id.into())),
			new: Target::Peeled(target_obj.id),
		},
		name: ref_to_update.name().to_owned(),
		deref: false,
	};

	let _edits = repo.edit_reference(transaction.clone()).unwrap();

	eprintln!(
		"Updated \x1b[34m{refname}\x1b[0m from \x1b[33m{previd}\x1b[0m ({prevmsg}) to \x1b[33m{newid}\x1b[0m ({newmsg})",
		refname = ref_to_update.name().shorten(),
		previd = victim.resolved_id.shorten_or_id(),
		prevmsg = victim.summary.as_bstr(),
		newid = target_id.shorten_or_id(),
		newmsg = target_commit_summary.as_bstr(),
	);

	Ok(())
}
