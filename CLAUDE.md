# rustkyll

Rust static site generator replacing Jekyll for the DataTalks.Club website.

## Development Process

See `docs/PROCESS.md` for the full agent-driven development workflow.

## Quick Reference

- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt`

## Project Structure

- `src/` -- Rust source code
- `docs/tracker/` -- Issue tracker (file-based)
- `docs/plan.md` -- Project vision and architecture
- `datatalksclub.github.io/` -- Original Jekyll site (reference)

## Agents

| Agent | File |
|-------|------|
| Product Manager | `.claude/agents/product-manager.md` |
| Software Engineer | `.claude/agents/software-engineer.md` |
| Tester | `.claude/agents/tester.md` |

## Conventions

- Idiomatic Rust: strong types, enums, Result/Option, no unwrap in library code
- Every issue must include tests
- Only commit after PM accepts
- Commit message: "Implement issue NN: short description"
