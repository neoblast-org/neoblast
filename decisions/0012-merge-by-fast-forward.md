# 0012: Changes reach `main` by fast-forward only

- Status: accepted
- Date: 2026-11-10
- Settles: review finding S-007

## Context and problem statement

GitHub's merge button makes new commits on GitHub's side, committed and
signed by GitHub, whichever of its methods is used. A squash or a merge
commit also names a GitHub account, the pull request's or the merger's, as
author, and carries a time-zone offset instead of UTC. The project's own
commits are by `neoblast maintainers`, in UTC and signed with a key in
`.github/allowed_signers`, but with the button they are not the ones that
reach `main`. The offset can tell where a maintainer lives, which matters
for a censorship-resistance project. The merges up to #8 were made this way
(S-007).

#9 and #10 were re-made on `main` as project commits (`36cb370`,
`5393f67`), and since #11 every change has reached `main` as a
fast-forward of the project's own commits, pushed by the maintainer; but
that was a practice, not a written rule.

## Decision outcome

**Every change reaches `main` as a fast-forward of the project's own
signed, UTC commits, pushed by the maintainer. The merge button is never
used.**

- A pull request is merged only once CI has passed and GitHub shows every
  new commit as verified, with:

  ```sh
  git fetch origin <branch>
  git push origin FETCH_HEAD:main
  ```

  GitHub then marks the pull request merged, as its commits are on `main`.
- A branch that is behind `main` is rebased by the maintainer, signed
  again and pushed, never updated with GitHub's "Update branch" button,
  which makes the same kind of commit.
- A change from outside the project, Dependabot's included, is re-made as
  a project commit and lands the same way (`f2380e7`, `35a6193`,
  `6c2e947`). The pull request keeps the record of who proposed it, and its
  commits carry the author's DCO sign-off; the hygiene job fails on them by
  design, as only the project may author or sign off a commit on `main`.

The push goes through `protect-main`'s bypass for organisation admins, as
its pull-request rule would refuse it otherwise. The bypass also skips that
ruleset's six required checks, so the maintainer checks CI and verification
before pushing (S-023).

What enforces it today:

- `hygiene-required` requires the hygiene job to have passed on the commit
  pushed to `main`, and nobody can bypass it (S-014). The job fails on any
  commit after `5393f67` that is not by the project, in UTC (S-008), and on
  any from `f2380e7` on that is not signed with a key in
  `.github/allowed_signers` (S-026), so the push itself is refused unless
  the commits are the project's.
- `protect-main` requires linear history and signed commits, and allows the
  button only to squash: a merge commit or a rebase from the button is
  refused.

What does not: a squash from the button still reaches `main`, signed by
GitHub. Its required checks are those of the pull request's head, which
are the project's commits and pass; the hygiene job then fails on the
squash when it runs on `main`, after it has landed. A ruleset must allow
one merge method, as GitHub must, so the button can be narrowed, as it is,
but not turned off.

### Consequences

- Good: every commit on `main` since #9 is the project's and in UTC, and
  the rule keeps it so; `git log` on `main` is the commits that were
  reviewed, not GitHub's copies of them.
- Good: the commits a release tags are the commits that were reviewed and
  signed, with no copy made on GitHub's side in between.
- Bad: one click on "Squash and merge" undoes it, and is caught only
  after the fact. Rewriting `main` to remove such a commit would break
  every clone; copies may already exist.
- Bad: merging needs a terminal, and a pull request from outside takes a
  maintainer's re-made commit.
- Neutral: the merges up to #8 keep GitHub's metadata; rewriting them
  would need a force-push to `main` (S-007).

## Considered options

- **Squash merges, with GitHub's metadata accepted.** One click, but the
  merger's account and time zone reach public history.
- **Set the merging account's time zone to UTC.** Removes the offset only;
  the commit is still GitHub's, signed by GitHub, by the merging account.
- **The button, limited to one method by a ruleset.** `protect-main`
  already allows only squash, but every method makes a GitHub commit, so
  the button is kept only because one method must be.
