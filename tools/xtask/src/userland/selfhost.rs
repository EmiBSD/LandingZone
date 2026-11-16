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
//! The self-hosting disk (TDk, TD1a until 2026-11-15): what `just smoke-selfhost-kernel`
//! boots to build this repository's kernel inside EmiBSD with DevTools' native toolchain.
//!
//! `cargo xtask selfhost --arch A --userland TAR --userland-sha256 HEX [--rev REV]` makes
//! `target/selfhost/<arch>/selfhost.ffs`, an ffs image of:
//! - the base system: the `base` and `comp` sets of the Userland repository's release (TD0,
//!   the user's decision of 2026-11-15: this repository is the kernel and takes the userland
//!   Userland makes, it does not build one for the VM). The release tarball is checked
//!   against its pin in the justfile and its two sets are unpacked unmodified, the owners
//!   and modes of their tar headers kept in the image. Since Userland 2026.11.6 its `comp`
//!   carries DevTools' native LLVM and Rust (`aarch64-emibsd-emibsd8.0`; D-0009, D-0010,
//!   D-0012), so nothing is laid over the sets. The tree is staged on a case-sensitive APFS
//!   volume ([`Staging`]): the sets hold names that differ only in case (`/usr/bin/Mail`
//!   and `mail`, `/usr/sbin/ZZZ` and `zzz`, terminfo's entries), which the Mac's own file
//!   system cannot hold;
//! - `/src/lz`: this repository's `HEAD` (`git archive`; `--rev REV` takes another commit,
//!   e.g. `git stash create`'s, to build a tree that is not committed yet), with every
//!   crate of `Cargo.lock` vendored (`cargo vendor --locked`, offline: the crates come from
//!   the host's cargo cache, never the network), `/src/cg/config.toml` pointing cargo at
//!   them, `/src/build.sh`, the build `smoke-selfhost-kernel` runs in a chroot, and
//!   `/src/ci.sh`, `smoke-selfhost-ci`'s ([`CI_SH`]);
//! - room to build: the image is the tree times 5/4 plus [`BUILD_ROOM`].
//!
//! The image is made afresh on every run, by OpenBSD's makefs(8) built for the Mac with
//! `ramdisk.rs`'s ownership shim (`ramdisk::build_makefs`, the one host tool built here).

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
# TDk: builds LandingZone's arm64 kernel with the comp set's native toolchain, in the chroot.
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

/// What `smoke-selfhost-ci` runs in the chroot as `/src/ci.sh`: the steps of `just ci` that
/// fit inside EmiBSD, for arm64 and for the host (which is EmiBSD there), in `ci`'s order.
/// Left out: amd64's half (the native arm64 tree has no amd64 target), the smokes (they
/// boot QEMU), and `check-ports`, `check-syscalls` and `test-ref` (they read the reference
/// tree, which is not on the disk). Each step's output goes to `/src/ci-<step>.log`, its
/// last lines and `lz-ci-<step>-rc-<status>` to the console; every step runs whatever the
/// one before did, and the last line, `lz-ci-rc-<N>`, counts the steps that failed. The
/// xtask tests need `lo0` configured (the recipe does it) and sh, sleep, ps and kill.
const CI_SH: &str = r#"#!/bin/ksh
# The steps of `just ci` that fit inside EmiBSD (arm64 and the host), in the chroot.
export HOME=/tmp CARGO_HOME=/src/cg CARGO_INCREMENTAL=0
export CARGO_TARGET_AARCH64_EMIBSD_NONE_SOFTFLOAT_LINKER=/usr/bin/ld.lld
export EMIBSD_BUILD=0 SOURCE_DATE_EPOCH=0 EMIBSD_BUILD_HOST=emibsd
export EMIBSD_LLVM_BIN=/usr/bin
ulimit -d $(ulimit -H -d)
cd /src/lz || exit 9
T="--target aarch64-emibsd-none-softfloat"
W="-- -D warnings"
C="--offline -j ${JOBS:-4}"
short() { typeset -L110 l; while IFS= read -r l; do print -r -- "$l"; done; }
bad=0
step() {
    n=$1; shift
    s=$(date +%s)
    "$@" > /src/ci-$n.log 2>&1
    rc=$?
    tail -n 4 /src/ci-$n.log | short
    echo "lz-ci-$n-rc-$rc in $(($(date +%s) - s)) s"
    [ $rc -eq 0 ] || bad=$((bad + 1))
}
step fmt cargo fmt --all -- --check
step clippy-kernel cargo clippy $C -p bsd $T --features qemu $W
step clippy-mp cargo clippy $C -p bsd $T --features qemu,multiprocessor $W
step clippy-init cargo clippy $C -p init $T $W
step clippy-host cargo clippy $C -p libkern -p libz -p bsd -p xtask $W
step clippy-boot cargo clippy $C -p libsa -p boot -p efi $W
step clippy-boot-arm64 cargo clippy $C -p libsa -p boot -p efi $T $W
step clippy-efiboot cargo clippy $C -p efiboot-arm64 $T $W
step test cargo test $C -p libkern -p libz -p bsd -p xtask
step test-boot cargo test $C -p libsa -p boot -p efi
step test-efiboot cargo test $C -p efiboot-arm64
step build cargo build $C -p bsd $T --features qemu,multiprocessor
step build-up cargo build $C -p bsd $T
step build-init cargo build $C -p init $T
echo "lz-ci-rc-$bad"
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

/// The version in the names of the release's sets (`base80.tar.zst`): EmiBSD 8.0.
const SETS_VERSION: &str = "80";

/// macOS's disk image tool (`/usr/bin/hdiutil`).
const HDIUTIL: &str = "/usr/bin/hdiutil";

/// The case-sensitive volume the disk's tree is staged on (module docs): a sparse APFS image
/// `out/staging.sparseimage` attached at `out/staging`, without ownership and out of the
/// Finder's sight. Dropping it detaches the volume and removes the image, so nothing stays
/// attached after `selfhost` returns, whether it worked or not.
struct Staging {
    image: PathBuf,
    mount: PathBuf,
}

impl Staging {
    /// Makes the image afresh and attaches it. A run that was killed leaves its volume
    /// attached: that one is detached first.
    fn attach(out: &Path) -> Result<Staging> {
        let staging = Staging {
            image: out.join("staging.sparseimage"),
            mount: out.join("staging"),
        };
        staging.detach();
        fs::create_dir_all(&staging.mount)
            .map_err(|e| format!("{}: {e}", staging.mount.display()))?;
        run(Command::new(HDIUTIL)
            .args(["create", "-quiet", "-type", "SPARSE", "-size", "64g"])
            .args(["-fs", "Case-sensitive APFS", "-volname", "emibsd-selfhost"])
            .arg(&staging.image))?;
        run(Command::new(HDIUTIL)
            .args(["attach", "-quiet", "-nobrowse", "-mountpoint"])
            .arg(&staging.mount)
            .arg(&staging.image))?;
        Ok(staging)
    }

    /// Detaches the volume if it is attached, and removes the image and the mount point.
    fn detach(&self) {
        if self.mount.is_dir() {
            let _ = Command::new(HDIUTIL)
                .args(["detach", "-quiet", "-force"])
                .arg(&self.mount)
                .stderr(Stdio::null())
                .status();
            let _ = fs::remove_dir(&self.mount);
        }
        let _ = fs::remove_file(&self.image);
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        self.detach();
    }
}

/// A pinned tarball for [`selfhost`] (the Userland repository's release): its path and the
/// sha256 it is pinned to.
pub(crate) struct Pinned<'a> {
    pub(crate) tarball: &'a Path,
    pub(crate) sha256: &'a str,
}

/// The base of the self-hosting disk, made in `staging`'s `root`: the `base` and `comp` sets
/// of the Userland release `userland` (checked against its pin; one top directory holding
/// `base80.tar.zst` and `comp80.tar.zst`), unpacked in that order. Returns the root and the
/// ownership table of the sets' entries (`owners.txt`, the makefs shim's `mode uid gid path`
/// lines).
fn base_root(out: &Path, staging: &Staging, userland: &Pinned<'_>) -> Result<(PathBuf, PathBuf)> {
    let rootdir = staging.mount.join("root");
    let unpack = out.join("unpack");
    if unpack.exists() {
        fs::remove_dir_all(&unpack).map_err(|e| format!("{}: {e}", unpack.display()))?;
    }
    fs::create_dir_all(&rootdir).map_err(|e| format!("{}: {e}", rootdir.display()))?;
    check_sha256(userland.tarball, userland.sha256)?;
    let release = unpack.join("release");
    fs::create_dir_all(&release).map_err(|e| format!("{}: {e}", release.display()))?;
    run(Command::new("sh").arg("-c").arg(format!(
        "zstd -dcq '{}' | tar -xf - -C '{}'",
        userland.tarball.display(),
        release.display()
    )))?;
    let tops: Vec<PathBuf> = fs::read_dir(&release)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    let [top] = tops.as_slice() else {
        return Err(format!(
            "{}: expected one top directory, found {}",
            userland.tarball.display(),
            tops.len()
        )
        .into());
    };
    let mut table = String::new();
    for what in ["base", "comp"] {
        let set = top.join(format!("{what}{SETS_VERSION}.tar.zst"));
        if !set.is_file() {
            return Err(format!("{}: no {}", userland.tarball.display(), set.display()).into());
        }
        table.push_str(&lay_set(out, &rootdir, what, &set)?);
    }
    fs::remove_dir_all(&release).map_err(|e| format!("{}: {e}", release.display()))?;
    let owners = out.join("owners.txt");
    fs::write(&owners, table).map_err(|e| format!("{}: {e}", owners.display()))?;
    Ok((rootdir, owners))
}

/// Unpacks the set `set` (a zstd-compressed ustar archive of paths `./...`, from the release
/// [`base_root`] checked) into `rootdir` and returns the ownership table of its entries.
fn lay_set(out: &Path, rootdir: &Path, what: &str, set: &Path) -> Result<String> {
    let dir = out.join("unpack");
    let tar = dir.join(format!("{what}.tar"));
    run(Command::new("sh").arg("-c").arg(format!(
        "zstd -dcq '{}' > '{}'",
        set.display(),
        tar.display()
    )))?;
    let bytes = fs::read(&tar).map_err(|e| format!("{}: {e}", tar.display()))?;
    let table = set_owners(&bytes).map_err(|e| format!("{}: {e}", set.display()))?;
    run(Command::new("tar")
        .arg("-xf")
        .arg(&tar)
        .arg("-C")
        .arg(rootdir))?;
    // The sets hold directories their owner cannot read (`/var/spool/ftp/hidden`, 0111) and
    // files nobody can write; the staged tree is the user's, who must read all of it and
    // write the source beside it. The image gets every entry's own mode from the table.
    run(Command::new("chmod").args(["-R", "u+rwX"]).arg(rootdir))?;
    fs::remove_file(&tar).map_err(|e| format!("{}: {e}", tar.display()))?;
    println!(
        "  {what}: {} ({} entries)",
        set.display(),
        table.lines().count()
    );
    Ok(table)
}

/// The `mode uid gid path` lines of every entry of the ustar archive `tar`, from its headers
/// (the permission bits of `mode`, the numeric owner and group, the name with its `prefix`,
/// `./` turned into `/`). Unpacked by a user, the files are the user's; makefs gives them the
/// set's owners and modes back from this table.
fn set_owners(tar: &[u8]) -> Result<String> {
    let mut lines = String::new();
    let mut at = 0;
    while let Some(h) = tar.get(at..at + 512) {
        if h.iter().all(|&b| b == 0) {
            break;
        }
        let size = octal(&h[124..136])?;
        let mode = octal(&h[100..108])? & 0o7777;
        let (uid, gid) = (octal(&h[108..116])?, octal(&h[116..124])?);
        let name = c_str(&h[..100]);
        let prefix = if h[257..262] == *b"ustar" {
            c_str(&h[345..500])
        } else {
            ""
        };
        let full = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        };
        let path = full.trim_start_matches('.').trim_end_matches('/');
        if !path.is_empty() {
            lines.push_str(&format!("{mode:o} {uid} {gid} {path}\n"));
        }
        let blocks = usize::try_from(size.div_ceil(512)).map_err(|_| "entry too large")?;
        at += 512 * (1 + blocks);
    }
    Ok(lines)
}

/// A tar header's octal number field: digits padded with spaces or NULs.
fn octal(field: &[u8]) -> Result<u64> {
    let text = c_str(field).trim();
    if text.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(text, 8).map_err(|e| format!("tar header field `{text}`: {e}").into())
}

/// A NUL-terminated (or full) header field as text.
fn c_str(field: &[u8]) -> &str {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    std::str::from_utf8(&field[..end]).unwrap_or("")
}

/// Writes `text` to `path` with `mode`.
fn write_mode(path: &Path, text: &str, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|e| format!("{}: {e}", path.display()).into())
}

/// The ffs image `out/<name>` of `rootdir`, the tree times 5/4 plus `room` bytes, by
/// the host `makefs` with the ownership table `owners`.
fn make_image(
    makefs: &Path,
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
    run(Command::new(makefs)
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
pub(crate) fn selfhost(root: &Path, arch: Arch, userland: Pinned<'_>, rev: &str) -> Result<()> {
    // OpenBSD's makefs(8) for the Mac with the ownership shim, as the install media get it:
    // built here if it is missing or its shim changed, and nothing else of `userland` with it.
    let makefs = super::with_ctx(root, arch, ramdisk::build_makefs)?;
    let out = root.join("target/selfhost").join(arch.name());
    fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let staging = Staging::attach(&out)?;
    let (rootdir, owners) = base_root(&out, &staging, &userland)?;
    // The source, its crates and the build.
    let src = rootdir.join("src");
    let lz = src.join("lz");
    let cg = src.join("cg");
    for d in [&lz, &cg, &rootdir.join("tmp")] {
        fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    run(Command::new("sh").arg("-c").arg(format!(
        "git -C '{}' archive --format=tar '{rev}' | tar -xf - -C '{}'",
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
    write_mode(&src.join("ci.sh"), CI_SH, 0o755)?;
    make_image(&makefs, &out, &rootdir, &owners, "selfhost.ffs", BUILD_ROOM)
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
        for marker in ["lz-ci-$n-rc-$rc", "lz-ci-rc-$bad", "ulimit -d", "step fmt "] {
            assert!(CI_SH.contains(marker), "{marker}");
        }
        assert!(CI_SH.lines().all(|l| l.len() < 128));
        assert!(CARGO_CONFIG.contains("directory = \"/src/lz/vendor\""));
    }

    /// A ustar header block for `name` (split into `prefix` when given).
    fn header(prefix: &str, name: &str, mode: u32, uid: u32, gid: u32, size: usize) -> Vec<u8> {
        let mut h = vec![0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        for (at, text) in [
            (100, format!("{mode:07o}")),
            (108, format!("{uid:07o}")),
            (116, format!("{gid:07o}")),
            (124, format!("{size:011o}")),
        ] {
            h[at..at + text.len()].copy_from_slice(text.as_bytes());
        }
        h[156] = if name.ends_with('/') { b'5' } else { b'0' };
        h[257..263].copy_from_slice(b"ustar\0");
        h[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
        h
    }

    #[test]
    fn the_sets_owners_come_from_their_tar_headers() {
        let mut tar = header("", "./", 0o755, 0, 0, 0);
        tar.extend(header("", "./usr/bin/", 0o755, 0, 7, 0));
        tar.extend(header("", "./usr/bin/passwd", 0o4555, 0, 7, 600));
        tar.extend(vec![b'x'; 1024]); // 600 bytes of data, padded to two blocks
        tar.extend(header("./usr/share", "misc/termcap", 0o444, 0, 7, 0));
        tar.extend(vec![0u8; 1024]);
        tar.extend(header("", "./ignored-after-the-end", 0o644, 9, 9, 0));
        assert_eq!(
            set_owners(&tar).unwrap(),
            "755 0 7 /usr/bin\n4555 0 7 /usr/bin/passwd\n444 0 7 /usr/share/misc/termcap\n"
        );
    }

    #[test]
    fn a_bad_header_number_is_an_error() {
        let mut h = header("", "./x", 0o644, 0, 0, 0);
        h[124..136].copy_from_slice(b"99999999999\0");
        assert!(set_owners(&h).is_err());
    }
}
/* </TESTS> */
