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
