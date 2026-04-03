# Issue 404: Bash shell variable syntax highlighting

## Problem

Bash code blocks on two DTC pages have ~170 combined diffs because environment
variable names and assignment operators are not highlighted with the correct
Rouge token classes.

Jekyll/Rouge output for `-e POSTGRES_USER="root"`:
```html
<span class="nt">-e</span> <span class="nv">POSTGRES_USER</span><span class="o">=</span><span class="s2">"root"</span>
```

Rustkyll output:
```html
<span class="nt">-e</span> POSTGRES_USER=<span class="s2">"root"</span>
```

Missing: `nv` (Name.Variable) for variable names, `o` (Operator) for `=`.

## Affected Pages

- `blog/how-to-run-postgresql-and-pgadmin-with-docker.html` (~90 of 133 diffs)
- `blog/ml-deployment-lambda.html` (~80 of 164 diffs)

## Scope

Fix the bash/shell syntax highlighting scope mapping in `src/syntax.rs` to:
1. Wrap environment variable names (`POSTGRES_USER`, `AWS_REGION`, etc.) with `nv` class
2. Wrap `=` assignment operators with `o` class
3. Handle both `-e VAR=val` (docker) and bare `VAR=val` patterns
4. Map `export` to `nb` (builtin) instead of `k` (keyword)

### Technical Analysis

**Root cause:** There are no bash-specific scope overrides in the `build_scope_map()` rules.
The generic rules map `variable.other` to `n` and `keyword.operator` to `o`, but syntect's
bash grammar likely does not scope bare `VAR_NAME` tokens as `variable.other` in assignment
context -- they may appear as unscoped text or under a different scope.

**Approach: scope mapping + post-processing in `src/syntax.rs`**

The fix likely requires a combination of:

1. **Scope-level overrides** (in `build_scope_map()` rules, added before the generic rules):
   - `source.shell variable.other` -> `nv` (same pattern as PHP override on line 55)
   - `source.shell keyword.other` -> `nb` (for `export`, `source`, etc.)
   - Any bash-specific `keyword.operator` scopes that syntect assigns to `=`

2. **Post-processing** (in the bash post-processing block around line 437):
   - If syntect does not assign scopes to `VAR_NAME=` in assignment context (leaving
     them as bare text), a post-processing pass must detect `UPPERCASE_VAR=` patterns
     and wrap the variable name in `<span class="nv">` and the `=` in `<span class="o">`
   - Pattern: bare text matching `[A-Z_][A-Z0-9_]*=` that is not already inside a span

**Important:** The engineer must dump actual syntect scopes for test inputs to determine
exactly which approach is needed. Do NOT guess -- inspect the scope output.

### Dependencies

None. This issue has no dependencies on other issues.

## DTC DOM Baseline

- Current: 788/790
- Must not drop below: 788/790
- Target: 790/790 (fixing both affected pages)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] `highlight_code("bash", "POSTGRES_USER=\"root\"\n")` output contains `<span class="nv">POSTGRES_USER</span><span class="o">=</span><span class="s2">"root"</span>`
- [ ] `highlight_code("bash", "docker run -e POSTGRES_USER=\"root\"\n")` wraps `POSTGRES_USER` in `nv` and `=` in `o`
- [ ] `highlight_code("bash", "export AWS_ACCOUNT=12345\n")` wraps `export` in `nb`, `AWS_ACCOUNT` in `nv`, and `=` in `o`
- [ ] `highlight_code("bash", "export AWS_DEFAULT_REGION=eu-central-1\n")` wraps `export` in `nb`, `AWS_DEFAULT_REGION` in `nv`, `=` in `o`, and `eu-central-1` is plain text or appropriately classed
- [ ] `highlight_code("bash", "LAMBDA=lambda_function\n")` wraps `LAMBDA` in `nv` and `=` in `o`
- [ ] Existing bash tests still pass: prompt lines (`$ cmd`), flags (`-b` as `nt`), strings (`"..."` as `s2`), comments (`#` as `c1`), `install` as `nb`, escaped backslashes as `se`, line continuations as `p`
- [ ] DTC DOM match count >= 788/790 (no regression)
- [ ] Tests include at least one non-ASCII/Unicode variable name scenario (e.g., `MY_VAR_caf\u{00e9}=val`)

## Test Scenarios

### Unit: Bash variable assignment highlighting
- Parse `POSTGRES_USER="root"` -- verify `nv` for variable name, `o` for `=`, `s2` for value
- Parse `export AWS_ACCOUNT=12345` -- verify `nb` for `export`, `nv` for variable, `o` for `=`
- Parse `export AWS_DEFAULT_REGION=eu-central-1` -- verify full token sequence
- Parse `docker run -e POSTGRES_USER="root"` -- verify `-e` is `nt`, variable is `nv`, `=` is `o`
- Parse `LAMBDA=lambda_function` -- verify bare assignment without export
- Parse `VAR="multi word value"` -- verify variable before quoted string
- Parse `FOO=bar BAZ=qux command` -- verify multiple inline env var assignments

### Unit: No regression on existing bash patterns
- Verify `git checkout -b dev` still produces `nt` for `-b`
- Verify `git commit -m "msg"` still produces `s2` for the string
- Verify `# comment` still produces `c1`
- Verify `pip install pre-commit` still produces `nb` for `install`
- Verify `$ promptfoo eval config.yaml` still produces `nv` for `$ ` prompt
- Verify `docker run -it \\` still produces `se` for escaped backslash
- Verify `curl ... \` still produces `p` for line continuation

### Unit: Unicode
- Parse bash code with non-ASCII characters in context -- verify no panics or mangled output

### Integration: DTC DOM comparison
- Build the full DTC site and run DOM comparison
- Verify DOM match count >= 788/790 (no regression)
- Verify `blog/how-to-run-postgresql-and-pgadmin-with-docker.html` diff count decreases
- Verify `blog/ml-deployment-lambda.html` diff count decreases

## Output Verification

After implementation, the engineer and tester must:

1. Build the DTC site: `./scripts/cargo-safe build` then run the binary against the DTC source
2. Inspect the generated HTML for `blog/how-to-run-postgresql-and-pgadmin-with-docker.html`:
   - Search for `POSTGRES_USER` -- must be wrapped in `<span class="nv">`
   - Search for `=` adjacent to variable names -- must be wrapped in `<span class="o">`
3. Inspect the generated HTML for `blog/ml-deployment-lambda.html`:
   - Search for `export` -- must be wrapped in `<span class="nb">`
   - Search for `AWS_ACCOUNT`, `AWS_DEFAULT_REGION` -- must be in `<span class="nv">`
4. Run DOM comparison and report the exact match count
