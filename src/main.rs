use std::env;
use std::error::Error as StdError;
use std::path::PathBuf;

use bstr::{BString, ByteSlice, ByteVec};
use clap::{Parser, ValueEnum, Arg, ArgAction};

use gix::prelude::*;
use gix::refs::transaction::Change;
use gix::refs::transaction::LogChange;
use gix::refs::transaction::PreviousValue;
use gix::refs::transaction::RefEdit;
use gix::refs::transaction::RefLog;
use gix::refs::Target;
use gix::Object;
use gix::ObjectId;
use gix::Repository;

#[allow(unused)]
use log::{trace, debug, warn, info, error};

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
	from: String,

	/// revision to point <FROM> to
	to: String,

	/// create a new ref of <KIND> instead of updating an existing one
	#[arg(short, long, action = ArgAction::Set, value_name = "KIND")]
	new: Option<NewRefKind>,
}

//fn resolve_target<'r>(repo: &'r Repository, args: &GitPointCmd) -> Result<Object<'r>, Box<dyn StdError>>
//{
//	let to = args.to.as_str();
//
//	let target_id = repo.rev_parse_single(to)
//		.map_err(|e| {
//			error!("error parsing revision {}", to);
//			e
//		})?;
//
//	let target_object = target_id
//		.object()
//		.unwrap();
//
//	let target_commit = target_object
//		.clone()
//		.into_commit();
//	let msg = target_commit
//		.message_raw()
//		.unwrap()
//		.lines()
//		.next()
//		.unwrap();
//
//	eprintln!("Ref \x1b[34m{}\x1b[0m resolved to \x1b[33m{}\x1b[0m ({})", to, target_id.shorten_or_id(), msg.as_bstr());
//
//	Ok(target_object)
//}

fn main()
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

	let current_id = ref_to_update.clone().peel_to_id_in_place().unwrap();
	let current_commit = repo
		.find_object(current_id).unwrap()
		.into_commit();
	let current_commit_summary = current_commit
		.message_raw().unwrap()
		.lines().next().unwrap();

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
		current_id,
		target_obj.id,
	);

	let transaction = RefEdit {
		change: Change::Update {
			log: LogChange {
				mode: RefLog::AndReference,
				force_create_reflog: false,
				message: BString::from(reflog_msg),
			},
			expected: PreviousValue::MustExistAndMatch(Target::Peeled(current_id.into())),
			new: Target::Peeled(target_obj.id),
		},
		name: ref_to_update.name().to_owned(),
		deref: false,
	};

	let _edits = repo.edit_reference(transaction.clone()).unwrap();

	eprintln!(
		"Updated \x1b[34m{refname}\x1b[0m from \x1b[33m{previd}\x1b[0m ({prevmsg}) to \x1b[33m{newid}\x1b[0m ({newmsg})",
		refname = ref_to_update.name().shorten(),
		previd = current_id.shorten_or_id(),
		prevmsg = current_commit_summary.as_bstr(),
		newid = target_id.shorten_or_id(),
		newmsg = target_commit_summary.as_bstr(),
	);
}
