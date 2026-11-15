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
//! - the base: a clone of `comp`'s staging root (libc, crt, ld.so, the headers), its libc++
//!   headers and clang resource directory removed (DevTools' LLVM replaces them as a whole),
//!   and the shared libraries of `userland`'s root laid over it, so libcrypto and libssl are
//!   this tree's (native cargo loads them);
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

use super::*;

/// Free space for the kernel's build inside the VM: one debug kernel with its dependencies
/// takes about 2 GiB of `target/` with `CARGO_INCREMENTAL=0`; the rest is margin.
const BUILD_ROOM: u64 = 8 << 30;

/// What the guest runs, in the chroot, as `/src/build.sh`. Every line it prints stays under
/// arm64's 128-byte console limit; its markers are printed, never typed, so the smoke's
/// expectations cannot be met by an echoed command line.
const BUILD_SH: &str = r#"#!/bin/ksh
# TD1a: builds LandingZone's arm64 kernel with DevTools' native toolchain, in the chroot.
export HOME=/tmp CARGO_HOME=/src/cg CARGO_INCREMENTAL=0
export CARGO_TARGET_AARCH64_EMIBSD_NONE_SOFTFLOAT_LINKER=/usr/bin/ld.lld
export EMIBSD_BUILD=0 SOURCE_DATE_EPOCH=0 EMIBSD_BUILD_HOST=emibsd
cd /src/lz || exit 9
s=$(date +%s)
cargo build --offline -p bsd --target aarch64-emibsd-none-softfloat \
    --features qemu,multiprocessor -j ${JOBS:-4} > /src/build.log 2>&1
rc=$?
e=$(date +%s)
tail -n 25 /src/build.log | cut -c1-110
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

/// `cargo xtask selfhost` (module docs).
pub(crate) fn selfhost(root: &Path, arch: Arch, llvm: Pinned<'_>, rust: Pinned<'_>) -> Result<()> {
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
    let makefs = root
        .join("target/userland")
        .join(arch.name())
        .join("host/bin/makefs");
    if !makefs.is_file() || !user_lib.is_dir() {
        return Err(format!("{}: missing; run `just userland`", makefs.display()).into());
    }
    let out = root.join("target/selfhost").join(arch.name());
    let unpack = out.join("unpack");
    let rootdir = out.join("root");
    for d in [&unpack, &rootdir] {
        if d.exists() {
            fs::remove_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
    }
    fs::create_dir_all(&unpack).map_err(|e| format!("{}: {e}", unpack.display()))?;
    // The base, an APFS clone of comp's root (seconds, no space until written).
    run(Command::new("cp").arg("-Rc").arg(&comp_root).arg(&rootdir))?;
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
    for (what, pin) in [("llvm", &llvm), ("rust", &rust)] {
        check_sha256(pin.tarball, pin.sha256)?;
        let dir = unpack.join(what);
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
            return Err(
                format!("{}: no usr/ under {}", pin.tarball.display(), top.display()).into(),
            );
        }
        // `cp -R usr/. root/usr/` merges into the existing directories.
        run(Command::new("cp")
            .arg("-R")
            .arg(usr.join("."))
            .arg(rootdir.join("usr")))?;
        println!("  {what}: {}", top.display());
    }
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
    let build = src.join("build.sh");
    fs::write(&build, BUILD_SH)?;
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&build, fs::Permissions::from_mode(0o755))?;
    }
    let owners = out.join("owners.txt");
    fs::copy(&comp_owners, &owners).map_err(|e| format!("{}: {e}", owners.display()))?;
    const MIB: u64 = 1 << 20;
    let size = (ramdisk::tree_bytes(&rootdir)? * 5 / 4 + BUILD_ROOM).div_ceil(MIB) * MIB;
    let disktab = out.join("disktab");
    ramdisk::write_if_changed(&disktab, &ramdisk::disktab_entry(size / 512))?;
    let image = out.join("selfhost.ffs");
    let _ = fs::remove_file(&image);
    run(Command::new(&makefs)
        .args(["-t", "ffs", "-T", &ramdisk::TIMESTAMP.to_string()])
        .env("EMIBSD_DISKTAB", &disktab)
        .env("EMIBSD_OWNERS", &owners)
        .env("EMIBSD_STAGING", &rootdir)
        .args(["-o", "disklabel=rdroot,minfree=0,density=16384"])
        .arg(&image)
        .arg(&rootdir))?;
    let bytes = fs::metadata(&image).map(|m| m.len()).unwrap_or(0);
    println!("  image: {} ({bytes} bytes)", image.display());
    Ok(())
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
        for marker in ["lz-build-rc-$rc", "lz-elf-$n", "lz-build-secs-"] {
            assert!(BUILD_SH.contains(marker), "{marker}");
        }
        assert!(BUILD_SH.lines().all(|l| l.len() < 128));
        assert!(CARGO_CONFIG.contains("directory = \"/src/lz/vendor\""));
    }
}
/* </TESTS> */
