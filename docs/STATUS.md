# Status

Milestone: **TDk met** (TD1a until 2026-11-15: this kernel built inside an arm64 EmiBSD VM
by DevTools' native cargo, and since 2026-11-16 `ci` run inside it, its second criterion);
**TD0 met** on 2026-11-16 (the kernel's debt: TD0..TD6; a sub-milestone, closed with
`just ci`); **TD1 under way**, then TD2..TD6, then TB (the boot debt) and TX (the kernel
under its own checks), then M17.
Updated: 2026-11-18.

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
  `just ci` green at the close: rc 0 in 1055 s, 85 of 85 smokes in 13m10s, after one
  `xtask userland` run per arch, each allowed by the user, for the smokes' ramdisks (arm64's
  had no `thrtest`; amd64's still held the old TD1b tools and was rebuilt from a clean root).
- `ci` inside EmiBSD (2026-11-16, `just smoke-selfhost-ci`; TDk's criterion too, the user's
  decision of that day): fmt, the arm64 and host clippy
  lines, the tests (the kernel's 2461 host tests on the kernel under test) and the kernel
  and init builds, by the native toolchain in the VM: 14 steps, `lz-ci-rc-0`, the smoke ok
  in 550.5 s. The smokes, amd64 and what reads `reference/` stay on the Mac. Two fixes it
  brought: `__guard_local` exported only from the kernel, and pckbc's tests start the
  timeout wheel (a host test that aborted now and then).
- TDk (TD1a, 2026-11-15): the kernel built inside the VM; the "EmiBSD" ELF note,
  `__thrsleep`, sysarch, the direct map's UXN (hvf), pins +emibsd.5/+emibsd.6 without rustup.
- TD1 so far (2026-11-18): 53 of the 138 `wip` rows closed, 85 left (`ports status`: 237
  `todo`, 85 `wip`, 109 `skipped`, 1229 `ported`). Headers read whole beside their ports
  and closed or completed (armreg.h, specialreg.h, exec_elf.h, smr.h's lists, the device id
  tables by `cargo xtask gen-devs`, the sysctl name tables of socket.h, if.h and in.h,
  time.h, route.h, uvm_pager.h, uvm_pmap.h, malloc.h, syscall_mi.h), and files finished:
  profil(2) (subr_prof.c, kern_clock.c, gmon.h), subr_poison.c with kern_malloc.c,
  subr_pool.c (its ddb printers and pool_chk), kern_clockintr.c (its sysctl and printers),
  kern_resource.c, kern_exec.c, both vm_machdep.c. Checks the C makes and the port only
  reported are real: RLIMIT_NPROC in fork, MAP_STACK at every system call, RLIMIT_DATA in
  mprotect, the null device on a set-id exec's closed descriptors; arm64 has per-process
  pointer authentication keys and the C's `pmap_vp_pool` with `pmap_populate`. The tracker
  itself was repaired (20 rows carried another row's notes, 18 another file's blob, since
  2026-10-04) and `ports check` now compares blobs. Seven slips of the C logged on the way
  (EXT-229, EXT-231 to EXT-236), one boot fault fixed (SVE, EXT-230, `smoke-sve`).

Next:
- TD1 the `wip` rows (85 left: the uvm, kern and arch files by cluster); TD2 the `unported!` stubs; TD3 swapctl, SysV IPC, ptrace, ktrace;
  TD4 the other MI files; TD5 networking; TD6 the arch files. Drivers are not in TD.
- TB, the boot debt (the user's milestone of 2026-11-16, after TD6): the BIOS boot programs
  of amd64, booting from softraid in efiboot, rdboot; 44 rows (docs/ROADMAP.md). Limine is
  retired there: the ported loaders boot the kernel (the user's decision of 2026-11-16).
- TX, the kernel under its own checks (the user's milestone of 2026-11-16, after TB): a
  `diagnostic` kernel in every smoke; the stacks' guard page, a high-water mark and a
  softraid stress VM over every discipline; the counters two CPUs write as atomics. It takes
  the three blockers below marked TX.
- M17 (the user's decisions below first).

Unsafe (`cargo xtask unsafe-report`): kernel 8992 blocks, 1143 fn, 822 impl, 31 trait, 340 other; tests 826 more.

Blockers:
- `just ci-full` was not run for TD0 (a sub-milestone); it is owed once, at the close of TD
  and TB together (the user's word of 2026-11-16). Its
  installer smokes pack this repository's own `target/userland` and `target/comp` (comp is
  the build of 2026-10-09), which nobody may rebuild here without the user's OK.
- amd64 kernel stacks are tight: about 4.9 KB stay free under softraid I/O (M10f measure). TX.
- A `diagnostic` MP kernel panics at boot (`uvm_page_physload: page size not set!`). TX.
- Statistics counters the C bumps unlocked stay `Cell`s (docs/ARCHITECTURE.md, M11e). TX.
- Flakes: amd64 serial cut at QEMU exit (M16f); host test `no idleproc set on CPU0` (~1/80);
  EDK2's UhciDxe ASSERT on arm64 (EXT-1); `smoke-thrsleep` on arm64, "thrsleep timeout
  slept": a `__thrsleep` with a 50 ms deadline returned before it by the monotonic clock in
  2 runs of 10 (2026-11-18, not looked into yet).
- Feature `diagnostic` is built by no `ci` line (it compiles and lints clean on the host,
  arm64 and amd64 as of 2026-11-18, checked by hand). TX.

Decisions pending (the user's): the scope section; vga(4)'s SeaBIOS path (taken up in TB);
M17's PC CPU, Pi 4 model and networking; reporting EXTERNAL_BUGS upstream; the restated M16
criteria.
