use std::env;
use std::error::Error as StdError;
use std::iter;
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

	/// Allow mutating checked out refs.
	/// Note that this will *not* change any of the actual files in the worktree.
	#[arg(long, short = 'W', action = ArgAction::SetTrue)]
	pub allow_worktree: bool,
}

/// The ref we will mutate.
#[derive(Debug, Clone, PartialEq, Hash)]
struct VictimRef<'repo>
{
	/// The original, requested revision (`git rev-parse`able).
	revspec: BString,

	/// The short form of the ref, e.g. `main`.
	short: BString,

	/// The fully resolved commit ID that the ref to be mutated
	/// points to, before the mutation.
	resolved_id: Id<'repo>,

	/// The first line of the commit message.
	summary: BString,
}

impl<'repo> VictimRef<'repo>
{
	/// Constructs a [VictimRef] from a [Reference].
	pub fn from(revspec: BString, reference: &'repo Reference) -> Result<Self, Box<dyn StdError>>
	{
		let peeled = reference.clone().into_fully_peeled_id()
			.tap_err(|e| error!("while peeling {}: {}", reference.name().as_bstr(), e))?;
		let id = peeled.detach();

		let commit = peeled
			.object()
			.tap_err(|e| error!("while finding object {}: {}", id.to_hex(), e))?
			.into_commit();

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

/// The revision we will mutate the [VictimRef] to.
#[derive(Debug, Clone, PartialEq, Hash)]
struct TargetRev<'repo>
{
	/// The original, requested revision (`git rev-parse`able).
	revspec: BString,

	/// The fully resolved commit ID we're going to mutate the [VictimRef] to.
	resolved_id: Id<'repo>,

	/// The first line of the commit message.
	summary: BString,
}

impl<'repo> TargetRev<'repo>
{
	/// Constructs [TargetRev] from a revspec.
	pub fn from(repo: &'repo Repository, revspec: BString) -> Result<Self, Box<dyn StdError>>
	{
		let id = repo.rev_parse_single(revspec.as_bstr())
			.tap_err(|e| error!("while resolving revspec {}: {}", &revspec, e))?;

		let commit = id
			.object()
			.tap_err(|e| error!("while finding object {}: {}", id.to_hex(), e))?
			.into_commit();

		let summary = commit
			.message_raw()
			.tap_err(|e| error!("while getting message of commit {}: {}", id.to_hex(), e))?
			.lines()
			.next()
			.unwrap_or(b"<empty msg>");

		Ok(Self {
			revspec,
			resolved_id: id,
			summary: BString::from(summary.to_vec()),
		})
	}
}

/// Will std::process:exit() if check condition matches.
fn check_worktrees(repo: &Repository, victim_ref: &Reference)
{
	let worktrees = repo
		.worktrees()
		.tap_err(|e| warn!("ignoring error finding active worktrees: {}", e))
		.unwrap_or_else(|_e| Vec::new());

	let worktree_repos = worktrees
		.into_iter()
		.filter_map(|worktree| {
			let id = worktree.id().to_owned();
			worktree
				.into_repo_with_possibly_inaccessible_worktree()
				.tap_err(|e| warn!("ignoring error accessing worktree {}: {}", id, e))
				.ok()
		})
		.chain(iter::once(repo.clone()));

	for tree_repo in worktree_repos {
		let dir = tree_repo.work_dir().expect("unreachable");
		eprintln!("looking at worktree {}", dir.display());

		let tree_head = tree_repo
			.head_ref()
			.tap_err(|e| warn!("ignoring error discovering worktree {} HEAD: {}", dir.display(), e));
		let Ok(tree_head) = tree_head else {
			continue;
		};

		if tree_head.as_ref().map(|r| &r.inner) == Some(&victim_ref.inner) {
			eprintln!(
				"\x1b[91merror:\x1b[0m refusing to update ref \x1b[34m{}\x1b[0m checked out at {}; \
				pass --allow-worktree to override",
				victim_ref.name().shorten(),
				dir.display(),
			);

			std::process::exit(1);
		}
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

	let cwd: PathBuf = env::current_dir()?;

	let repo: Repository = gix::open(&cwd)
		.tap_err(|e| error!("while opening git repo in {}: {}", cwd.display(), e))?;

	let victim_ref = match &args.new {
		Some(kind) => {
			todo!();
		},
		None => repo.find_reference(&args.from)
			.tap_err(|e| error!("while finding reference {}: {}", &args.from, e))?
	};

	let victim = VictimRef::from(BString::from(args.from), &victim_ref)?;
	let target = TargetRev::from(&repo, BString::from(args.to))?;

	if !args.allow_worktree {
		// Check if the victim *ref* is checked out anywhere.
		// This function will exit the process if so.
		check_worktrees(&repo, &victim_ref);
	}

	let reflog_msg = format!(
		"git-point: updating {} from {} to {}",
		victim_ref.name().as_bstr(),
		victim.resolved_id,
		target.resolved_id,
	);

	let transaction = RefEdit {
		change: Change::Update {
			log: LogChange {
				mode: RefLog::AndReference,
				force_create_reflog: false,
				message: BString::from(reflog_msg),
			},
			expected: PreviousValue::MustExistAndMatch(Target::Peeled(victim.resolved_id.into())),
			new: Target::Peeled(target.resolved_id.detach()),
		},
		name: victim_ref.name().to_owned(),
		deref: false,
	};

	let _edits = repo.edit_reference(transaction.clone()).unwrap();

	eprintln!(
		"Updated \x1b[34m{refname}\x1b[0m from \x1b[33m{previd}\x1b[0m ({prevmsg}) to \x1b[33m{newid}\x1b[0m ({newmsg})",
		refname = victim_ref.name().shorten(),
		previd = victim.resolved_id.shorten_or_id(),
		prevmsg = victim.summary.as_bstr(),
		newid = target.resolved_id.shorten_or_id(),
		newmsg = target.summary.as_bstr(),
	);

	Ok(())
}
