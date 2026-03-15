# Development Process

## Overview

We use file-based issue tracking in `docs/tracker/`. Four agents handle the lifecycle: PM grooms, Engineer implements, Tester verifies, PM accepts.

Terminology:
- Issue = a file in `docs/tracker/` describing work to be done (bug fix, feature, etc.)
- Task = a Claude Code task panel item tracking pipeline steps within the current session

## Issue Lifecycle

```
PM grooms (.todo)  ->  Engineer builds (.in-progress)  ->  Tester verifies  ->  PM accepts (.done)
```

### File-Based Status

Issue status is encoded in the filename:

| Status | Filename Pattern | Meaning |
|--------|-----------------|---------|
| Todo | `01-name.todo.md` | Not started, needs PM grooming before pickup |
| Groomed | `01-name.groomed.md` | PM has groomed, ready for engineer |
| In Progress | `01-name.in-progress.md` | Engineer is working on it |
| Done | `01-name.done.md` | PM accepted, complete |

### Status Transitions

```
.todo.md  -->  PM grooms  -->  .groomed.md  -->  Engineer picks up  -->  .in-progress.md
                                                      |
                                              Engineer done + Tester pass + PM accept
                                                      |
                                                      v
                                                 .done.md
```

## Orchestrator Role

The orchestrator (top-level Claude Code session) is a MANAGER, not an implementer. It:

- Launches agents (PM, SWE, QA) and routes work between them
- Routes rejection feedback: when QA fails, send SWE back with the QA feedback; when PM rejects, send SWE back with the PM feedback
- Commits code ONLY after PM accepts
- Picks next issues from the backlog
- Creates task panel items to track pipeline progress

The orchestrator NEVER writes or modifies code (src/, tests/, scripts/). It only touches:
- docs/tracker/ files (creating issues, status transitions)
- Task panel items
- Git commits (after PM accepts)

**NEVER wait for user input. NEVER idle. The pipeline must always be running.**

- If something needs user action (configuring a secret, testing on their machine), note it in the issue file as "USER ACTION REQUIRED" and immediately move to the next issue
- After committing a batch, immediately pick the next 2 issues and start grooming/implementing
- After asking the user a question, don't wait for the answer — keep working on other issues
- If issue A is groomed but issue B is still grooming, launch SWE for A immediately
- If there are issues in the backlog, there is work to do — never end with "waiting for completion"
- The orchestrator's job is to keep agents busy at ALL times

## Agent Workflow

1. PM Grooms: Pick `.todo.md` issues, add acceptance criteria and test scenarios, rename to `.groomed.md`
2. Pick 2 issues: Select the lowest-numbered `.groomed.md` issues whose dependencies are met
3. SWE implements: Write code + tests, rename to `.in-progress.md`
4. QA reviews: Run tests, verify acceptance criteria, report PASS/FAIL
5. If QA FAIL: Launch SWE agent again with QA's feedback. SWE fixes. Then launch QA again. Repeat until QA passes.
6. If QA PASS: Launch PM for acceptance review
7. If PM rejects: Launch SWE agent again with PM's feedback. SWE fixes. Then QA re-verifies. Then PM re-reviews. Repeat until PM accepts.
8. If PM accepts: Orchestrator renames to `.done.md` and commits
9. Pick next 2 issues and repeat

### Done Means DONE

An issue moves to `.done.md` ONLY when ALL acceptance criteria are fully satisfied and verified. Writing code is not done. Passing tests is not done. The actual deliverable must be complete:

- "Publish to PyPI" is done when wheels are on PyPI and `uvx` works — not when the workflow YAML is written
- "Benchmark" is done when results have real numbers — not when the script exists
- "Visual comparison" is done when screenshots show real diffs — not when the test infrastructure is set up
- "CI fix" is done when CI is green — not when the workflow is committed

If the deliverable requires deployment, external verification, or running against real data, the issue stays `.in-progress.md` until that happens. The orchestrator must verify the actual outcome before moving to done.

**IMPORTANT: One agent per issue.** Every agent invocation handles exactly ONE issue. When working on 2 issues in a batch, launch 2 separate agents in parallel — never combine multiple issues into a single agent call. This applies to all agent types (SWE, QA, PM).

### Rejection Loop

```
QA FAIL  -->  SWE fixes (with QA feedback)  -->  QA re-verifies  -->  repeat until PASS
PM REJECT --> SWE fixes (with PM feedback)  -->  QA re-verifies  -->  PM re-reviews --> repeat until ACCEPT
```

The orchestrator's job in a rejection is to launch a new SWE agent with the rejection details, NOT to fix the code itself.

### Issue Log (Communication via Issue File)

Every agent MUST append log entries to the issue file as they work. The issue file is the single source of truth for what happened. This makes it possible to track work, debug problems, and review history.

Each agent appends a `## Log` section (or appends to it if it already exists) with timestamped entries:

```markdown
## Log

### [SWE] 2026-03-14 12:30
- Started implementation
- Root cause: slug sanitization missing in collection.rs:372
- Fixed: added trim() and replace(" ", "-") to slug generation
- Tests added: 5 unit tests for slug sanitization
- Build: 985 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/collection.rs, tests/integration_pages.rs

### [QA] 2026-03-14 13:15
- All tests pass (985 passed, 0 failed)
- Clippy clean, fmt clean
- Acceptance criteria 1-8: PASS
- Acceptance criterion 9: FAIL — sitemap still has spaces in 2 URLs
- VERDICT: FAIL

### [SWE] 2026-03-14 13:45
- Fixed sitemap URL generation to use sanitized slugs
- Tests: 987 pass, 0 fail
- Files modified: src/sitemap.rs

### [QA] 2026-03-14 14:00
- All criteria pass
- VERDICT: PASS

### [PM] 2026-03-14 14:30
- Reviewed diff, output verified
- VERDICT: ACCEPT
```

**What to log:**
- What was done (implementation steps, root causes found, fixes applied)
- Test results (pass count, fail count, specific failures)
- Files modified
- Build/lint results
- Acceptance criteria verdicts (per criterion)
- Rejection reasons (if rejecting)
- Any follow-up issues created

**Why:** Without logs, the orchestrator and user have no visibility into what happened. Agents are ephemeral — the issue file is permanent.

### No Silent Descoping

**PM must NEVER silently drop acceptance criteria.** If a requirement from the original issue is too large or out of scope for the current implementation:

1. PM must explicitly call out what is being descoped and why
2. PM must create a new `.todo.md` issue for each descoped requirement, OR assign it to an existing open issue
3. The descoped items must be traceable — the new issue should reference the original issue number

The orchestrator must verify that PM acceptance does not silently drop criteria from the groomed spec. If the PM accepts with unmet criteria and no follow-up issues, the orchestrator must reject the acceptance and require follow-up issues be created.

## Agents

| Agent | File | Role |
|-------|------|------|
| Product Manager | `.claude/agents/product-manager.md` | Grooms issues + final acceptance |
| Software Engineer | `.claude/agents/software-engineer.md` | Implements code + tests |
| Tester | `.claude/agents/tester.md` | Runs tests, verifies acceptance criteria |

## Technology Stack

- Language: Rust (latest stable)
- Build system: Cargo
- Testing: `./scripts/cargo-safe test`
- Linting: `./scripts/cargo-safe clippy`
- Formatting: `cargo fmt`

### Memory Safety

**Always use `./scripts/cargo-safe` instead of raw `cargo`** for build, test, and clippy commands. This wrapper runs cargo in an isolated cgroup with a memory limit (default 24G). If cargo hits the limit, only cargo dies — the shell/tmux/claude session survives and gets a non-zero exit code.

Plain `cargo fmt` is fine since it uses negligible memory.

## How to Pick Issues

1. List `.groomed.md` files in `docs/tracker/`
2. Pick the lowest-numbered issues first (lower = more foundational)
3. Check dependencies -- don't start until deps are `.done.md`
4. Pick 2 independent issues at a time for parallel implementation

## Output Verification (Critical)

For any issue that changes HTML output or templating:

1. Build the site and verify the generated HTML is correct
2. Check that links, images, and structured data are properly rendered
3. Compare output against the original Jekyll site where applicable
4. Verify RSS/Atom feeds are valid XML

### Common output issues to check:
- Broken links or missing images
- Malformed HTML (unclosed tags, wrong nesting)
- Missing or incorrect metadata (Open Graph, structured data)
- Incorrect URL generation or permalink structure
- Missing content from collections (posts, books, podcast, etc.)

## Task Panel (Claude Code Built-in Tasks)

The orchestrator MUST use the Claude Code task panel to track every step of the pipeline. Tasks are session-scoped progress trackers -- they are NOT the same as issues in `docs/tracker/`.

### How Task Panel Items Should Look

Each task panel item tracks a pipeline step for ONE issue. When working on 2 issues, create separate task items for each:

| Task Subject | Example |
|---|---|
| `[PM groom] issue #59` | PM grooming one issue |
| `[SWE] implement issue #59` | Engineering one issue |
| `[QA] verify issue #59` | Testing one issue |
| `[PM accept] issue #59` | Acceptance + commit one issue |
| `[Pull next] pick 2 issues from backlog` | Pick up more work |

### Setting Up a Batch

When starting work on a batch of 2 issues (#N, #M), create separate task panel items per issue:

1. `[PM groom] issue #N`
2. `[PM groom] issue #M`
3. `[SWE] implement issue #N`
4. `[SWE] implement issue #M`
5. `[QA] verify issue #N`
6. `[QA] verify issue #M`
7. `[PM accept] issue #N -> commit`
8. `[PM accept] issue #M -> commit`
9. `[Pull next] pick 2 issues from backlog`

Set up blockedBy dependencies: each issue's SWE is blocked by its PM groom, QA by its SWE, PM accept by its QA. The two issues' pipelines run in parallel. [Pull next] is blocked by both PM accept tasks.

Launch parallel agents: for example, spawn 2 SWE agents simultaneously (one per issue), then 2 QA agents, etc.

### Pipeline Per Batch (2 issues in parallel)

```
[PM groom #N] -> [SWE #N] -> [QA #N] -> [PM accept #N] --\
                                                           +--> [Pull next]
[PM groom #M] -> [SWE #M] -> [QA #M] -> [PM accept #M] --/
```

Within each issue pipeline, reject sends back to that issue's SWE, not to grooming.

### Task Panel Tags

| Panel Tag | Agent | When | What happens |
|-----------|-------|------|-------------|
| `[PM groom]` | Product Manager | BEFORE implementation | Adds acceptance criteria, test scenarios. Renames .todo -> .groomed. **One agent per issue.** |
| `[SWE]` | Software Engineer | After grooming | Implements code + tests. Renames .groomed -> .in-progress. **One agent per issue.** |
| `[QA]` | Tester | After implementation | Verifies acceptance criteria, builds site and checks output. Pass/Fail. **One agent per issue.** |
| `[PM accept]` | Product Manager | AFTER QA passes | Final review. Builds and inspects output. Accept -> .done + commit. Reject -> back to SWE to finish. **One agent per issue.** |
| `[Pull next]` | Orchestrator | AFTER commit | Check docs/tracker/ for remaining .todo/.groomed files. Pick 2 lowest-numbered, create new batch in task panel, repeat |

PM has two distinct roles:
1. Before engineering: groom the issue (define what "done" looks like)
2. After QA: accept or reject (verify it actually looks right). Reject sends it back to engineer for finishing, NOT back to grooming.

### Pull Next Work

The last item in every batch is always "[Pull next] pick 2 issues from backlog". This ensures work continues automatically:
1. Check `docs/tracker/` for `.todo.md` or `.groomed.md` files
2. Pick the 2 lowest-numbered groomed issues (groom first if only .todo.md)
3. Create a new batch of task panel items with dependencies
4. Start the pipeline again

## Conventions

- Every issue must include tests (`cargo test`)
- Lint with `cargo clippy -- -D warnings`
- Format with `cargo fmt`
- Commit message references issue: "Implement issue 01: project setup"
- Only commit after PM accepts
- Issues are NEVER deleted -- they move through statuses (.todo -> .groomed -> .in-progress -> .done)
- Commit regularly -- don't accumulate large uncommitted changes
