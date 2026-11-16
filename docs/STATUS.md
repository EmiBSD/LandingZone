# Status

Milestone: **TDk met** (TD1a until 2026-11-15: this kernel built inside an arm64 EmiBSD VM
by DevTools' native cargo); **TD0 under way** (the kernel's debt: TD0..TD6); then M17.
Updated: 2026-11-15.

Toolchain: DevTools' Rust 1.99.1+emibsd.5 and LLVM 23.1.4+emibsd.6 (D-0008), run straight
from `~/devel/emibsd/toolchains` (no rustup; `just cargo`); the userland by DevTools' clang.

Done:
- TD0 (2026-11-15, branch `td0`, not committed): TDk's disk is made of Userland 2026.11.4's
  `base80` and `comp80` and DevTools' native LLVM and Rust; `smoke-selfhost-kernel` ok on it
  (`lz-build-rc-0`, 30m31s in the VM, TCG). TD1b is out (`selfhost-userland`, the build tools
  in the base, `base.ffs`, `smoke-buildtools`). The inventory: 268 rows for the kernel `.c`
  files that had none, and `ports check` refuses such a file with no row (238 `todo`,
  138 `wip`, 109 `skipped`, 1174 `ported`).
- TDk (TD1a, 2026-11-15): the kernel built inside the VM; the "EmiBSD" ELF note,
  `__thrsleep`, sysarch, the direct map's UXN (hvf), pins +emibsd.5/+emibsd.6 without rustup.
- 2026-11-02: DevTools' per-arch pairs and the renamed kernel targets; amd64 back in `ci`.

Next:
- TD0's close: `just ci` green (blocked, below).
- TD1 the `wip` rows; TD2 the `unported!` stubs; TD3 swapctl, SysV IPC, ptrace, ktrace;
  TD4 the other MI files; TD5 networking; TD6 the arch files. Drivers are not in TD.
- M17 (the user's decisions below first).

Unsafe (`cargo xtask unsafe-report`): kernel 8992 blocks, 1143 fn, 822 impl, 31 trait, 340 other; tests 826 more.

Blockers:
- `just ci` cannot be green in this checkout: `target/userland/arm64` is the build of
  2026-10-09 (no `thrtest`, so `smoke-thrsleep` fails on arm64), and building a userland
  here is forbidden since 2026-11-15. The user's decision.
- DevTools' native rustc and cargo die of SIGILL on a CPU with BTI (EXT-228), so
  `smoke-selfhost-kernel` runs under TCG, not `--accel hvf`.
- amd64 kernel stacks are tight: about 4.9 KB stay free under softraid I/O (M10f measure).
- A `diagnostic` MP kernel panics at boot (`uvm_page_physload: page size not set!`).
- Statistics counters the C bumps unlocked stay `Cell`s (docs/ARCHITECTURE.md, M11e).
- Flakes: amd64 serial cut at QEMU exit (M16f); host test `no idleproc set on CPU0` (~1/80);
  EDK2's UhciDxe ASSERT on arm64 (EXT-1).

Decisions pending (the user's): the scope section; vga(4)'s SeaBIOS path; Limine's retirement;
M17's PC CPU, Pi 4 model and networking; reporting EXTERNAL_BUGS upstream; the restated M16
criteria.
