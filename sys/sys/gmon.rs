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

/*-
 * Copyright (c) 1982, 1986, 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)gmon.h	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/gmon.h>`: the profiling data a profiled program, gprof(1) and the kernel share.
//!
//! Upstream: sys/sys/gmon.h @ 3ce1f3f79392
//!
//! Status: `ported`, the whole header: `struct gmonhdr` (what `profil(2)`'s buffer starts
//! with, and `prof_write` reads), the histogram and arc table parameters, `struct tostruct`,
//! `struct rawarc`, `struct gmonparam`, the profiling states and the `GPROF_*` sysctl names.
//!
//! ## Deviations
//! - `HISTCOUNTER`, a macro that names a type, is the type [`Histcounter`].
//! - `gmoninit` is the kernel's own profiling (`GPROF`), which is not configured: not
//!   declared. `<machine/profile.h>` (`mcount`) is not ported for the same reason.
//! - `_gmonparam`, `_mcleanup`, `_monstartup`, `moncontrol` and `_gmon_alloc` are declared
//!   outside the kernel only.

use core::ffi::c_void;

use crate::sys::queue::SlistEntry;

/// `GMONVERSION`: the version number in a `gmonhdr`.
pub const GMONVERSION: i32 = 0x0005_1879;

/// `HISTFRACTION`: fraction of text space to allocate for histogram counters here, 1/2.
pub const HISTFRACTION: u64 = 2;

/// `HASHFRACTION`: fraction of text space to allocate for from hash buckets, 2 across all
/// architectures.
pub const HASHFRACTION: u64 = 2;

/// `ARCDENSITY`: percent of text space to allocate for tostructs, with a minimum.
pub const ARCDENSITY: u64 = 2;
/// `MINARCS`.
pub const MINARCS: i64 = 50;
/// `MAXARCS`: `(1 << (8 * sizeof(HISTCOUNTER))) - 2`.
pub const MAXARCS: i64 = (1 << Histcounter::BITS) - 2;

// Possible states of profiling.

/// `GMON_PROF_ON`.
pub const GMON_PROF_ON: i32 = 0;
/// `GMON_PROF_BUSY`.
pub const GMON_PROF_BUSY: i32 = 1;
/// `GMON_PROF_ERROR`.
pub const GMON_PROF_ERROR: i32 = 2;
/// `GMON_PROF_OFF`.
pub const GMON_PROF_OFF: i32 = 3;

// Sysctl definitions for extracting profiling information from the kernel.

/// `GPROF_STATE`: int: profiling enabling variable.
pub const GPROF_STATE: i32 = 0;
/// `GPROF_COUNT`: struct: profile tick count buffer.
pub const GPROF_COUNT: i32 = 1;
/// `GPROF_FROMS`: struct: from location hash bucket.
pub const GPROF_FROMS: i32 = 2;
/// `GPROF_TOS`: struct: destination/count structure.
pub const GPROF_TOS: i32 = 3;
/// `GPROF_GMONPARAM`: struct: profiling parameters.
pub const GPROF_GMONPARAM: i32 = 4;

/// `struct gmonhdr`: structure prepended to gmon.out profiling data file.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Gmonhdr {
    /// `lpc`: base pc address of sample buffer.
    pub lpc: u64,
    /// `hpc`: max pc address of sampled buffer.
    pub hpc: u64,
    /// `ncnt`: size of sample buffer (plus this header).
    pub ncnt: i32,
    /// `version`: version number.
    pub version: i32,
    /// `profrate`: profiling clock rate.
    pub profrate: i32,
    /// `totarc`: space used by arcs.
    pub totarc: i32,
    /// `spare`: reserved.
    pub spare: [i32; 2],
}

/// `HISTCOUNTER`: histogram counters are unsigned shorts (according to the kernel).
pub type Histcounter = u16;

/// `struct tostruct`: an arc's destination and count.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Tostruct {
    /// `selfpc`.
    pub selfpc: u64,
    /// `count`.
    pub count: i64,
    /// `link`.
    pub link: u16,
    /// `pad`.
    pub pad: u16,
}

/// `struct rawarc`: a raw arc, with pointers to the calling site and the called site and a
/// count.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Rawarc {
    /// `raw_frompc`.
    pub raw_frompc: u64,
    /// `raw_selfpc`.
    pub raw_selfpc: u64,
    /// `raw_count`.
    pub raw_count: i64,
}

/// `struct gmonparam`: the profiling data structures are housed in this structure.
#[repr(C)]
pub struct Gmonparam {
    /// `state`.
    pub state: i32,
    /// `kcount`.
    pub kcount: *mut u16,
    /// `kcountsize`.
    pub kcountsize: u64,
    /// `froms`.
    pub froms: *mut u16,
    /// `fromssize`.
    pub fromssize: u64,
    /// `tos`.
    pub tos: *mut Tostruct,
    /// `tossize`.
    pub tossize: u64,
    /// `tolimit`.
    pub tolimit: i64,
    /// `lowpc`.
    pub lowpc: u64,
    /// `highpc`.
    pub highpc: u64,
    /// `textsize`.
    pub textsize: u64,
    /// `hashfraction`.
    pub hashfraction: u64,
    /// `outbuf`.
    pub outbuf: *mut c_void,
    /// `outbuflen`.
    pub outbuflen: usize,
    /// `rawarcs`.
    pub rawarcs: *mut c_void,
    /// `dirfd`.
    pub dirfd: i32,
    /// `list`.
    pub list: SlistEntry<Gmonparam>,
}

/// `ROUNDDOWN(x, y)`.
pub const fn rounddown(x: u64, y: u64) -> u64 {
    (x / y) * y
}

/// `ROUNDUP(x, y)`.
pub const fn roundup(x: u64, y: u64) -> u64 {
    x.div_ceil(y) * y
}

const _: () = {
    assert!(size_of::<Gmonhdr>() == 40);
    assert!(size_of::<Tostruct>() == 24);
    assert!(size_of::<Rawarc>() == 24);
    assert!(size_of::<Gmonparam>() == 136);
    assert!(MAXARCS == 65534);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_is_to_a_multiple() {
        assert_eq!(rounddown(1001, 4), 1000);
        assert_eq!(roundup(1001, 4), 1004);
        assert_eq!(roundup(1000, 4), 1000);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/gmon.h");
        crate::reftest::assert_defines!(defs;
            GMONVERSION, HISTFRACTION, HASHFRACTION, ARCDENSITY, MINARCS, GMON_PROF_ON,
            GMON_PROF_BUSY, GMON_PROF_ERROR, GMON_PROF_OFF, GPROF_STATE, GPROF_COUNT,
            GPROF_FROMS, GPROF_TOS, GPROF_GMONPARAM,
        );
    }
}
/* </TESTS> */
