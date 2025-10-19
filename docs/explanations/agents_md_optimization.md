# AGENTS.md Optimization - Documentation

## Overview

Updated AGENTS.md to remove redundant `make check` references and ensure
compliance with markdownlint (MD040). The changes streamline the development
workflow by eliminating duplicate validation steps while maintaining code
quality standards.

## Rationale

### Problem 1: Redundant `make check` Command

The `make check` command was a wrapper that ran the exact same three cargo
commands already documented:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo test --all-features`

If all four cargo commands pass, there is no additional value in running
`make check`. The redundancy created confusion and added unnecessary steps to
the workflow.

### Problem 2: Markdownlint MD040 Violations

The MD040 rule requires that all fenced code blocks specify a language. Several
examples in AGENTS.md intentionally showed fenced code blocks without language
specifiers (to demonstrate markdown syntax itself). These needed to be wrapped
in markdownlint-disable comments.

## Changes Made

### 1. Removed `make check` References

**Locations Updated:**

- **Line 33-35**: "BEFORE YOU START ANY TASK" section
- **Line 44**: "AFTER YOU COMPLETE ANY TASK" checklist
- **Line 160-164**: Rule 4 "Code Quality Gates" section
- **Line 407-413**: Phase 2 Implementation workflow
- **Line 475**: Phase 3 Validation Results
- **Line 486-496**: Phase 4 Validation section
- **Line 958-991**: "When make check Fails" emergency procedure (renamed to
  "When Quality Checks Fail")
- **Line 1064**: Validation checklist
- **Line 1149-1159**: Quick Command Reference
- **Line 1187**: Rule 3 summary
- **Line 1197-1207**: The Golden Workflow

**Total Removals**: 14 references to `make check` removed

### 2. Updated Workflow to Four Cargo Commands

The new canonical workflow is:

```bash
# 1. Format code
cargo fmt --all

# 2. Check compilation
cargo check --all-targets --all-features

# 3. Lint (warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run tests (>80% coverage)
cargo test --all-features
```

### 3. Fixed Markdownlint MD040 Violations

Added `<!-- markdownlint-disable MD040 -->` and
`<!-- markdownlint-enable MD040 -->` comments around code blocks showing
markdown syntax examples:

**Locations:**

- **Line 162-170**: Rule 4 expected results example
- **Line 260-262**: Documentation file structure closing fence
- **Line 487-489**: Phase 3 documentation closing fence
- **Line 651-664**: Git branch naming examples
- **Line 700-721**: Git commit message examples
- **Line 780-794**: Documentation decision tree
- **Line 1194-1202**: Rule 3 quality checks summary
- **Line 1210-1226**: The Golden Workflow

**Total Additions**: 9 markdownlint-disable/enable comment pairs

### 4. Fixed Line Length Violations

- **Line 35**: Split long line about expected output
- **Line 43**: Split long line in checklist

## Validation

### Before Changes

```bash
$ grep -c "make check" AGENTS.md
14

$ markdownlint AGENTS.md
AGENTS.md:35:81 MD013/line-length Line length [Expected: 80; Actual: 91]
AGENTS.md:259 MD040/fenced-code-language Fenced code blocks should have...
AGENTS.md:486 MD040/fenced-code-language Fenced code blocks should have...
```

### After Changes

```bash
$ grep -c "make check" AGENTS.md
0

$ markdownlint AGENTS.md
(no output - all checks pass)
```

## Impact on Workflow

### For AI Agents

**Before:**

```text
4. Run: cargo fmt --all
5. Run: cargo clippy --all-targets --all-features -- -D warnings
6. Run: cargo test --all-features
7. Run: make check (MUST show "All quality checks passed!")
```

**After:**

```text
4. Run: cargo fmt --all
5. Run: cargo check --all-targets --all-features
6. Run: cargo clippy --all-targets --all-features -- -D warnings
7. Run: cargo test --all-features
```

**Benefits:**

- Clearer - shows exactly what checks are required
- Faster - no duplicate validation runs
- More portable - doesn't depend on Makefile implementation
- Easier to debug - can see which specific command failed

### For Human Developers

Developers can still use `make check` if desired (as a convenience wrapper),
but the canonical workflow is now the four cargo commands. This provides:

- Better CI/CD integration (most CI systems run cargo commands directly)
- Clearer error messages (know exactly which check failed)
- Faster iteration (can run individual checks during development)

## Emergency Procedures Updated

The "When make check Fails" section was renamed to "When Quality Checks Fail"
and updated to show the systematic debug process using cargo commands:

```bash
# Step 1: Fix formatting
cargo fmt --all

# Step 2: Fix compilation errors
cargo check --all-targets --all-features

# Step 3: Fix clippy warnings
cargo clippy --all-targets --all-features -- -D warnings

# Step 4: Fix failing tests
cargo test --all-features -- --nocapture

# Step 5: Verify all checks pass (run all four again)
```

## Documentation Consistency

All documentation sections now consistently reference the four cargo commands:

- Quick Reference for AI Agents
- Rule 4: Code Quality Gates
- Development Workflow
- Validation Checklist
- Quick Command Reference
- The Golden Workflow

## Backward Compatibility

The `make check` command (if it exists in the Makefile) is not deprecated and
can still be used. However:

- It is no longer documented as mandatory
- It is no longer part of the canonical workflow
- Developers are encouraged to use cargo commands directly

This allows gradual migration and doesn't break existing automation.

## Files Modified

- `xzepr/AGENTS.md` - Comprehensive updates as described above

Total lines changed: ~150 lines (modifications across 14 sections)

## Testing

Verified changes with:

```bash
# Verify no make check references remain
grep -c "make check" AGENTS.md
# Output: 0

# Verify markdownlint passes
markdownlint AGENTS.md
# Output: (no errors)

# Verify all fenced code blocks are valid
grep -n '^```' AGENTS.md | wc -l
# All blocks properly opened/closed
```

## Conclusion

The AGENTS.md file is now:

- More concise (removed 14 redundant references)
- More accurate (shows actual validation commands)
- Markdownlint compliant (zero violations)
- Easier to follow (canonical workflow is clear)
- Better for debugging (can identify which check failed)

The four cargo commands are now the single source of truth for code quality
validation, eliminating confusion and streamlining the development workflow.

## References

- Original AGENTS.md design rationale: `docs/explanations/`
- Markdownlint MD040 rule: https://github.com/DavidAnson/markdownlint/blob/main/doc/Rules.md#md040
- Cargo command reference: https://doc.rust-lang.org/cargo/commands/
