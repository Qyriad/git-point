# git-point — set arbitrary refs without shooting yourself in the foot, a porcelain `git update-ref`

```
Usage: git-point [OPTIONS] <FROM> <TO>

Arguments:
  <FROM>
          ref to update

  <TO>
          revision to point <FROM> to

Options:
  -n, --new <KIND>
          create a new ref of <KIND> instead of updating an existing one

          Possible values:
          - tag:           New lightweight tag in refs/tags/<FROM>
          - branch:        New branch refs/heads/<FROM>
          - remote-branch: refs/remotes/<FROM> (e.g. refs/remotes/origin/main)
          - raw:           No prefix, interpreted literally (like update-ref, be careful!)

  -W, --allow-worktree
          Allow mutating checked out refs. This will *not* change any of the actual files in the worktree

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

git-point is a single command to change what commit a [ref](https://git-scm.com/book/en/v2/Git-Internals-Git-References) points to — a [porcelain](https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain) alternative to `git update-ref`, which is [easy](https://stackoverflow.com/a/36008283/4231588) to misuse, makes no distinction between updating and creating refs, checked out refs versus not, and logs nothing.

git-point:
* always requires the `<FROM>` argument to resolve to exactly one unambiguous and existing ref, or for you to intentionally specify creation with `--new`
* always fully resolves the `<TO>` argument to exactly one unambiguous and existing commit
* allows both `<FROM>` and `<TO>` to be abbreviated (e.g., `v2.3` instead of `refs/tags/v2.3`)
* never modifies your worktree
* accepts the full syntax for revisions, so you can `git point v2.3 'HEAD^{/version bump: 2.3}'` to your heart's content
* logs the state before and after

## Installation and usage

### Nix

If you have a Nix implementation, you may use git-point without installing, for example with flakes:

```bash
$ nix run 'github:Qyriad/git-point' -- some-topic some-topic~3
# OR
$ nix shell 'github:Qyriad/git-point'
```

without flakes:

```bash
$ nix run --impure --expr 'import (fetchGit "https://github.com/Qyriad/git-point") { }' . -- some-topic some-topic~3
# OR
$ nix shell --impure 'import (fetchGit "https://github.com/Qyriad/git-point") { }'
[nix-shell]$ git-point some-topic some-topic~3
```

with `nix-shell`:

```bash
$ nix-shell --expr 'import (fetchGit "https://github.com/Qyriad/git-point") { }'
```

### Cargo

git-point is written in Rust, and thus may be installed with Cargo:

```bash
$ cargo install git-point
```

Though note that this will *not* install the man page for git-point.
You can generate it yourself with `git point --mangen`, which will output the man page to stdout.
If you wish to install it, you could do something like:

```bash
$ git point --mangen > /usr/local/share/man1/git-point.1
```
