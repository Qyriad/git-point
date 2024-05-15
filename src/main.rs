use std::env;
use std::error::Error as StdError;
use std::iter;
use std::path::PathBuf;

use bstr::{BStr, BString, ByteSlice};
use clap::CommandFactory;
use clap::{Parser, ValueEnum, ArgAction};

use gix::refs::transaction::Change;
use gix::refs::transaction::LogChange;
use gix::refs::transaction::PreviousValue;
use gix::refs::transaction::RefEdit;
use gix::refs::transaction::RefLog;
use gix::refs::{FullName, Target};
use gix::refs::Category as RefCategory;
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
	/// New lightweight tag in refs/tags/<FROM>
	Tag,

	/// New branch refs/heads/<FROM>
	Branch,

	/// refs/remotes/<FROM> (e.g. refs/remotes/origin/main)
	RemoteBranch,

	/// No prefix, interpreted literally (like update-ref, be careful!).
	Raw,

	// TODO: notes?
}

impl NewRefKind
{
	fn to_prefix(self) -> &'static BStr
	{
		use NewRefKind::*;
		match self {
			Tag => RefCategory::Tag.prefix(),
			Branch => RefCategory::LocalBranch.prefix(),
			RemoteBranch => RefCategory::RemoteBranch.prefix(),
			Raw => BStr::new(b""),
		}
	}
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

	/// Generates man pages.
	#[arg(long, hide = true)]
	pub mangen: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
enum Victim<'repo>
{
	Known(KnownVictim<'repo>),
	New(NewVictim),
}

#[derive(Debug, Clone, PartialEq, Hash)]
struct NewVictim
{
	revspec: BString,
	/// The fully qualified name of the ref, e.g. refs/heads/main.
	name: BString,
	short: BString,
}

impl<'repo> Victim<'repo>
{
	pub fn name_bstr(&self) -> &BStr
	{
		use Victim::*;
		match self {
			Known(victim) => victim.name.as_bstr(),
			New(new) => new.name.as_bstr(),
		}
	}
}

/// The ref we will mutate.
#[derive(Debug, Clone, PartialEq, Hash)]
struct KnownVictim<'repo>
{
	/// The original, requested revision (`git rev-parse`able).
	revspec: BString,

	/// Rich object representing the fully qualified name of the ref, e.g. `refs/heads/main`.
	name: FullName,

	/// The short form of the ref, e.g. `main`.
	short: BString,

	/// The fully resolved commit ID that the ref to be mutated
	/// points to, before the mutation.
	resolved_id: Id<'repo>,

	/// The first line of the commit message.
	summary: BString,
}

impl<'repo> KnownVictim<'repo>
{
	/// Constructs a [VictimRef] from a [Reference].
	pub fn from(revspec: BString, reference: Reference<'repo>) -> Result<Self, Box<dyn StdError>>
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
			name: reference.name().to_owned(),
			short: reference.name().shorten().to_owned(),
			resolved_id: peeled,
			summary: BString::from(commit_summary.to_vec()),
		})
	}
}

impl NewVictim
{
    pub fn new(kind: NewRefKind, revspec: BString) -> Self
    {
        let prefix = kind.to_prefix();
        let refname: BString = prefix
            .iter()
            .chain(revspec.as_bytes())
            .copied()
            .collect();
        debug!("going to create ref {}", &refname);

        Self {
            revspec,
            short: refname.strip_prefix(prefix.as_bytes()).unwrap_or(&refname).into(),
			// lol, has to be in this order to avoid a clone().
			name: refname,
        }
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
		debug!("checking if worktree {} has {} checked out", dir.display(), victim_ref.name().as_bstr());

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

	if let Some(out_path) = args.mangen {
		let man = clap_mangen::Man::new(GitPointCmd::command());

		let mut man_buffer: Vec<u8> = Default::default();
		man.render(&mut man_buffer)?;

		std::fs::write(out_path.join("git-point.1"), man_buffer)?;

		eprintln!("wrote man pages to {}", out_path.display());
		std::process::exit(0);
	}

	let cwd: PathBuf = env::current_dir()?;

	let repo: Repository = gix::open(&cwd)
		.tap_err(|e| error!("while opening git repo in {}: {}", cwd.display(), e))?;

	let victim = match &args.new {
		Some(kind) => {
			debug!("requested to create new {} ref", kind.to_prefix());

			// Disallow if the ref already exists, though we will
			// enforce this at the transaction level below as well.
			let maybe_existing = repo.try_find_reference(&args.from)
				.tap_err(|e| warn!("ignoring error checking if {} already exists: {}", args.from, e));

			if let Ok(Some(existing_ref)) = maybe_existing {

				let existing_id = existing_ref
					.clone()
					.into_fully_peeled_id()
					.map(|peeled| peeled.to_hex().to_string())
					.unwrap_or_else(|e| {
						warn!("error resolving existing ref {}: {}", existing_ref.name().as_bstr(), e);
						String::from("<could not resolve>")
					});

				eprintln!(
					"\x1b[91merror:\x1b[0m refusing to create ref \x1b[34m{}\x1b[0m which already exists at \x1b[33m{}\x1b[0m",
					existing_ref.name().as_bstr(),
					existing_id,
				);

				std::process::exit(2);
			}

			Victim::New(NewVictim::new(*kind, BString::from(args.from.clone())))
		},
		None => {
			let reference = repo.find_reference(&args.from)
				.tap_err(|e| error!("while finding reference {}: {}", &args.from, e))?;

			if !args.allow_worktree {
				// Check if the victim *ref* is checked out anywhere.
				// This function will exit the process if so.
				// Technically this is a TOC/TOU race condition, but if someone else is
				// concurrently mutating this repo then we're fucked anyway.
				check_worktrees(&repo, &reference);
			}

			Victim::Known(KnownVictim::from(BString::from(args.from.clone()), reference)?)
		},
	};

	let target = TargetRev::from(&repo, BString::from(args.to))?;

    let reflog_msg = match victim {
        Victim::Known(ref victim_ref) => format!(
            "git-point: updating {} from {} to {}",
			victim_ref.name.as_bstr(),
            victim_ref.resolved_id,
            target.resolved_id,
        ),
		Victim::New(ref name) => format!(
			"git-point: created {} from {}",
			name.name.as_bstr(),
			target.resolved_id
		),
    };

	let transaction = RefEdit {
		change: Change::Update {
			log: LogChange {
				mode: RefLog::AndReference,
				force_create_reflog: false,
				message: BString::from(reflog_msg.clone()),
			},
			expected: match &victim {
				Victim::Known(victim_ref) => {
					PreviousValue::MustExistAndMatch(Target::Peeled(victim_ref.resolved_id.into()))
				},
				Victim::New(_new) => PreviousValue::MustNotExist,
			},
			new: Target::Peeled(target.resolved_id.detach()),
		},
		name: {
			FullName::try_from(victim.name_bstr()).unwrap()
		},
		deref: false,
	};

	if log::log_enabled!(log::Level::Trace) {
		match &victim {
			Victim::Known(known) => {
				trace!("mutating ref {}: {:?}", known.name.as_bstr(), &transaction);
			},
			Victim::New(new) => {
				trace!("creating ref {}: {:?}", new.name.as_bstr(), &transaction);
			},
		}
	}

	let _edits = repo.edit_reference(transaction.clone())
		.tap_err(|e| {
			match &victim {
				Victim::Known(_known) => error!(
					"while mutating ref {} to {}: {}",
					victim.name_bstr(),
					target.resolved_id,
					e,
				),
				Victim::New(_new) => error!(
					"while creating ref {} at {}: {}",
					victim.name_bstr(),
					target.resolved_id,
					e,
				),
			}
		})?;

	match &victim {
		Victim::Known(known) => eprintln!(
			"Updated \x1b[34m{refname}\x1b[0m from \x1b[33m{previd}\x1b[0m ({prevmsg}) to \x1b[33m{newid}\x1b[0m ({newmsg})",
			refname = known.name.as_bstr(),
			previd = known.resolved_id.shorten_or_id(),
			prevmsg = known.summary.as_bstr(),
			newid = target.resolved_id.shorten_or_id(),
			newmsg = target.summary.as_bstr(),
		),
		Victim::New(new) => eprintln!(
			"Created \x1b[34m{refname}\x1b[0m at \x1b[33m{target_id}\x1b[0m ({msg})",
			refname = new.name.as_bstr(),
			target_id = target.resolved_id.shorten_or_id(),
			msg = target.summary,
		)
	}

	Ok(())
}
