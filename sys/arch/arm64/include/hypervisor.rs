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
 * Copyright (c) 2013, 2014 Andrew Turner
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * $FreeBSD: head/sys/arm64/include/hypervisor.h 281494 2015-04-13 14:43:10Z andrew $
 */
/* </LICENSES> */

/* <CODE> */
//! arm64 `<machine/hypervisor.h>`: the EL2 registers' bit definitions.
//!
//! Upstream: sys/arch/arm64/include/hypervisor.h @ 3ce1f3f79392
//!
//! Status: `ported`. These registers are only useful when in hypervisor context, e.g.
//! specific to EL2, or controlling the hypervisor: the bits of the architecture feature
//! trap register, `CPTR_EL2`, and of the hypervisor config register, `HCR_EL2`. `locore.S`'s
//! `drop_to_el1` writes `CPTR_RES1`, `HCR_TGE`, `HCR_RW`, `HCR_E2H`, `HCR_APK` and
//! `HCR_API` when boot(8) enters the kernel at EL2; the rest waits for its user, vmm(4).

// Architecture feature trap register

/// `CPTR_RES0`: the bits of `CPTR_EL2` that read as zero.
pub const CPTR_RES0: u64 = 0x7fef_c800;
/// `CPTR_RES1`: the bits of `CPTR_EL2` that read as one.
pub const CPTR_RES1: u64 = 0x0000_32ff;
/// `CPTR_TFP`: trap the floating point and SIMD registers.
pub const CPTR_TFP: u64 = 0x0000_0400;
/// `CPTR_TTA`: trap the trace registers.
pub const CPTR_TTA: u64 = 0x0010_0000;
/// `CPTR_TCPAC`: trap `CPACR_EL1`.
pub const CPTR_TCPAC: u64 = 0x8000_0000;

// Hypervisor Config Register

/// `HCR_VM`: enable stage 2 translation.
pub const HCR_VM: u64 = 0x0000_0000_0000_0001;
/// `HCR_SWIO`: set/way invalidation override.
pub const HCR_SWIO: u64 = 0x0000_0000_0000_0002;
/// `HCR_PTW`: protected table walk.
pub const HCR_PTW: u64 = 0x0000_0000_0000_0004;
/// `HCR_FMO`: route physical FIQs to EL2.
pub const HCR_FMO: u64 = 0x0000_0000_0000_0008;
/// `HCR_IMO`: route physical IRQs to EL2.
pub const HCR_IMO: u64 = 0x0000_0000_0000_0010;
/// `HCR_AMO`: route physical SErrors to EL2.
pub const HCR_AMO: u64 = 0x0000_0000_0000_0020;
/// `HCR_VF`: a virtual FIQ is pending.
pub const HCR_VF: u64 = 0x0000_0000_0000_0040;
/// `HCR_VI`: a virtual IRQ is pending.
pub const HCR_VI: u64 = 0x0000_0000_0000_0080;
/// `HCR_VSE`: a virtual SError is pending.
pub const HCR_VSE: u64 = 0x0000_0000_0000_0100;
/// `HCR_FB`: force broadcast of TLB and cache maintenance.
pub const HCR_FB: u64 = 0x0000_0000_0000_0200;
/// `HCR_BSU_MASK`: barrier shareability upgrade.
pub const HCR_BSU_MASK: u64 = 0x0000_0000_0000_0c00;
/// `HCR_DC`: default cacheable.
pub const HCR_DC: u64 = 0x0000_0000_0000_1000;
/// `HCR_TWI`: trap `wfi`.
pub const HCR_TWI: u64 = 0x0000_0000_0000_2000;
/// `HCR_TWE`: trap `wfe`.
pub const HCR_TWE: u64 = 0x0000_0000_0000_4000;
/// `HCR_TID0`: trap ID group 0.
pub const HCR_TID0: u64 = 0x0000_0000_0000_8000;
/// `HCR_TID1`: trap ID group 1.
pub const HCR_TID1: u64 = 0x0000_0000_0001_0000;
/// `HCR_TID2`: trap ID group 2.
pub const HCR_TID2: u64 = 0x0000_0000_0002_0000;
/// `HCR_TID3`: trap ID group 3.
pub const HCR_TID3: u64 = 0x0000_0000_0004_0000;
/// `HCR_TSC`: trap `smc`.
pub const HCR_TSC: u64 = 0x0000_0000_0008_0000;
/// `HCR_TIDCP`: trap the implementation defined registers.
pub const HCR_TIDCP: u64 = 0x0000_0000_0010_0000;
/// `HCR_TACR`: trap the auxiliary control registers.
pub const HCR_TACR: u64 = 0x0000_0000_0020_0000;
/// `HCR_TSW`: trap the cache maintenance by set/way.
pub const HCR_TSW: u64 = 0x0000_0000_0040_0000;
/// `HCR_TPC`: trap the data and unified cache maintenance.
pub const HCR_TPC: u64 = 0x0000_0000_0080_0000;
/// `HCR_TPU`: trap the cache maintenance of EL0.
pub const HCR_TPU: u64 = 0x0000_0000_0100_0000;
/// `HCR_TTLB`: trap the TLB maintenance.
pub const HCR_TTLB: u64 = 0x0000_0000_0200_0000;
/// `HCR_TVM`: trap the writes of the virtual memory controls.
pub const HCR_TVM: u64 = 0x0000_0000_0400_0000;
/// `HCR_TGE`: trap general exceptions to EL2 (host mode with `HCR_E2H`).
pub const HCR_TGE: u64 = 0x0000_0000_0800_0000;
/// `HCR_TDZ`: trap `dc zva`.
pub const HCR_TDZ: u64 = 0x0000_0000_1000_0000;
/// `HCR_HCD`: disable `hvc`.
pub const HCR_HCD: u64 = 0x0000_0000_2000_0000;
/// `HCR_TRVM`: trap the reads of the virtual memory controls.
pub const HCR_TRVM: u64 = 0x0000_0000_4000_0000;
/// `HCR_RW`: EL1 is AArch64.
pub const HCR_RW: u64 = 0x0000_0000_8000_0000;
/// `HCR_CD`: stage 2 data access cacheability disable.
pub const HCR_CD: u64 = 0x0000_0001_0000_0000;
/// `HCR_ID`: stage 2 instruction access cacheability disable.
pub const HCR_ID: u64 = 0x0000_0002_0000_0000;
/// `HCR_E2H`: EL2 host (VHE).
pub const HCR_E2H: u64 = 0x0000_0004_0000_0000;
/// `HCR_APK`: do not trap the pointer authentication key registers.
pub const HCR_APK: u64 = 0x0000_0100_0000_0000;
/// `HCR_API`: do not trap the pointer authentication instructions.
pub const HCR_API: u64 = 0x0000_0200_0000_0000;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// The C header's values, read from `$OPENBSD_SRC` (`just test-ref`).
    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn matches_reference() {
        let want = [
            ("CPTR_RES0", CPTR_RES0 as i64),
            ("CPTR_RES1", CPTR_RES1 as i64),
            ("CPTR_TFP", CPTR_TFP as i64),
            ("CPTR_TTA", CPTR_TTA as i64),
            ("CPTR_TCPAC", CPTR_TCPAC as i64),
            ("HCR_VM", HCR_VM as i64),
            ("HCR_SWIO", HCR_SWIO as i64),
            ("HCR_PTW", HCR_PTW as i64),
            ("HCR_FMO", HCR_FMO as i64),
            ("HCR_IMO", HCR_IMO as i64),
            ("HCR_AMO", HCR_AMO as i64),
            ("HCR_VF", HCR_VF as i64),
            ("HCR_VI", HCR_VI as i64),
            ("HCR_VSE", HCR_VSE as i64),
            ("HCR_FB", HCR_FB as i64),
            ("HCR_BSU_MASK", HCR_BSU_MASK as i64),
            ("HCR_DC", HCR_DC as i64),
            ("HCR_TWI", HCR_TWI as i64),
            ("HCR_TWE", HCR_TWE as i64),
            ("HCR_TID0", HCR_TID0 as i64),
            ("HCR_TID1", HCR_TID1 as i64),
            ("HCR_TID2", HCR_TID2 as i64),
            ("HCR_TID3", HCR_TID3 as i64),
            ("HCR_TSC", HCR_TSC as i64),
            ("HCR_TIDCP", HCR_TIDCP as i64),
            ("HCR_TACR", HCR_TACR as i64),
            ("HCR_TSW", HCR_TSW as i64),
            ("HCR_TPC", HCR_TPC as i64),
            ("HCR_TPU", HCR_TPU as i64),
            ("HCR_TTLB", HCR_TTLB as i64),
            ("HCR_TVM", HCR_TVM as i64),
            ("HCR_TGE", HCR_TGE as i64),
            ("HCR_TDZ", HCR_TDZ as i64),
            ("HCR_HCD", HCR_HCD as i64),
            ("HCR_TRVM", HCR_TRVM as i64),
            ("HCR_RW", HCR_RW as i64),
            ("HCR_CD", HCR_CD as i64),
            ("HCR_ID", HCR_ID as i64),
            ("HCR_E2H", HCR_E2H as i64),
            ("HCR_APK", HCR_APK as i64),
            ("HCR_API", HCR_API as i64),
        ];
        let defs = crate::reftest::defines("sys/arch/arm64/include/hypervisor.h");
        assert_eq!(
            defs.keys()
                .filter(|k| k.starts_with("CPTR_") || k.starts_with("HCR_"))
                .count(),
            want.len(),
            "a define of the header is not ported"
        );
        for (name, value) in want {
            assert_eq!(crate::reftest::int(&defs, name), Some(value), "{name}");
        }
    }
}
/* </TESTS> */
