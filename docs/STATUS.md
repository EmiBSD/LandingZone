# Status

Milestone: **TDk met** (TD1a until 2026-11-15: this kernel built inside an arm64 EmiBSD VM
by DevTools' native cargo, and since 2026-11-16 `ci` run inside it, its second criterion);
**TD0's criteria seen** on 2026-11-16 (the kernel's debt: TD0..TD6; `ci-full` not run,
below); then TD1 and on, then M17.
Updated: 2026-11-16.

Toolchain: DevTools' Rust 1.99.1+emibsd.6 and LLVM 23.1.4+emibsd.7 (D-0008), run straight
from `~/devel/emibsd/toolchains` (no rustup; `just cargo`); the userland by DevTools' clang.

Done:
- TD0 (2026-11-16, branch `td0`): TDk's disk is Userland 2026.11.6's `base80` and `comp80`
  alone (its `comp` carries DevTools' native LLVM and Rust); `smoke-selfhost-kernel` ok on it
  under `--accel hvf` (`lz-build-rc-0`, cargo 1m52s, the smoke 122.8 s: the native Rust has
  BTI since 1.99.1+emibsd.6, EXT-228 fixed). TD1b is out (`selfhost-userland`, the build
  tools in the base, `base.ffs`, `smoke-buildtools`). The inventory: 268 rows for the kernel
  `.c` files that had none, and `ports check` refuses such a file with no row (238 `todo`,
  138 `wip`, 109 `skipped`, 1174 `ported`). The version-control id lines (`$OpenBSD$`, ...)
  are out of every file and `ports check` refuses one (the user's decision of 2026-11-15).
  `just ci` green on that tree: rc 0 in 1090 s, 85 of 85 smokes in 13m14s, after the one
  `xtask userland --arch arm64` the user allowed for the smokes' ramdisk (it had no `thrtest`).
- `ci` inside EmiBSD (2026-11-16, `just smoke-selfhost-ci`; TDk's criterion too, the user's
  decision of that day): fmt, the arm64 and host clippy
  lines, the tests (the kernel's 2461 host tests on the kernel under test) and the kernel
  and init builds, by the native toolchain in the VM: 14 steps, `lz-ci-rc-0`, the smoke ok
  in 550.5 s. The smokes, amd64 and what reads `reference/` stay on the Mac. Two fixes it
  brought: `__guard_local` exported only from the kernel, and pckbc's tests start the
  timeout wheel (a host test that aborted now and then).
- TDk (TD1a, 2026-11-15): the kernel built inside the VM; the "EmiBSD" ELF note,
  `__thrsleep`, sysarch, the direct map's UXN (hvf), pins +emibsd.5/+emibsd.6 without rustup.
- 2026-11-02: DevTools' per-arch pairs and the renamed kernel targets; amd64 back in `ci`.

Next:
- The user's word on `ci-full` for TD0 (its three criteria are seen; the series is committed
  on `td0`, not pushed).
- TD1 the `wip` rows; TD2 the `unported!` stubs; TD3 swapctl, SysV IPC, ptrace, ktrace;
  TD4 the other MI files; TD5 networking; TD6 the arch files. Drivers are not in TD.
- M17 (the user's decisions below first).

Unsafe (`cargo xtask unsafe-report`): kernel 8992 blocks, 1143 fn, 822 impl, 31 trait, 340 other; tests 826 more.

Blockers:
- `just ci-full` cannot run in this checkout: its installer smokes need `just comp`, and
  building a userland or a comp set here is forbidden since 2026-11-15 (the smokes' ramdisk
  is the exception the user allowed once for arm64, on 2026-11-16). amd64's ramdisk is the
  build of 2026-11-15, with the 61 build tools of the old TD1b still in it: harmless to the
  smokes, but not what this tree's `userland` makes.
- amd64 kernel stacks are tight: about 4.9 KB stay free under softraid I/O (M10f measure).
- A `diagnostic` MP kernel panics at boot (`uvm_page_physload: page size not set!`).
- Statistics counters the C bumps unlocked stay `Cell`s (docs/ARCHITECTURE.md, M11e).
- Flakes: amd64 serial cut at QEMU exit (M16f); host test `no idleproc set on CPU0` (~1/80);
  EDK2's UhciDxe ASSERT on arm64 (EXT-1).

Decisions pending (the user's): the scope section; vga(4)'s SeaBIOS path; Limine's retirement;
M17's PC CPU, Pi 4 model and networking; reporting EXTERNAL_BUGS upstream; the restated M16
criteria.
