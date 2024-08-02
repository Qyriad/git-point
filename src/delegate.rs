use bstr::BStr;
#[allow(unused)]
use log::{trace, debug, warn, info, error};

use gix::Repository;
use gix::revision::plumbing::spec::Kind as SpecKind;
use gix::revision::plumbing::spec::parse::{Delegate as RevParseDelegate, Error as RevParseError};
use gix::revision::plumbing::spec::parse::delegate::{
	Kind,
	Navigate,
	PeelTo,
	PrefixHint,
	ReflogLookup,
	Revision,
	SiblingBranch,
	Traversal,
};

use crate::{RepositoryExt, MaybeAmbigRef};

#[derive(Debug)]
/// Gix revision parsing delegate which stubs everything except what we need to
/// determine if the refs in a revision spec are ambiguous.
pub struct StubDisambDelegate<'repo>
{
	pub repo: &'repo Repository,
	pub kind: Option<SpecKind>,
	pub found_refs: Option<MaybeAmbigRef<'repo>>,
	pub error: Option<miette::Report>,
}

impl<'repo> StubDisambDelegate<'repo>
{
	pub fn new(repo: &'repo Repository) -> Self {
		Self {
			repo,
			kind: None,
			found_refs: None,
			error: None,
		}
	}

	pub fn parse(&mut self, revspec: &BStr) -> Result<(), RevParseError>
	{
		gix::revision::plumbing::spec::parse(revspec, self)
	}
}

impl<'repo> Kind for StubDisambDelegate<'repo>
{
	fn kind(&mut self, kind: gix::revision::plumbing::spec::Kind) -> Option<()>
	{
        debug!("Delegate::kind({:?})", kind);
		self.kind = Some(kind);
        Some(())
	}
}

impl<'repo> Navigate for StubDisambDelegate<'repo>
{
	fn traverse(&mut self, kind: Traversal) -> Option<()>
	{
        debug!("Delegate::traverse({:?})", kind);
		Some(())
	}

	fn peel_until(&mut self, kind: PeelTo) -> Option<()>
	{
        debug!("Delegate::peel_until({:?})", kind);
        Some(())
	}

	fn find(&mut self, regex: &BStr, negated: bool) -> Option<()>
	{
        debug!("Delegate::find({:?}, {:?})", regex, negated);
        Some(())
	}

	fn index_lookup(&mut self, path: &BStr, stage: u8) -> Option<()>
	{
        debug!("Delegate::index_lookup({:?}, {:?})", path, stage);
        Some(())
	}
}

impl<'repo> Revision for StubDisambDelegate<'repo>
{
	fn find_ref(&mut self, name: &BStr) -> Option<()>
	{
        debug!("Delegate::find_ref({:?})", name);

		let maybe_ambiguous_refs = match self.repo.find_ambiguous_references(name) {
			Ok(refs) => refs,
			Err(e) => {
				assert!(self.error.is_none());
				self.error = Some(e.wrap_err(format!("while looking for ref '{}'", name)));
				return None;
			},
		};

		self.found_refs = Some(maybe_ambiguous_refs);

		Some(())
	}

	fn disambiguate_prefix(&mut self, prefix: gix::hash::Prefix, hint: Option<PrefixHint<'_>>) -> Option<()>
	{
        debug!("Delegate::disambiguate_prefix({:?}, {:?})", prefix, hint);
        Some(())
	}

	fn reflog(&mut self, query: ReflogLookup) -> Option<()>
	{
        debug!("Delegate::reflog({:?})", query);
        Some(())
	}

	fn nth_checked_out_branch(&mut self, branch_no: usize) -> Option<()>
	{
        debug!("Delegate::nth_checked_out_branch({:?})", branch_no);
        Some(())
	}

	fn sibling_branch(&mut self, kind: SiblingBranch) -> Option<()>
	{
        debug!("Delegate::sibling_branch({:?})", kind);
        Some(())
	}
}

impl<'repo> RevParseDelegate for StubDisambDelegate<'repo>
{
	fn done(&mut self)
	{
		// We do nothing. Caller will do whatever needs to be done from here.
	}
}
