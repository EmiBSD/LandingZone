# Status

Milestone: **TDk met** (TD1a until 2026-11-15: this kernel built inside an arm64 EmiBSD VM
by DevTools' native cargo); **TD0 under way** (the kernel's debt: TD0..TD6); then M17.
Updated: 2026-11-15.

Toolchain: DevTools' Rust 1.99.1+emibsd.5 and LLVM 23.1.4+emibsd.6 (D-0008), run straight
from `~/devel/emibsd/toolchains` (no rustup; `just cargo`); the userland by DevTools' clang.

Done:
- 2026-11-15: TD restated by the user: this repository is the kernel; userland
  self-hosting is the Userland repository's (the in-VM userland build made here comes out
  in TD0). The debt measured: 138 `wip`, 42 `todo`, 198 `unported!`, `sys_nosys` calls.
- TD1a (2026-11-15): `smoke-selfhost-kernel` ok (32m21s in the VM). Fixed on the way: the
  "EmiBSD" ELF note (ENOEXEC), `__thrsleep`, shared libcrypto/libssl, the selfhost disk's base;
  also sysarch (TD3), the userland by DevTools' clang (TD2), the build tools in the base (TD1b),
  `--accel hvf` (the direct map's UXN under EPAN), pins +emibsd.5/+emibsd.6 without rustup.
- 2026-11-02: DevTools' per-arch pairs and the renamed kernel targets; amd64 back in `ci`.
- M16 (2026-10-09): QEMU drivers, M16a..M16g, with the install images.

Next:
- TD0: TDk's disk from Userland's base; the userland build for the VM out; a row for every
  kernel `.c` outside the drivers, checked by `ports check`.
- TD1 the `wip` rows; TD2 the `unported!` stubs; TD3 swapctl, SysV IPC, ptrace, ktrace;
  TD4 the other MI files; TD5 networking; TD6 the arch files. Drivers are not in TD.
- M17 (the user's decisions below first).
- Left by M14: GPL parts of the sets, lldb, efi(4).

Unsafe (`cargo xtask unsafe-report`): kernel 8992 blocks, 1143 fn, 822 impl, 31 trait, 340 other; tests 826 more.

Blockers:
- amd64 kernel stacks are tight: about 4.9 KB stay free under softraid I/O (M10f measure).
- A `diagnostic` MP kernel panics at boot (`uvm_page_physload: page size not set!`).
- Statistics counters the C bumps unlocked stay `Cell`s (docs/ARCHITECTURE.md, M11e).
- Flakes: amd64 serial cut at QEMU exit (M16f); host test `no idleproc set on CPU0` (~1/80);
  EDK2's UhciDxe ASSERT on arm64 (EXT-1; twice in a row on 2026-11-01 and 2026-11-15).
- main's target/comp holds paths of the pre-rename checkout: a seeded `just comp` fails.

Decisions pending (the user's): the scope section; vga(4)'s SeaBIOS path; Limine's retirement;
M17's PC CPU, Pi 4 model and networking; reporting EXTERNAL_BUGS upstream; the restated M16
criteria.
