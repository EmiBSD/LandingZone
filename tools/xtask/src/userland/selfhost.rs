/* <LICENSES> */
/*
 * Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */
/* </LICENSES> */

/* <CODE> */
//! The self-hosting disk (TD1a): what `just smoke-selfhost-kernel` boots to build this
//! repository's kernel inside EmiBSD with DevTools' native toolchain.
//!
//! `cargo xtask selfhost --arch A --llvm TAR --llvm-sha256 HEX --rust TAR --rust-sha256 HEX`
//! makes `target/selfhost/<arch>/selfhost.ffs`, an ffs image of:
//! - the base: a clone of `userland`'s root (`/bin/ksh` and the tools `build.sh` runs) with
//!   `comp`'s staging root (libc, crt, ld.so, the headers) laid over it, its libc++ headers
//!   and clang resource directory removed (DevTools' LLVM replaces them as a whole), and the
//!   shared libraries of `userland`'s root laid over again, so libcrypto and libssl are this
//!   tree's (native cargo loads them);
//! - DevTools' native LLVM, then Rust (`aarch64-emibsd-emibsd8.0`; D-0009, D-0010): each
//!   tarball checked against its pin, holding one top directory, its `usr/` laid over the
//!   base unmodified;
//! - `/src/lz`: this repository's `HEAD` (`git archive`), with every crate of `Cargo.lock`
//!   vendored (`cargo vendor --locked`, offline: the crates come from the host's cargo cache,
//!   never the network), `/src/cg/config.toml` pointing cargo at them, and `/src/build.sh`,
//!   the build the smoke runs in a chroot;
//! - room to build: the image is the tree times 5/4 plus [`BUILD_ROOM`].
//!
//! The image is made afresh on every run, by the same makefs as `comp.ffs`.
//!
//! `cargo xtask selfhost-userland --arch A --llvm TAR --llvm-sha256 HEX` (TD1b) makes
//! `target/selfhost-userland/<arch>/userland.ffs`, what `just smoke-selfhost-userland` boots
//! to build the base system's own programs and libraries with the base's make(1), the pin's
//! `share/mk` and DevTools' native clang, as `make build` does:
//! - the same base and DevTools' native LLVM (no Rust);
//! - `/etc`: the ramdisk's, with OpenBSD's stock `master.passwd`, `group` and arm64
//!   `login.conf` (`etc/` of the pin) and their databases, since `make obj` run as root
//!   makes the object directories as the `build` user of the `wobj` group (`bsd.obj.mk`);
//! - [`MK_CONF`] as `/etc/mk.conf`: the compilers keep their own triple,
//!   `aarch64-emibsd-emibsd8.0`, which predefines `__EmiBSD__` (the user's decision; no
//!   `--target`), and read [`CLANG_CFG`], what the base system's clang does by default and
//!   DevTools' does not;
//! - `/usr/src`: a copy of the pin's [`USR_SRC`] directories in which every `__OpenBSD__`
//!   is renamed `__EmiBSD__`, as the Userland repository's `world.py` does in its copy
//!   (the user's decision of 2026-11-04: the userland is compiled with `__EmiBSD__` alone);
//!   the reference tree is only read. `sys/` goes without the drm driver's sources (its
//!   headers stay), half a gigabyte no userland Makefile reads;
//! - `/usr/obj` (`build:wobj`, 0770, as OpenBSD's mtree makes it), `/td1b/build.sh` (the
//!   steps the smoke runs, [`UL_BUILD_SH`]), and [`UL_BUILD_ROOM`] of free space.

use super::*;

/// Free space for the kernel's build inside the VM: one debug kernel with its dependencies
/// takes about 2 GiB of `target/` with `CARGO_INCREMENTAL=0`; the rest is margin.
const BUILD_ROOM: u64 = 8 << 30;

/// What the guest runs, in the chroot, as `/src/build.sh`: cargo's lines go to the console as
/// they come (so the run's log grows and a hang shows) and to `/src/build.log`. Every line it
/// prints stays under arm64's 128-byte console limit; its markers are printed, never typed,
/// so the smoke's expectations cannot be met by an echoed command line. `JOBS` is cargo's
/// `-j` (the recipe passes it).
const BUILD_SH: &str = r#"#!/bin/ksh
# TD1a: builds LandingZone's arm64 kernel with DevTools' native toolchain, in the chroot.
export HOME=/tmp CARGO_HOME=/src/cg CARGO_INCREMENTAL=0
export CARGO_TARGET_AARCH64_EMIBSD_NONE_SOFTFLOAT_LINKER=/usr/bin/ld.lld
export EMIBSD_BUILD=0 SOURCE_DATE_EPOCH=0 EMIBSD_BUILD_HOST=emibsd
# rustc on the kernel crate outgrows the default data limit, DFLDSIZ (512 MiB, as OpenBSD's):
# raise the soft limit to the hard one, MAXDSIZ, as a user would (`ulimit -d`).
ulimit -d $(ulimit -H -d)
cd /src/lz || exit 9
set -o pipefail
# Each line cut to 110 columns with ksh's own `typeset -L` (the base may have no cut(1)).
short() { typeset -L110 l; while IFS= read -r l; do print -r -- "$l"; done; }
s=$(date +%s)
cargo build --offline -p bsd --target aarch64-emibsd-none-softfloat \
    --features qemu,multiprocessor -j ${JOBS:-4} 2>&1 | tee /src/build.log | short
rc=$?
e=$(date +%s)
echo "lz-build-secs-$((e - s))"
k=target/aarch64-emibsd-none-softfloat/debug/bsd
n=$(llvm-readelf -h $k 2>/dev/null | grep -cE 'AArch64|EXEC')
echo "lz-elf-$n"
echo "lz-build-rc-$rc"
"#;

/// The cargo configuration of the chroot: the vendored crates, no progress bars on a serial
/// console.
const CARGO_CONFIG: &str = r#"[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "/src/lz/vendor"

[term]
progress.when = "never"
color = "never"
"#;

/// One DevTools tarball for [`selfhost`]: its path and the sha256 it is pinned to.
pub(crate) struct Pinned<'a> {
    pub(crate) tarball: &'a Path,
    pub(crate) sha256: &'a str,
}

/// Free space for TD1b's builds inside the VM: the object trees of libc (static, profiled
/// and shared, with `-g`) and of `bin`, `sbin`, `usr.bin` and `libexec`.
const UL_BUILD_ROOM: u64 = 6 << 30;

/// `/etc/mk.conf` of the TD1b disk, read by `bsd.own.mk` after `sys.mk`'s defaults. The
/// compilers are `sys.mk`'s own (`cc`, `c++`, `cpp`: DevTools' native clang) with their
/// default triple, `aarch64-emibsd-emibsd8.0`: it predefines `__EmiBSD__`, links with the
/// system's start files, `/usr/libexec/ld.so` and compiler-rt's builtins, gives lld the
/// base system's link defaults (`--undefined-version`, `--ignore-function-address-equality`)
/// and makes executables PIE, all in its driver. The sources test `__EmiBSD__` because
/// `/usr/src` is the renamed copy (module docs). Every compiler, `HOSTCC` too, reads
/// [`CLANG_CFG`]. `STACK_PROTECTOR` (read by ld.so's Makefile alone) builds ld.so's
/// stack-protector stubs: the base system's clang guards aarch64's returns with retguard
/// and emits no stack protector, DevTools' clang protects the stack by default (strong,
/// `__guard_local` and `__stack_smash_handler`), so ld.so takes the branch its Makefile
/// has for the architectures that use the protector, as `world.py` and the cross build
/// (`shlib.rs`) do; without it ld.so's link fails on `__stack_smash_handler`.
const MK_CONF: &str = "\
# /etc/mk.conf of EmiBSD's TD1b build disk (cargo xtask selfhost-userland).
# The compilers keep their own triple, aarch64-emibsd-emibsd8.0 (__EmiBSD__);
# /usr/src is the pin's with __OpenBSD__ renamed __EmiBSD__, as Userland's
# world.py does. /etc/clang-defaults.cfg holds what the base system's clang
# does by default and DevTools' does not.
EMIBSD_CFG=\t--config=/etc/clang-defaults.cfg
CC=\t\tcc ${EMIBSD_CFG}
CXX=\t\tc++ ${EMIBSD_CFG}
CPP=\t\tcpp ${EMIBSD_CFG}
HOSTCC=\t\t${CC}
# DevTools' clang emits the stack protector where the base system's relies on
# retguard: ld.so then needs its own stubs, which its Makefile builds when
# STACK_PROTECTOR is set.
STACK_PROTECTOR=
";

/// `/etc/clang-defaults.cfg`, a clang configuration file (`--config`, its options before
/// the command line's): what the base system's clang does by default and DevTools' does
/// not, the same compile defaults as `world.py`'s `COMPILE_DEFAULTS` where the build needs
/// them (each kept because the build inside EmiBSD failed without it).
///
/// - `-Wno-pointer-sign`: the pin's clang makes `-Wpointer-sign` `DefaultIgnore`
///   (`gnu/llvm/clang/include/clang/Basic/DiagnosticSemaKinds.td`, the diagnostic
///   `ext_typecheck_convert_incompatible_pointer_sign`); upstream's, and so DevTools', is
///   on, and the libraries that build with `-Werror` (librthread, libssl, libtls) then
///   fail on `char *` passed as `uint8_t *`. The cross build turns it off where `-Werror`
///   is (`userland.rs`, `WERROR_DEFAULTS`).
/// - `-Wno-error=unused-but-set-global`: LLVM 23's clang warns about a static global that
///   is set and never read (`-Wunused-but-set-global`, in `-Wunused`); the pin's clang has
///   no such diagnostic (its `Sema::DiagnoseUnusedButSetDecl`,
///   `gnu/llvm/clang/lib/Sema/SemaDecl.cpp`, sees function-local variables only), so
///   `usr.bin/ssh`, built with `-Wunused -Werror`, fails on `sshd-auth.c`'s `saved_argv`
///   and `have_agent` (lines 122 and 126 at the pin). A plain `-Wno-unused-but-set-global`
///   here would not do: the Makefile's later `-Wunused` turns the group on again, while
///   `-Wno-error=` survives a later `-Werror`. The warning is still printed; locals keep
///   `-Wunused-but-set-variable` as an error.
const CLANG_CFG: &str = "\
# What the base system's clang does by default and DevTools' clang does not
# (cargo xtask selfhost-userland). It ignores -Wpointer-sign by default
# (DiagnosticSemaKinds.td); it does not diagnose set-but-unread static
# globals, which LLVM 23 does (-Wno-error=, which a Makefile's later
# -Wunused -Werror does not undo).
-Wno-pointer-sign
-Wno-error=unused-but-set-global
";

/// The pin's directories copied to `/usr/src` (relative to the reference tree): what
/// `make build` reads for `include`, `lib`, `bin`, `sbin`, `usr.bin` and `libexec`
/// (`share/mk` too, though the base installs it, with `share/Makefile.inc` for its
/// `BINDIR`; `etc` for the stock files above;
/// `usr.sbin` for the sources usr.bin's at(1), crontab(1) and ypwhich(1) share with
/// cron(8) and ypserv(8)).
const USR_SRC: &[&str] = &[
    "Makefile",
    "share/Makefile.inc",
    "share/mk",
    "include",
    "lib",
    "bin",
    "sbin",
    "usr.bin",
    "libexec",
    "sys",
    "etc",
    "usr.sbin",
];

/// What the guest runs in the chroot as `/td1b/build.sh STEP`, STEP one of `ls` (the
/// criterion: `bin/ls` by its own Makefile, then the program it made runs), `build`
/// (`/usr/src/Makefile`'s `do-build` for TD1b's directories: share/mk installed, `make
/// includes`, then `lib`, `bin`, `sbin`, `usr.bin` and `libexec` built as the `build`
/// user and installed), or a directory of `/usr/src` (`bin`, `lib/libc`, ...): `make
/// obj`, then `make` in each of its `SUBDIR`s as the `build` user (what `bsd.subdir.mk`
/// does, one at a time so that every failure is listed), then `make` at the top. Each build's output goes to `/td1b/log/`; the tail of
/// a failed one to the console. Every line stays under arm64's 128-byte console limit and
/// every marker is printed, never typed. `JOBS` is make's `-j`.
const UL_BUILD_SH: &str = r#"#!/bin/ksh
# TD1b: OpenBSD's programs built by the base's make(1) and share/mk, in the chroot.
export PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME=/root
ulimit -d $(ulimit -H -d)
umask 022
set -o pipefail
short() { typeset -L110 l; while IFS= read -r l; do print -r -- "$l"; done; }
J=${JOBS:-4}
step=$1
tag=$(echo $step | tr / -)
L=/td1b/log
mkdir -p $L
s=$(date +%s)
# The hierarchy OpenBSD's install makes (etc/Makefile's distribution target): su(1)
# unveils /var/run/dev.db, whose directory must exist.
mtree -qdef /usr/src/etc/mtree/4.4BSD.dist -p / -U >$L/mtree.log 2>&1
echo "ul-mtree-rc-$?"
# The run-time link editor's hints, which /etc/rc makes at boot (`ldconfig -R` rescans
# them).
ldconfig /usr/lib >$L/ldconfig.log 2>&1
echo "ul-hints-rc-$?"
# Each SUBDIR of the tree $1 by itself, so that every failure is listed: make as
# the build user (bsd.subdir.mk's loop), then, with $2 = install, make install as root,
# as `make build` does; NOMAN=1 for libraries, as do-build.
tree() {
	typeset d=$1 t=$(echo $1 | tr / -) n=0 f=0 sd
	cd /usr/src/$d || return 9
	make obj >$L/$t-obj.log 2>&1
	echo "ul-$t-obj-rc-$?"
	for sd in $(make -V SUBDIR); do
		n=$((n + 1))
		(cd $sd && su $B -c "exec make -j$J" && if [ "$2" = install ]; then
			$NM make install; fi) >$L/$t-$sd.log 2>&1
		if [ $? -eq 0 ]; then
			echo "ul-ok-$d/$sd"
		else
			f=$((f + 1))
			echo "ul-FAIL-$d/$sd"
			tail -n 15 $L/$t-$sd.log | short
		fi
	done
	echo "ul-$t-subdirs-$((n - f))-of-$n"
	return $f
}
# One command $2... with its output in $L/$1.log: its rc marker, and its tail if it fails.
step1() {
	typeset n=$1
	shift
	"$@" >$L/$n.log 2>&1
	typeset r=$?
	echo "ul-$n-rc-$r"
	[ $r -eq 0 ] || tail -n 15 $L/$n.log | short
}
B=build
case $step in
build)
	# /usr/src/Makefile's do-build for TD1b's directories: share/mk, the headers
	# (`make includes`), the libraries (built, installed, ldconfig -R), then bin,
	# sbin, usr.bin and libexec, each built and installed.
	# `make obj` first, as release(8) runs it before `make build`. `make includes`
	# runs whatever `make prereq` says, so that one missing generator (libcrypto's
	# perl) does not hide what follows; both are reported.
	step1 mk-install eval 'cd /usr/src/share/mk && make install'
	step1 obj eval 'for o in include lib; do (cd /usr/src/$o && make obj); done'
	step1 prereq eval "cd /usr/src/include && su $B -c 'exec make prereq'"
	step1 includes eval 'cd /usr/src/include && make includes'
	NM="env NOMAN=1"
	tree lib install
	rc=$?
	NM=
	/sbin/ldconfig -R
	echo "ul-ldconfig-rc-$?"
	for d in bin sbin usr.bin libexec; do
		tree $d install
		rc=$((rc + $?))
	done
	;;
ls)
	cd /usr/src/bin/ls || exit 9
	{ make obj && make -j$J; } 2>&1 | tee $L/ls.log | short
	rc=$?
	echo "ul-ls-out-$(/usr/obj/bin/ls/ls -d /usr/share/mk/sys.mk)"
	./obj/ls -l / >/dev/null
	echo "ul-ls-run-rc-$?"
	;;
*)
	tree $step
	(cd /usr/src/$step && su $B -c "exec make -j$J") 2>&1 | tee $L/$tag.log |
	    tail -n 15 | short
	rc=$?
	;;
esac
e=$(date +%s)
echo "ul-$tag-secs-$((e - s))"
echo "ul-$tag-rc-$rc"
"#;

/// The base of every self-hosting disk, made afresh in `out/root`: a clone of `userland`'s
/// root, `comp`'s staging root laid over it without its libc++ headers and clang resource
/// directory, and `userland`'s shared libraries laid over again (module docs). Returns the
/// root and comp's ownership table.
fn base_root(root: &Path, arch: Arch, out: &Path) -> Result<(PathBuf, PathBuf)> {
    let comp_root = root.join("target/comp").join(arch.name()).join("root");
    let comp_owners = root
        .join("target/comp")
        .join(arch.name())
        .join("owners.txt");
    if !comp_root.join("usr/lib").is_dir() || !comp_owners.is_file() {
        return Err(format!("{}: missing; run `just comp`", comp_root.display()).into());
    }
    let user_lib = root
        .join("target/userland")
        .join(arch.name())
        .join("root/usr/lib");
    if !makefs_of(root, arch).is_file() || !user_lib.is_dir() {
        return Err(format!(
            "{}: missing; run `just userland`",
            makefs_of(root, arch).display()
        )
        .into());
    }
    let rootdir = out.join("root");
    for d in [&out.join("unpack"), &rootdir] {
        if d.exists() {
            fs::remove_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
    }
    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    // The base system (/bin/ksh and the tools build.sh runs), an APFS clone of userland's
    // root (seconds, no space until written), with comp's root (libc, crt, ld.so, the
    // headers) laid over it.
    let user_root = root.join("target/userland").join(arch.name()).join("root");
    run(Command::new("cp").arg("-Rc").arg(&user_root).arg(&rootdir))?;
    run(Command::new("cp")
        .arg("-R")
        .arg(comp_root.join("."))
        .arg(&rootdir))?;
    for stale in ["usr/include/c++", "usr/lib/clang"] {
        let d = rootdir.join(stale);
        if d.exists() {
            fs::remove_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
    }
    // The base system's shared libraries as `userland` last built them (libcrypto, libssl).
    let mut libs = 0;
    for e in fs::read_dir(&user_lib).map_err(|e| format!("{}: {e}", user_lib.display()))? {
        let p = e?.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with("lib") && name.contains(".so.") {
            let to = rootdir.join("usr/lib").join(name);
            let _ = fs::remove_file(&to);
            fs::copy(&p, &to).map_err(|e| format!("{}: {e}", to.display()))?;
            libs += 1;
        }
    }
    println!(
        "  base: {} ({libs} shared libraries from userland)",
        comp_root.display()
    );
    let owners = out.join("owners.txt");
    fs::copy(&comp_owners, &owners).map_err(|e| format!("{}: {e}", owners.display()))?;
    Ok((rootdir, owners))
}

/// `userland`'s makefs(8), built for the Mac.
fn makefs_of(root: &Path, arch: Arch) -> PathBuf {
    root.join("target/userland")
        .join(arch.name())
        .join("host/bin/makefs")
}

/// Lays the DevTools tarball `pin` (checked against its pin; one top directory holding
/// `usr/`) over `rootdir/usr`, unpacking it in `out/unpack/<what>`.
fn lay_tarball(out: &Path, rootdir: &Path, what: &str, pin: &Pinned<'_>) -> Result<()> {
    check_sha256(pin.tarball, pin.sha256)?;
    let dir = out.join("unpack").join(what);
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    run(Command::new("sh").arg("-c").arg(format!(
        "zstd -dcq '{}' | tar -xf - -C '{}'",
        pin.tarball.display(),
        dir.display()
    )))?;
    let tops: Vec<PathBuf> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    let [top] = tops.as_slice() else {
        return Err(format!(
            "{}: expected one top directory, found {}",
            pin.tarball.display(),
            tops.len()
        )
        .into());
    };
    let usr = top.join("usr");
    if !usr.is_dir() {
        return Err(format!("{}: no usr/ under {}", pin.tarball.display(), top.display()).into());
    }
    // `cp -R usr/. root/usr/` merges into the existing directories.
    run(Command::new("cp")
        .arg("-R")
        .arg(usr.join("."))
        .arg(rootdir.join("usr")))?;
    println!("  {what}: {}", top.display());
    Ok(())
}

/// Writes `text` to `path` with `mode`.
fn write_mode(path: &Path, text: &str, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|e| format!("{}: {e}", path.display()).into())
}

/// The ffs image `out/<name>` of `rootdir`, the tree times 5/4 plus `room` bytes, by
/// `userland`'s makefs with the ownership table `owners`.
fn make_image(
    root: &Path,
    arch: Arch,
    out: &Path,
    rootdir: &Path,
    owners: &Path,
    name: &str,
    room: u64,
) -> Result<()> {
    const MIB: u64 = 1 << 20;
    let size = (ramdisk::tree_bytes(rootdir)? * 5 / 4 + room).div_ceil(MIB) * MIB;
    let disktab = out.join("disktab");
    ramdisk::write_if_changed(&disktab, &ramdisk::disktab_entry(size / 512))?;
    let image = out.join(name);
    let _ = fs::remove_file(&image);
    run(Command::new(makefs_of(root, arch))
        .args(["-t", "ffs", "-T", &ramdisk::TIMESTAMP.to_string()])
        .env("EMIBSD_DISKTAB", &disktab)
        .env("EMIBSD_OWNERS", owners)
        .env("EMIBSD_STAGING", rootdir)
        .args(["-o", "disklabel=rdroot,minfree=0,density=16384"])
        .arg(&image)
        .arg(rootdir))?;
    let bytes = fs::metadata(&image).map(|m| m.len()).unwrap_or(0);
    println!("  image: {} ({bytes} bytes)", image.display());
    Ok(())
}

/// `cargo xtask selfhost` (module docs).
pub(crate) fn selfhost(root: &Path, arch: Arch, llvm: Pinned<'_>, rust: Pinned<'_>) -> Result<()> {
    let out = root.join("target/selfhost").join(arch.name());
    let (rootdir, owners) = base_root(root, arch, &out)?;
    lay_tarball(&out, &rootdir, "llvm", &llvm)?;
    lay_tarball(&out, &rootdir, "rust", &rust)?;
    // The source, its crates and the build.
    let src = rootdir.join("src");
    let lz = src.join("lz");
    let cg = src.join("cg");
    for d in [&lz, &cg, &rootdir.join("tmp")] {
        fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    run(Command::new("sh").arg("-c").arg(format!(
        "git -C '{}' archive --format=tar HEAD | tar -xf - -C '{}'",
        root.display(),
        lz.display()
    )))?;
    run(Command::new("cargo")
        .args(["vendor", "--locked", "--quiet", "vendor"])
        .env("CARGO_NET_OFFLINE", "true")
        .current_dir(&lz)
        .stdout(Stdio::null()))?;
    fs::write(cg.join("config.toml"), CARGO_CONFIG)?;
    write_mode(&src.join("build.sh"), BUILD_SH, 0o755)?;
    make_image(
        root,
        arch,
        &out,
        &rootdir,
        &owners,
        "selfhost.ffs",
        BUILD_ROOM,
    )
}

/// The macro the base system's compiler predefines, and EmiBSD's in its place.
const OPENBSD_MACRO: &[u8] = b"__OpenBSD__";
const EMIBSD_MACRO: &[u8] = b"__EmiBSD__";

/// `text` with every `__OpenBSD__` renamed `__EmiBSD__`, or `None` when it names none.
fn renamed_macro(text: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len());
    let mut rest = text;
    let mut found = false;
    while let Some(at) = rest
        .windows(OPENBSD_MACRO.len())
        .position(|w| w == OPENBSD_MACRO)
    {
        out.extend_from_slice(&rest[..at]);
        out.extend_from_slice(EMIBSD_MACRO);
        rest = &rest[at + OPENBSD_MACRO.len()..];
        found = true;
    }
    out.extend_from_slice(rest);
    found.then_some(out)
}

/// Renames `__OpenBSD__` to `__EmiBSD__` in every regular file under `dir` (the disk's
/// copy of the sources, never the reference tree), as `world.py`'s `tree` step does in its
/// copy: DevTools' clang predefines `__EmiBSD__` alone (the user's decision of 2026-11-04).
/// Symbolic links are left alone. A file rewritten in place keeps its mode; an APFS clone
/// then gets its own blocks. Returns how many files changed.
fn rename_openbsd_macro(dir: &Path) -> Result<usize> {
    let mut n = 0;
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .file_type();
        if kind.is_dir() {
            n += rename_openbsd_macro(&path)?;
        } else if kind.is_file() {
            let text = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if let Some(new) = renamed_macro(&text) {
                fs::write(&path, new).map_err(|e| format!("{}: {e}", path.display()))?;
                n += 1;
            }
        }
    }
    Ok(n)
}

/// `cargo xtask selfhost-userland` (module docs).
pub(crate) fn selfhost_userland(root: &Path, arch: Arch, llvm: Pinned<'_>) -> Result<()> {
    let out = root.join("target/selfhost-userland").join(arch.name());
    let (rootdir, owners) = base_root(root, arch, &out)?;
    lay_tarball(&out, &rootdir, "llvm", &llvm)?;
    let src = openbsd_src(root)?;
    // /usr/src: APFS clones of the pin's directories (the reference is only read).
    let usr_src = rootdir.join("usr/src");
    for dir in USR_SRC {
        let from = src.join(dir);
        let to = usr_src.join(dir);
        let parent = to.parent().ok_or("/usr/src has no parent")?;
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        run(Command::new("cp").arg("-Rc").arg(&from).arg(&to))?;
    }
    // The drm driver's sources are 0.5 GiB no userland Makefile reads; its headers stay.
    let drm = usr_src.join("sys/dev/pci/drm");
    run(Command::new("find")
        .arg(&drm)
        .args(["-type", "f", "!", "-name", "*.h", "-delete"]))?;
    let renamed = rename_openbsd_macro(&usr_src)?;
    println!(
        "  /usr/src: {} of {}; __OpenBSD__ renamed __EmiBSD__ in {renamed} files",
        USR_SRC.join(" "),
        src.display()
    );
    // /etc: the ramdisk's, with OpenBSD's stock users, groups and login classes.
    let etc = rootdir.join("etc");
    let ramdisk_etc = root
        .join("target/userland")
        .join(arch.name())
        .join("ramdisk-root/etc");
    run(Command::new("cp").arg("-R").arg(&ramdisk_etc).arg(&etc))?;
    for (from, to) in [
        ("etc/master.passwd", "master.passwd"),
        ("etc/group", "group"),
        (
            &*format!("etc/etc.{}/login.conf", arch.name()),
            "login.conf",
        ),
    ] {
        let p = etc.join(to);
        let _ = fs::remove_file(&p);
        fs::copy(src.join(from), &p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    for f in ["passwd", "pwd.db", "spwd.db"] {
        let _ = fs::remove_file(etc.join(f));
    }
    let pwd_mkdb = root
        .join("target/userland")
        .join(arch.name())
        .join("host/bin/pwd_mkdb");
    passwd::make_databases(&pwd_mkdb, &etc)?;
    write_mode(&etc.join("mk.conf"), MK_CONF, 0o644)?;
    write_mode(&etc.join("clang-defaults.cfg"), CLANG_CFG, 0o644)?;
    for d in ["tmp", "var/tmp", "var/empty", "root", "usr/obj", "td1b"] {
        let p = rootdir.join(d);
        fs::create_dir_all(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    write_mode(&rootdir.join("td1b/build.sh"), UL_BUILD_SH, 0o755)?;
    // Owners and modes beyond comp's: OpenBSD's mtree (`etc/mtree/4.4BSD.dist`) gives
    // /usr/obj to build:wobj 0770; spwd.db is root:_shadow 0640, as pwd_mkdb(8) makes it.
    let mut table =
        fs::read_to_string(&owners).map_err(|e| format!("{}: {e}", owners.display()))?;
    for line in [
        "1777 0 0 /tmp",
        "1777 0 0 /var/tmp",
        "770 21 21 /usr/obj",
        "600 0 0 /etc/master.passwd",
        "640 0 65 /etc/spwd.db",
    ] {
        table.push_str(line);
        table.push('\n');
    }
    fs::write(&owners, table).map_err(|e| format!("{}: {e}", owners.display()))?;
    make_image(
        root,
        arch,
        &out,
        &rootdir,
        &owners,
        "userland.ffs",
        UL_BUILD_ROOM,
    )
}

/// Refuses `file` unless its SHA-256, by the host's `shasum`, is `want` (lowercase hex).
fn check_sha256(file: &Path, want: &str) -> Result<()> {
    let out = Command::new("shasum")
        .args(["-a", "256"])
        .arg(file)
        .output()
        .map_err(|e| format!("shasum: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let got = text.split_whitespace().next().unwrap_or("");
    if !out.status.success() || got != want {
        return Err(format!("{}: sha256 {got}, pinned {want}", file.display()).into());
    }
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_script_prints_its_markers_and_fits_the_console() {
        for marker in [
            "lz-build-rc-$rc",
            "lz-elf-$n",
            "lz-build-secs-",
            "ulimit -d",
        ] {
            assert!(BUILD_SH.contains(marker), "{marker}");
        }
        assert!(BUILD_SH.lines().all(|l| l.len() < 128));
        assert!(CARGO_CONFIG.contains("directory = \"/src/lz/vendor\""));
    }

    #[test]
    fn userland_script_prints_its_markers_and_fits_the_console() {
        for marker in [
            "ul-ls-out-",
            "ul-ls-run-rc-$?",
            "ul-$tag-rc-$rc",
            "ul-$tag-secs-",
            "ul-$t-obj-rc-$?",
            "ul-FAIL-$d/$sd",
            "ul-$t-subdirs-",
            "ul-$n-rc-$r",
            "ul-hints-rc-$?",
            "ul-ldconfig-rc-$?",
            "step1 mk-install ",
            "step1 includes ",
            "ul-mtree-rc-$?",
            "ulimit -d",
        ] {
            assert!(UL_BUILD_SH.contains(marker), "{marker}");
        }
        assert!(UL_BUILD_SH.lines().all(|l| l.len() < 128));
    }

    #[test]
    fn mk_conf_keeps_the_compilers_own_triple() {
        // No --target anywhere: the native clang's default triple is EmiBSD's.
        assert!(!MK_CONF.contains("--target") && !MK_CONF.contains("openbsd"));
        for var in ["CC=", "CXX=", "CPP="] {
            let line = MK_CONF
                .lines()
                .find(|l| l.starts_with(var))
                .unwrap_or_else(|| panic!("{var}"));
            assert!(line.contains("${EMIBSD_CFG}"), "{line}");
        }
        assert!(MK_CONF.contains("--config=/etc/clang-defaults.cfg"));
        assert!(MK_CONF.lines().any(|l| l == "HOSTCC=\t\t${CC}"));
        assert!(MK_CONF.lines().any(|l| l == "STACK_PROTECTOR="));
        assert!(USR_SRC.contains(&"share/mk") && USR_SRC.contains(&"lib"));
        let opts: Vec<&str> = CLANG_CFG.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(
            opts,
            ["-Wno-pointer-sign", "-Wno-error=unused-but-set-global"]
        );
    }

    #[test]
    fn the_macro_is_renamed_everywhere_and_only_it() {
        assert_eq!(renamed_macro(b"#ifdef __linux__\n"), None);
        assert_eq!(
            renamed_macro(b"#if defined(__OpenBSD__) || __OpenBSD__+0\n").as_deref(),
            Some(&b"#if defined(__EmiBSD__) || __EmiBSD__+0\n"[..])
        );
        assert_eq!(
            renamed_macro(b"__OpenBSD____OpenBSD__").as_deref(),
            Some(&b"__EmiBSD____EmiBSD__"[..])
        );
        // A near miss stays: OpenBSD's name in text is not the macro.
        assert_eq!(renamed_macro(b"__OpenBSD_ OpenBSD __OPENBSD__"), None);
        assert_eq!(renamed_macro(b""), None);
    }

    #[test]
    fn the_rename_walks_files_and_leaves_links() {
        let dir = std::env::temp_dir().join(format!("xtask-rename-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("a/b/x.h"), "#ifdef __OpenBSD__\n").unwrap();
        fs::write(dir.join("y.c"), "int y;\n").unwrap();
        let outside = dir.with_extension("outside");
        fs::write(&outside, "__OpenBSD__").unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("link")).unwrap();
        assert_eq!(rename_openbsd_macro(&dir).unwrap(), 1);
        assert_eq!(
            fs::read_to_string(dir.join("a/b/x.h")).unwrap(),
            "#ifdef __EmiBSD__\n"
        );
        assert_eq!(fs::read_to_string(&outside).unwrap(), "__OpenBSD__");
        fs::remove_dir_all(&dir).unwrap();
        fs::remove_file(&outside).unwrap();
    }
}
/* </TESTS> */
