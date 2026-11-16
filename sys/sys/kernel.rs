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
 * Copyright (c) 1990, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)kernel.h	8.3 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! Global variables for the kernel: `<sys/kernel.h>`.
//!
//! Upstream: sys/sys/kernel.h @ 3ce1f3f79392
//!
//! Status: `ported`. The header defines one value, [`HZ`], and declares twelve globals,
//! each of which is defined where the C defines it:
//!
//! | `<sys/kernel.h>` | defined in | here |
//! |---|---|---|
//! | `hostid`, `hostname`, `hostnamelen`, `domainname`, `domainnamelen` | `kern/kern_sysctl.c` | `kern/kern_sysctl.rs`: `HOSTID`, `HOSTNAME`, `HOSTNAMELEN`, `DOMAINNAME`, `DOMAINNAMELEN` |
//! | `utc_offset`, `tick`, `tick_nsec`, `hz` | `conf/param.c` | `conf/param.rs`: `UTC_OFFSET`, `TICK`, `TICK_NSEC`, `HZ` |
//! | `ticks`, `stathz`, `profhz` | `kern/kern_clock.c` | `kern/kern_clock.rs`: `TICKS`, `STATHZ`, `PROFHZ` |
//!
//! ## Deviations
//!
//! - An `extern` declaration has no counterpart in Rust: a user names the static's module.

/// `HZ`: the default system clock frequency, in ticks per second, when the kernel
/// configuration does not set one.
pub const HZ: i32 = 100;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn hz_matches_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/kernel.h");
        assert_eq!(crate::reftest::int(&defs, "HZ"), Some(HZ as i64));
    }

    /// The globals the header declares exist where the table in the module docs says.
    #[test]
    fn the_declared_globals_are_defined() {
        use core::sync::atomic::Ordering::Relaxed;
        let _ = crate::conf::param::HZ.load(Relaxed);
        let _ = crate::conf::param::TICK.load(Relaxed);
        let _ = crate::conf::param::TICK_NSEC.load(Relaxed);
        let _ = crate::conf::param::UTC_OFFSET.load(Relaxed);
        let _ = &crate::kern::kern_clock::TICKS;
        let _ = &crate::kern::kern_clock::STATHZ;
        let _ = &crate::kern::kern_clock::PROFHZ;
        let _ = &crate::kern::kern_sysctl::HOSTID;
        let _ = &crate::kern::kern_sysctl::HOSTNAME;
        let _ = &crate::kern::kern_sysctl::HOSTNAMELEN;
        let _ = &crate::kern::kern_sysctl::DOMAINNAME;
        let _ = &crate::kern::kern_sysctl::DOMAINNAMELEN;
    }
}
/* </TESTS> */
