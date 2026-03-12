

# GitHub CLI Commands Required for Orbit Agents

This document lists the minimal GitHub CLI (`gh`) command surface required for Orbit agents to implement the **Task → Branch → PR → Review → Merge** workflow.

Agents should not be allowed to execute arbitrary `gh` commands. Only the commands below should be exposed through Orbit tools.

---

# Authentication

Ensure the CLI is authenticated before performing any operation.

```
gh auth status
```

Purpose:
- Verify GitHub authentication
- Fail early if credentials are missing

---

# Repository Discovery

Retrieve repository metadata.

```
gh repo view --json name,defaultBranchRef
```

Purpose:
- Detect default branch (`main`, `master`, etc.)
- Confirm repository access

---

# Create Pull Request

Used by worker agents after implementing a task.

```
gh pr create \
  --title "<title>" \
  --body-file <file> \
  --base main \
  --head <branch> \
  --label orbit
```

Example:

```
gh pr create \
  --title "T-104 Refactor scheduler state machine" \
  --body-file .orbit/pr-template.md \
  --base main \
  --head orbit/T-104-refactor-scheduler \
  --label orbit
```

---

# List Pull Requests

Used by leader agents to discover open work.

```
gh pr list \
  --label orbit \
  --state open \
  --json number,title,headRefName,author
```

Purpose:
- Find PRs created by Orbit agents
- Poll for work requiring review

---

# View Pull Request

Retrieve full PR metadata.

```
gh pr view <pr> \
  --json number,title,body,headRefName,files,commits
```

Purpose:
- Inspect changes
- Parse PR metadata
- Identify modified files

---

# Checkout Pull Request

Used by review agents to run tests locally.

```
gh pr checkout <pr>
```

Equivalent to fetching the PR branch.

---

# Comment on Pull Request

Agents communicate results or feedback.

```
gh pr comment <pr> --body "<message>"
```

Example:

```
gh pr comment 212 --body "❌ clippy failed"
```

---

# Review Pull Request

Approve or request changes.

Approve:

```
gh pr review <pr> --approve
```

Request changes:

```
gh pr review <pr> --request-changes --body "fix lint errors"
```

---

# Merge Pull Request

Final step once review passes.

Recommended strategy:

```
gh pr merge <pr> --squash --delete-branch
```

Purpose:
- Maintain clean commit history
- Automatically remove feature branch

---

# Close Pull Request

Used if a PR must be rejected.

```
gh pr close <pr>
```

---

# Check CI Status

Used by leader agents to verify pipeline results.

```
gh pr checks <pr>
```

Structured output:

```
gh pr checks <pr> --json state,name
```

---

# Minimal Command Set

The following commands represent the minimal required set:

```
gh.auth.status
gh.repo.view

gh.pr.create
gh.pr.list
gh.pr.view
gh.pr.checkout

gh.pr.comment
gh.pr.review
gh.pr.merge
gh.pr.close
```

Optional but useful:

```
gh.pr.edit
gh.pr.checks
```

---

# Recommended Orbit Tool Names

Instead of exposing raw `gh`, Orbit should wrap these operations as internal tools.

Example tool names:

```
github.pr.create
github.pr.list
github.pr.view
github.pr.review
github.pr.merge
github.pr.comment
github.pr.checkout
github.pr.close
github.pr.checks
```

This abstraction allows Orbit to switch to GitHub API or other git providers without changing agent workflows.