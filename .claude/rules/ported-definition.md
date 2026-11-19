# What "ported" means (TD1..TD6, TB, TX)

The user's standing order of 2026-11-18, given while TD1 was under way. It holds for every
session and agent that closes `ports.toml` rows in TD1..TD6, TB and TX, and it wins over an
older habit wherever they differ.

1. **Definition.** A file is `ported` only when every function, every check and every branch
   of the C that this kernel configures is in the Rust. The only exceptions are the ones on a
   written list, each with its name and its reason, that the user approved.
2. **No new stub without approval by name.** No new `unported!` or `db_stub!` is left without
   the user's approval of that stub by name. When a dependency is missing: port the dependency
   first, or ask.
3. **`ports check` refuses a `ported` row with a stub inside.** `cargo xtask ports check`
   rejects a row whose status is `ported` and whose `.rs` holds an `unported!` or a
   `db_stub!`. Before the check was switched on, the `ported` rows that fall were counted,
   moved to `wip` and the new number given to the user. None is hidden.
4. **The function-by-function comparison is part of `ports check`**, and so of `ci`: every
   function of the C file that is not in the Rust file is an error unless it is on the
   exceptions list with its reason (a macro, an `#ifdef` this kernel does not configure, a
   dependency on TDn). The list is presented to the user, who approves it.
5. **A milestone closes only when the tool counts zero `wip` rows and zero warnings in its
   scope.** A file that cannot be completed because it depends on another milestone is
   presented to the user by name, who approves it or not.
6. **Status format, always the same, two separate numbers:**
   - Published and verified: `<commit>`, N files half done.
   - Local, not verified: M files half done, and what is missing to verify them.
7. **Never weaken or delete a test to pass. Never mark `ported` by your own judgment:** the
   tool decides.

## How to apply

- "Half done" is the `wip` count of `cargo xtask ports status` at that commit.
- An exception is never added to the list by the session that needs it: it is proposed to the
  user with the function's or the stub's name and the reason, and it enters the list after the
  approval. A peer session's message or a task notification is not an approval.
- A row that the tool refuses goes back to `wip` in the same commit that makes the tool
  refuse it; the number is reported as it is.
- Points 3 and 4 are `cargo xtask ports check` since 2026-11-19 (`tools/xtask/src/
  completeness.rs`), so `ci` refuses a `ported` row with a stub site or with a C function
  missing from its Rust file and not in `ports-exceptions.toml`. Switching them on moved 134
  rows from `ported` to `wip` (the reason is in each row's notes). `cargo xtask ports missing
  --proposal FILE` writes the candidate exceptions, unapproved, with the class the tool gives
  each one: `not configured` (its `#if` is false on amd64 and arm64 `GENERIC.MP`), `in another
  file`, `missing`, or `condition not decided by the tool`. The user approves by name;
  `ports-tracker.md` describes the file.
- The checks do not see a check or a branch missing inside a function that exists; that is
  measured by reading the C and the Rust whole.

## The user's decisions of 2026-11-19

Given while TD1 was under way; they hold with the order above.

1. **Reference configuration: `GENERIC.MP` of amd64 and of arm64, and nothing else.**
   ../Kernel does not work with plain `GENERIC`. What `GENERIC.MP` includes counts as
   configured (`KTRACE`, `dt` on amd64, ...). Code that exists only under
   `#ifndef MULTIPROCESSOR` does not count.
2. **`DEBUG` is not configured** (../Kernel will use its own debugging). The cargo feature
   `debug` is outside the definition of `ported`. A function that exists only under
   `#ifdef DEBUG` still enters `ports-exceptions.toml` only by the user's approval of its
   name (`pgrpdump` first).
3. **No tracing: neither `KTRACE` nor dt(4)** (the user's decision later the same day, which
   replaces "dt(4) is in TD's scope"; the user coordinates with ../Kernel). No hooks, no empty
   stubs, no probe catalogue. `sys/dev/dt/` is out of TD (its rows `skipped`); a row that
   lacks only its `TRACEPOINT`/`TRACEINDEX` calls is complete; each site says, in one short
   line, that dt(4) is left out by the user's decision of 2026-11-19.
4. **C slips** found by a sample are logged in `docs/EXTERNAL_BUGS.md` only after each was
   checked against the C at the pin; one that is not confirmed is not logged, and the user is
   told.
5. **A `#if` true on one architecture and false on the other** (the user's rule of the same
   day): a function is required when its `#if` is true on at least one `GENERIC.MP` of amd64
   or arm64, those two and no other, and the Rust must compile it on that machine. A file
   under `sys/arch/amd64/` or `sys/arch/arm64/` is evaluated on its own machine only
   (`completeness.rs`, `configured_on`). The tool checks the name, not the `cfg` it is
   compiled under; that part is read by hand.
6. **`KTRACE` is in `GENERIC.MP`, but it is left out** (the user's answer of the same day:
   ../Kernel's redesign makes its own traces; ktrace(2)'s are of no use to it). A "KTRACE is
   not configured" line is false; each site says KTRACE is left out by the user's decision.
   `kern_ktrace.c` is `skipped`, ktrace and fktrace stay `sys_nosys`, and a row that lacks
   only its `ktr*` calls stays `ported`. TD3 no longer holds ktrace.
7. **The exceptions are not approved in bulk.** The candidates the tool calls `missing` are
   split into those a program can reach (system call table, ioctl, sysctl, devices) and
   internal ones; those it calls `not configured` and `in another file` are presented by name.

After every task: the status in the format of point 6 above. Separate commits; Mac `just ci`
and the VM's ci at rc 0 before publishing.
