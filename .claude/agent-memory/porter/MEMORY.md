# porter: lessons across runs

One line per lesson, appended at the end; task state goes in HANDOFF.md
(`.claude/rules/subagents.md`, "Memory"). Committed with the tree.

- Worktree setup: the guard hook allows the reference symlink only as `ln -s <main>/reference/openbsd-src reference/openbsd-src` (link name relative, plain command); an absolute link name or a `test || ln` chain is refused.
- The worktree isolation check refuses `git -C reference/...`: take `upstream_blob` with `git hash-object reference/openbsd-src/<c path>` (same hash for an unmodified file).
- The isolation check also refuses long heredocs and multi-step shell edits: put a Python edit script in the scratchpad and run `python3 -I script.py <file>`, or use Edit.
- A new `sys_*` handler is wired by `just gen-syscalls` (it scans `pub fn sys_<name>(`); then `cargo fmt --all`, since the generator does not align comments; `just check-syscalls` must say up to date.
- C names with `___` (`sys___thrsleep`) need `#[allow(non_snake_case)] // the C name: ...` as in kern_exit/kern_sig.
- clippy `manual_clamp`: `MAX(1, MIN(x, M))` is `x.clamp(1, M)` (comment the C form above it).
- A machine header that is the same on both archs (`<machine/spinlock.h>`) is still per-arch `include/<h>.rs` + a `sys/machine/<h>.rs` trait with associated type/consts + a host impl + the assert in `sys/machine/mod.rs`.
- Host tests cannot sleep (`cpu_switchto` panics on host): factor a C post-`sleep_finish` block into a helper to test races (wakeup vs timeout) by feeding it the error.
- Own C test programs: `tools/<name>/{<name>.c,Makefile}` (`LDSTATIC= -static`) in `OWN_PROGRAMS` (userland.rs); the whole root goes into the ramdisk; a seeded worktree `cargo xtask userland --arch A` takes about a minute.
- Never run `cargo test -p xtask` or another `cargo xtask` while a smoke runs: cargo rewrites target/debug/xtask under it (EXT-225).
