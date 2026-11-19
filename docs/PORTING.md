# Porting process

How a C file becomes a Rust file. The rules Claude follows are in
`.claude/rules/porting-workflow.md`; this is the longer human version plus the status table.

## 1. Pick a file

`cargo xtask ports next` lists `todo` entries whose dependencies are `ported` or `skipped`.
If the file you want is not listed, add it to `ports.toml` as `todo` with its `deps` first.
`docs/ROADMAP.md` says which files belong to the current milestone.

## 2. Read

Read the `.c` completely. Then its header(s). Then the `(9)` man pages it mentions.
Before coding, write down:

- the call graph: what it calls, what calls it;
- locking and SPL assumptions (`splhigh`, `mtx_enter`, `KERNEL_LOCK`, "called at IPL_x");
- every error path and what it returns;
- every `#ifdef` and the `option(4)` behind it;
- data structures it owns versus borrows.

## 3. Header template

Every ported file starts like this (`sys/lib/libkern/strlcpy.rs`). The licence block is copied
verbatim from the C file (its `$OpenBSD$` id line is not: `Upstream:` and `ports.toml`'s
`upstream_blob` name the revision); the licence block sits between the
`/* <LICENSES> */` and `/* </LICENSES> */` marker lines. Everything else is inside
`/* <CODE> */` ... `/* </CODE> */`, and the tests, inline, in `/* <TESTS> */` ... `/* </TESTS> */`
at the end (only in files that have tests). `cargo xtask ports check` validates the markers, their
order, that `<LICENSES>` opens with the author's ISC block (before the original notice, or alone)
and `<TESTS>` is only where there is a `mod tests`. A file whose C has several notices keeps all
of them inside one pair of markers, after the author's block and one blank line; a C file with
no licence text gets `license = "none"` on its `ports.toml` entry and the author's block alone. To read a
ported file, read its code zone (`sed -n '/<CODE>/,/<\/CODE>/p' <file>`).

```rust
/* <LICENSES> */
/*
 * Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * ...
 */

/*
 * Copyright (c) 1998, 2015 Todd C. Miller <millert@openbsd.org>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * ...
 */
/* </LICENSES> */

/* <CODE> */
//! `strlcpy(3)`: size-bounded string copy.
//!
//! Upstream: sys/lib/libkern/strlcpy.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - Operates on byte slices: the destination size is `dst.len()`, not a separate argument, and
//!   `src` ends at its first NUL or at `src.len()`, whichever comes first.

use crate::strnlen;

// ... the port ...
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    // ... the tests ...
}
/* </TESTS> */
```

## 4. Write

Idiomatic Rust that preserves semantics. `docs/C_TO_RUST.md` is the idiom table; follow it or add
a row. Keep OpenBSD names. Put types in the header's module, functions in the file's module.
When something the file needs is not ported yet, stub it visibly
(`.claude/rules/scope-and-stubs.md`), never silently.

## 5. Test

- Pure logic: an inline `#[cfg(test)] mod tests` in the `<TESTS>` zone of the same file (never
  a `tests.rs`), runs with `just test`.
- Constants mirrored from C headers: `#[ignore]` reference-backed test, runs with `just test-ref`.
- Boot, console, trap behaviour: smoke expectation in `tools/xtask`, runs with `just smoke`.

## 6. Record

In `ports.toml`: `status = "ported"`, `upstream_commit` = `[meta].pinned`,
`upstream_blob` = `git -C reference/openbsd-src rev-parse HEAD:<c path>`, `notes` for stubs.
Then `cargo xtask ports check` and `cargo xtask ports status --write`.
`ports check` compares the blob of every row ported at the pin with the reference tree's, so a
row never carries another file's blob; change one row at a time, found by its `c =` line, never
by its blob or its notes.

`ported` is the tool's decision, not the porter's (the user's standing order of 2026-11-18,
`.claude/rules/ported-definition.md`): `ports check` refuses a `ported` row whose Rust file
holds an `unported!` or a `db_stub!`, and a `.c` row whose C file defines a function the Rust
file has no `fn` of, unless `ports-exceptions.toml` holds that function with the date the user
approved it. `cargo xtask ports missing` lists what is missing, with the `#if` conditions around
each function; `--proposal FILE` writes candidate exceptions for the user, never approved. The
checks do not see a check or a branch missing inside a function that exists: reading the file
whole against the C stays the porter's job.

## 7. Commit

`<scope>: port <file>.c (<what>)` with one `Upstream:` trailer per C file.
See `.claude/rules/git-commits.md`.

## Skipping

`status = "skipped"` with `notes` starting with one of `replaced-by-limine`, `provided-by-core`,
`deferred-driver`, `license: <which>`, `not-applicable`, followed by a short reason.
Skipped is a recorded decision, not "later".

## Bumping the reference pin

Deliberate, never automatic:

```sh
git -C reference/openbsd-src fetch --depth 1 origin master
git -C reference/openbsd-src checkout --detach FETCH_HEAD
cargo xtask ports drift --diff      # what changed upstream among ported files
# triage each DRIFT line: re-port, or add a note explaining why the change does not apply
# update reference/PINNED.md (Commit:, Date:) and ports.toml [meta].pinned
git commit -m "reference: bump OpenBSD pin to <12-hex>"
```

## Who ports: the agents

Since 2026-10-09 the porting is done by Claude Code subagents with fixed roles, defined in
`.claude/agents/` and sharing one contract, `.claude/rules/subagents.md` (setup in a worktree,
the machine lock, the long-run watcher, when to stop, what to report). Before that, every launch
carried the same rules in a hand-written prompt; 196 of them had been written when the roles
were extracted from them.

| Agent | Model | Does |
|---|---|---|
| `milestone-coordinator` | opus | a (sub-)milestone: probe OpenBSD 8.0, split, launch, integrate, close CI and docs |
| `porter` | opus | a delicate port: uvm, pmap, traps, MP, drivers with DMA or interrupts, any `.c` over 3,000 lines |
| `porter-mechanical` | sonnet | headers of constants and structs, small leaf functions, tables, `ports.toml` rows |
| `integrator` | opus | merge agent branches, resolve the shared tables (`ports.toml`, `ioconf.rs`, the justfile), `just ci` |
| `debugger` | opus | the root cause of a failing or flaky smoke or test, fixed without weakening an expectation |
| `reviewer` | opus | a read-only review of a branch: faithfulness to the C, `unsafe`, rules, security |
| `external-bugs` | opus | verify the OpenBSD C slips the ports met, for `docs/EXTERNAL_BUGS.md` |
| `openbsd-probe` | sonnet | boot the real OpenBSD 8.0 on a QEMU setup and report what it does |
| `image-worker` | sonnet | view, crop, resize images, so the coordinator's context stays small |

Six of them keep a memory, `.claude/agent-memory/<agent>/MEMORY.md`: one line per lesson that
outlives a run (an idiom that took two attempts, a known flake, a conflict pattern), committed
with the tree so the whole team of agents learns. An agent in a worktree writes it on its branch;
the `integrator` keeps both sides on merge. Task state lives in a `HANDOFF.md`, never in memory.
The `reviewer` has no memory on purpose: every change gets fresh eyes.

How a batch is split, in what order, with how many agents, when a branch is reviewed and when
the result is integrated is a `milestone-coordinator`'s (or the main session's) decision, taken
case by case with the roles above. It is not a script: a deterministic workflow over the same
roles (`/port`, 2026-10-09) was tried and failed, and was removed on 2026-10-10. The result of a
batch is still a branch; `main` moves only by hand, after the user's OK.

## Measuring progress

`/progress [milestone]` (`.claude/skills/progress/`) prints one row per ROADMAP milestone,
computed when asked from `ports.toml`, `docs/ROADMAP.md`, the reference tree and git: files
ported and left, C lines ported and left (`wc -l` at the pin), Rust lines, and the time the rest
would take at the project's own average of C lines per active day. Its conventions:

- a `wip` row counts whole as left; a `skipped` row counts nowhere;
- a parent milestone (M9, M10, M11, M16) shows its own rows plus its lettered children's;
- "unclaimed" is C the ROADMAP scope names in backticks with no `ports.toml` row yet, resolved
  in the reference tree: an estimate from prose, marked so;
- the time column is a linear extrapolation of the past; it promises nothing.

The numbers in README, STATUS and JOURNAL do not come from it: they come from
`cargo xtask ports status` and the git commands `docs/JOURNAL.md` names.

## Measuring unsafe

`cargo xtask unsafe-report` (M12+) counts the `unsafe` keywords of the kernel crates (`sys/`,
with `sys/lib/libkern` and `sys/lib/libz`, and `init/`; not `tools/xtask`). It is the Phase 2
baseline ([PHASE2.md](PHASE2.md)); `--write` puts the totals on the `Unsafe` line of
`docs/STATUS.md`.

- What counts: `unsafe { }` blocks, `unsafe fn` declarations (also `unsafe extern "C" fn`),
  `unsafe impl`, `unsafe trait`. "Other" is every remaining `unsafe`: `unsafe extern` blocks,
  `#[unsafe(no_mangle)]` attributes and `unsafe fn(..)` pointer types.
- What does not: an `unsafe` inside a comment, a string or a raw string (a small lexer skips
  them), and `r#unsafe`.
- Test code is a column of its own: the item after a test-only `#[cfg]` (the inline
  `mod tests` of the `<TESTS>` zone), files declared by a test-only `mod` (`testutil.rs`) (`test`, `all(.., test)`; `any(test, feature = "x")`
  also builds into a kernel and counts as kernel code). Moving the tests inline (M15) changed
  no total: only the "tests files" column shrank, to the files a test-only `mod` declares.
- Subsystems: the first directory under `sys/` (`kern`, `uvm`, `net`, `netinet`, `ufs` with
  ffs/mfs/ext2fs, `isofs`, ...); `arch/<a>` and `lib/<l>` by two; `dev/<d>` for the bus and
  chip directories that hold a `mod.rs` (`pci`, `pv`, `ic`, `isa`, `fdt`, `ofw`, `efi`), and
  `dev` for the rest of `sys/dev` (softraid, vnd, rd, bio, cons, rnd); `sys/*.rs` is
  `(crate root)`. `arch/host` is the `cargo test` double, counted as code.

## Status

<!-- ports:begin -->
_Generated by `cargo xtask ports status --write` against pin 3ce1f3f79392._

| Subsystem | todo | wip | ported | skipped | total |
|---|---:|---:|---:|---:|---:|
| arch/amd64 | 57 | 35 | 79 | 1 | 172 |
| arch/arm64 | 20 | 26 | 67 | 28 | 141 |
| conf | 0 | 1 | 1 | 2 | 4 |
| crypto | 4 | 5 | 44 | 0 | 53 |
| ddb | 11 | 6 | 9 | 0 | 26 |
| dev | 30 | 49 | 364 | 7 | 450 |
| isofs | 0 | 0 | 18 | 0 | 18 |
| kern | 18 | 24 | 63 | 2 | 107 |
| lib/libkern | 5 | 1 | 11 | 46 | 63 |
| lib/libsa | 18 | 6 | 44 | 11 | 79 |
| lib/libz | 0 | 7 | 13 | 0 | 20 |
| miscfs | 1 | 0 | 10 | 0 | 11 |
| msdosfs | 0 | 0 | 12 | 0 | 12 |
| net | 33 | 13 | 49 | 0 | 95 |
| net80211 | 18 | 0 | 0 | 0 | 18 |
| netinet | 4 | 3 | 55 | 0 | 62 |
| netinet6 | 1 | 0 | 30 | 0 | 31 |
| netmpls | 5 | 0 | 0 | 0 | 5 |
| nfs | 0 | 2 | 23 | 1 | 26 |
| ntfs | 0 | 1 | 12 | 0 | 13 |
| scsi | 10 | 2 | 10 | 0 | 22 |
| stand | 0 | 1 | 20 | 8 | 29 |
| sys | 0 | 4 | 105 | 2 | 111 |
| tmpfs | 0 | 1 | 7 | 0 | 8 |
| ufs | 0 | 3 | 41 | 1 | 45 |
| uvm | 2 | 24 | 13 | 0 | 39 |
| **total** | 237 | 214 | 1100 | 109 | 1660 |
<!-- ports:end -->
