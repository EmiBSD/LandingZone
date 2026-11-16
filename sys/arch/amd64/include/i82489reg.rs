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
 * Copyright (c) 1998 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Frank van der Linden.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! amd64 `<machine/i82489reg.h>`: registers and constants for the 82489DX and Pentium (and
//! up) integrated "local" APIC.
//!
//! Upstream: sys/arch/amd64/include/i82489reg.h @ 3ce1f3f79392
//!
//! Status: `ported`. The whole header: every register of the local APIC with the fields
//! and values the header nests under it, `LAPIC_BASE`, `LAPIC_IRQ_MASK` and the x2APIC
//! model-specific registers.
//!
//! ## Deviations
//! - A register's offset is an `i32` (what `lapic_readreg` and `lapic_writereg` take, the
//!   C's `int`), a field's mask or value a `u32` (the register's width), `LAPIC_BASE` a
//!   `usize` and an MSR's number a `u32`.
//! - `LAPIC_IRQ_MASK(i)` is a `const fn` with a lower-case name.

/// `LAPIC_ID`: ID. RW.
pub const LAPIC_ID: i32 = 0x020;
/// `LAPIC_ID_MASK`.
pub const LAPIC_ID_MASK: u32 = 0x0f00_0000;
/// `LAPIC_ID_SHIFT`.
pub const LAPIC_ID_SHIFT: u32 = 24;

/// `LAPIC_VERS`: Version. R.
pub const LAPIC_VERS: i32 = 0x030;
/// `LAPIC_VERSION_MASK`.
pub const LAPIC_VERSION_MASK: u32 = 0x0000_00ff;
/// `LAPIC_VERSION_LVT_MASK`.
pub const LAPIC_VERSION_LVT_MASK: u32 = 0x00ff_0000;
/// `LAPIC_VERSION_LVT_SHIFT`.
pub const LAPIC_VERSION_LVT_SHIFT: u32 = 16;

/// `LAPIC_TPRI`: Task Prio. RW.
pub const LAPIC_TPRI: i32 = 0x080;
/// `LAPIC_TPRI_MASK`.
pub const LAPIC_TPRI_MASK: u32 = 0x0000_00ff;
/// `LAPIC_TPRI_INT_MASK`.
pub const LAPIC_TPRI_INT_MASK: u32 = 0x0000_00f0;
/// `LAPIC_TPRI_SUB_MASK`.
pub const LAPIC_TPRI_SUB_MASK: u32 = 0x0000_000f;

/// `LAPIC_APRI`: Arbitration prio R.
pub const LAPIC_APRI: i32 = 0x090;
/// `LAPIC_APRI_MASK`.
pub const LAPIC_APRI_MASK: u32 = 0x0000_00ff;

/// `LAPIC_PPRI`: Processor prio. R.
pub const LAPIC_PPRI: i32 = 0x0a0;
/// `LAPIC_EOI`: End Int. W.
pub const LAPIC_EOI: i32 = 0x0b0;
/// `LAPIC_RRR`: Remote read R.
pub const LAPIC_RRR: i32 = 0x0c0;
/// `LAPIC_LDR`: Logical dest. RW.
pub const LAPIC_LDR: i32 = 0x0d0;
/// `LAPIC_DFR`: Dest. format RW.
pub const LAPIC_DFR: i32 = 0x0e0;

/// `LAPIC_SVR`: Spurious intvec RW.
pub const LAPIC_SVR: i32 = 0x0f0;
/// `LAPIC_SVR_VECTOR_MASK`.
pub const LAPIC_SVR_VECTOR_MASK: u32 = 0x0000_00ff;
/// `LAPIC_SVR_VEC_FIX`.
pub const LAPIC_SVR_VEC_FIX: u32 = 0x0000_000f;
/// `LAPIC_SVR_VEC_PROG`.
pub const LAPIC_SVR_VEC_PROG: u32 = 0x0000_00f0;
/// `LAPIC_SVR_ENABLE`.
pub const LAPIC_SVR_ENABLE: u32 = 0x0000_0100;
/// `LAPIC_SVR_SWEN`.
pub const LAPIC_SVR_SWEN: u32 = 0x0000_0100;
/// `LAPIC_SVR_FOCUS`.
pub const LAPIC_SVR_FOCUS: u32 = 0x0000_0200;
/// `LAPIC_SVR_FDIS`.
pub const LAPIC_SVR_FDIS: u32 = 0x0000_0200;

/// `LAPIC_ISR`: Int. status. R.
pub const LAPIC_ISR: i32 = 0x100;
/// `LAPIC_TMR`.
pub const LAPIC_TMR: i32 = 0x180;
/// `LAPIC_IRR`.
pub const LAPIC_IRR: i32 = 0x200;
/// `LAPIC_ESR`: Err status. R.
pub const LAPIC_ESR: i32 = 0x280;

/// `LAPIC_LVT_CMCI`: Corrected machine chk LVT.
pub const LAPIC_LVT_CMCI: i32 = 0x2f0;

/// `LAPIC_ICRLO`: Int. cmd. RW.
pub const LAPIC_ICRLO: i32 = 0x300;
/// `LAPIC_DLMODE_MASK`.
pub const LAPIC_DLMODE_MASK: u32 = 0x0000_0700;
/// `LAPIC_DLMODE_FIXED`.
pub const LAPIC_DLMODE_FIXED: u32 = 0x0000_0000;
/// `LAPIC_DLMODE_LOW`.
pub const LAPIC_DLMODE_LOW: u32 = 0x0000_0100;
/// `LAPIC_DLMODE_SMI`.
pub const LAPIC_DLMODE_SMI: u32 = 0x0000_0200;
/// `LAPIC_DLMODE_RR`.
pub const LAPIC_DLMODE_RR: u32 = 0x0000_0300;
/// `LAPIC_DLMODE_NMI`.
pub const LAPIC_DLMODE_NMI: u32 = 0x0000_0400;
/// `LAPIC_DLMODE_INIT`.
pub const LAPIC_DLMODE_INIT: u32 = 0x0000_0500;
/// `LAPIC_DLMODE_STARTUP`.
pub const LAPIC_DLMODE_STARTUP: u32 = 0x0000_0600;
/// `LAPIC_DLMODE_EXTINT`.
pub const LAPIC_DLMODE_EXTINT: u32 = 0x0000_0700;

/// `LAPIC_DSTMODE_LOG`.
pub const LAPIC_DSTMODE_LOG: u32 = 0x0000_0800;

/// `LAPIC_DLSTAT_BUSY`.
pub const LAPIC_DLSTAT_BUSY: u32 = 0x0000_1000;

/// `LAPIC_LVL_ASSERT`.
pub const LAPIC_LVL_ASSERT: u32 = 0x0000_4000;
/// `LAPIC_LVL_DEASSERT`.
pub const LAPIC_LVL_DEASSERT: u32 = 0x0000_0000;

/// `LAPIC_LVL_TRIG`.
pub const LAPIC_LVL_TRIG: u32 = 0x0000_8000;

/// `LAPIC_RRSTAT_MASK`.
pub const LAPIC_RRSTAT_MASK: u32 = 0x0003_0000;
/// `LAPIC_RRSTAT_INPROG`.
pub const LAPIC_RRSTAT_INPROG: u32 = 0x0001_0000;
/// `LAPIC_RRSTAT_VALID`.
pub const LAPIC_RRSTAT_VALID: u32 = 0x0002_0000;

/// `LAPIC_DEST_MASK`.
pub const LAPIC_DEST_MASK: u32 = 0x000c_0000;
/// `LAPIC_DEST_SELF`.
pub const LAPIC_DEST_SELF: u32 = 0x0004_0000;
/// `LAPIC_DEST_ALLINCL`.
pub const LAPIC_DEST_ALLINCL: u32 = 0x0008_0000;
/// `LAPIC_DEST_ALLEXCL`.
pub const LAPIC_DEST_ALLEXCL: u32 = 0x000c_0000;

/// `LAPIC_RESV2_MASK`.
pub const LAPIC_RESV2_MASK: u32 = 0xfff0_0000;

/// `LAPIC_ICRHI`: Int. cmd. RW.
pub const LAPIC_ICRHI: i32 = 0x310;

/// `LAPIC_LVTT`: Loc.vec.(timer) RW.
pub const LAPIC_LVTT: i32 = 0x320;
/// `LAPIC_LVTT_VEC_MASK`.
pub const LAPIC_LVTT_VEC_MASK: u32 = 0x0000_00ff;
/// `LAPIC_LVTT_DS`.
pub const LAPIC_LVTT_DS: u32 = 0x0000_1000;
/// `LAPIC_LVTT_M`.
pub const LAPIC_LVTT_M: u32 = 0x0001_0000;
/// `LAPIC_LVTT_TM`.
pub const LAPIC_LVTT_TM: u32 = 0x0006_0000;
/// `LAPIC_LVTT_TM_ONESHOT`.
pub const LAPIC_LVTT_TM_ONESHOT: u32 = 0x0000_0000;
/// `LAPIC_LVTT_TM_PERIODIC`.
pub const LAPIC_LVTT_TM_PERIODIC: u32 = 0x0002_0000;
/// `LAPIC_LVTT_TM_TSCDL`.
pub const LAPIC_LVTT_TM_TSCDL: u32 = 0x0004_0000;

/// `LAPIC_LVT_THERM`: Thermal sensor LVT.
pub const LAPIC_LVT_THERM: i32 = 0x330;
/// `LAPIC_PCINT`.
pub const LAPIC_PCINT: i32 = 0x340;
/// `LAPIC_LVINT0`: Loc.vec (LINT0) RW.
pub const LAPIC_LVINT0: i32 = 0x350;
/// `LAPIC_LVT_PERIODIC`.
pub const LAPIC_LVT_PERIODIC: u32 = 0x0002_0000;
/// `LAPIC_LVT_MASKED`.
pub const LAPIC_LVT_MASKED: u32 = 0x0001_0000;
/// `LAPIC_LVT_LEVTRIG`.
pub const LAPIC_LVT_LEVTRIG: u32 = 0x0000_8000;
/// `LAPIC_LVT_REMOTE_IRR`.
pub const LAPIC_LVT_REMOTE_IRR: u32 = 0x0000_4000;
/// `LAPIC_INP_POL`.
pub const LAPIC_INP_POL: u32 = 0x0000_2000;
/// `LAPIC_PEND_SEND`.
pub const LAPIC_PEND_SEND: u32 = 0x0000_1000;

/// `LAPIC_LVINT1`: Loc.vec (LINT1) RW.
pub const LAPIC_LVINT1: i32 = 0x360;
/// `LAPIC_LVERR`: Loc.vec (ERROR) RW.
pub const LAPIC_LVERR: i32 = 0x370;
/// `LAPIC_ICR_TIMER`: Initial count RW.
pub const LAPIC_ICR_TIMER: i32 = 0x380;
/// `LAPIC_CCR_TIMER`: Current count RO.
pub const LAPIC_CCR_TIMER: i32 = 0x390;

/// `LAPIC_DCR_TIMER`: Divisor config register.
pub const LAPIC_DCR_TIMER: i32 = 0x3e0;
/// `LAPIC_DCRT_DIV1`.
pub const LAPIC_DCRT_DIV1: u32 = 0x0b;
/// `LAPIC_DCRT_DIV2`.
pub const LAPIC_DCRT_DIV2: u32 = 0x00;
/// `LAPIC_DCRT_DIV4`.
pub const LAPIC_DCRT_DIV4: u32 = 0x01;
/// `LAPIC_DCRT_DIV8`.
pub const LAPIC_DCRT_DIV8: u32 = 0x02;
/// `LAPIC_DCRT_DIV16`.
pub const LAPIC_DCRT_DIV16: u32 = 0x03;
/// `LAPIC_DCRT_DIV32`.
pub const LAPIC_DCRT_DIV32: u32 = 0x08;
/// `LAPIC_DCRT_DIV64`.
pub const LAPIC_DCRT_DIV64: u32 = 0x09;
/// `LAPIC_DCRT_DIV128`.
pub const LAPIC_DCRT_DIV128: u32 = 0x0a;

/// `LAPIC_BASE`.
pub const LAPIC_BASE: usize = 0xfee0_0000;

/// `LAPIC_IRQ_MASK(i)`.
pub const fn lapic_irq_mask(i: u32) -> u32 {
    1 << (i + 1)
}

/// `MSR_X2APIC_BASE`.
pub const MSR_X2APIC_BASE: u32 = 0x800;
/// `MSR_X2APIC_END`.
pub const MSR_X2APIC_END: u32 = 0x83f;
/// `MSR_X2APIC_ID`: ID. R.
pub const MSR_X2APIC_ID: u32 = MSR_X2APIC_BASE + 0x02;
/// `MSR_X2APIC_VERSION`: Version. R.
pub const MSR_X2APIC_VERSION: u32 = MSR_X2APIC_BASE + 0x03;
/// `MSR_X2APIC_TPR`: Task prio. RW.
pub const MSR_X2APIC_TPR: u32 = MSR_X2APIC_BASE + 0x08;
/// `MSR_X2APIC_PPR`: Processor prio. R.
pub const MSR_X2APIC_PPR: u32 = MSR_X2APIC_BASE + 0x0a;
/// `MSR_X2APIC_EOI`: End Int. W.
pub const MSR_X2APIC_EOI: u32 = MSR_X2APIC_BASE + 0x0b;
/// `MSR_X2APIC_LDR`: Logical dest. R.
pub const MSR_X2APIC_LDR: u32 = MSR_X2APIC_BASE + 0x0d;
/// `MSR_X2APIC_SVR`: Spurious intvec RW.
pub const MSR_X2APIC_SVR: u32 = MSR_X2APIC_BASE + 0x0f;
/// `MSR_X2APIC_ISR0`: ISR 31:0. R.
pub const MSR_X2APIC_ISR0: u32 = MSR_X2APIC_BASE + 0x10;
/// `MSR_X2APIC_ISR7`: ISR 255:224. R.
pub const MSR_X2APIC_ISR7: u32 = MSR_X2APIC_BASE + 0x17;
/// `MSR_X2APIC_TMR0`: TMR 31:0. R.
pub const MSR_X2APIC_TMR0: u32 = MSR_X2APIC_BASE + 0x18;
/// `MSR_X2APIC_TMR7`: TMR 255:224. R.
pub const MSR_X2APIC_TMR7: u32 = MSR_X2APIC_BASE + 0x1f;
/// `MSR_X2APIC_IRR0`: IRR 31:0. R.
pub const MSR_X2APIC_IRR0: u32 = MSR_X2APIC_BASE + 0x20;
/// `MSR_X2APIC_IRR7`: IRR 255:224. R.
pub const MSR_X2APIC_IRR7: u32 = MSR_X2APIC_BASE + 0x27;
/// `MSR_X2APIC_ESR`: Error status RW.
pub const MSR_X2APIC_ESR: u32 = MSR_X2APIC_BASE + 0x28;
/// `MSR_X2APIC_LVT_CMCI`: CMCI LVT. RW.
pub const MSR_X2APIC_LVT_CMCI: u32 = MSR_X2APIC_BASE + 0x2f;
/// `MSR_X2APIC_ICR`: ICR. RW.
pub const MSR_X2APIC_ICR: u32 = MSR_X2APIC_BASE + 0x30;
/// `MSR_X2APIC_LVT_TIMER`: Timer LVT. RW.
pub const MSR_X2APIC_LVT_TIMER: u32 = MSR_X2APIC_BASE + 0x32;
/// `MSR_X2APIC_LVT_THERM`: Thermal LVT. RW.
pub const MSR_X2APIC_LVT_THERM: u32 = MSR_X2APIC_BASE + 0x33;
/// `MSR_X2APIC_LVT_PCINT`: Perf LVT. RW.
pub const MSR_X2APIC_LVT_PCINT: u32 = MSR_X2APIC_BASE + 0x34;
/// `MSR_X2APIC_LVT_LINT0`: LINT0 LVT. RW.
pub const MSR_X2APIC_LVT_LINT0: u32 = MSR_X2APIC_BASE + 0x35;
/// `MSR_X2APIC_LVT_LINT1`: LINT1 LVT. RW.
pub const MSR_X2APIC_LVT_LINT1: u32 = MSR_X2APIC_BASE + 0x36;
/// `MSR_X2APIC_LVT_ERROR`: Error LVT. RW.
pub const MSR_X2APIC_LVT_ERROR: u32 = MSR_X2APIC_BASE + 0x37;
/// `MSR_X2APIC_TIMER_ICR`: Initial count RW.
pub const MSR_X2APIC_TIMER_ICR: u32 = MSR_X2APIC_BASE + 0x38;
/// `MSR_X2APIC_TIMER_CCR`: Current count R.
pub const MSR_X2APIC_TIMER_CCR: u32 = MSR_X2APIC_BASE + 0x39;
/// `MSR_X2APIC_TIMER_DCR`: Divisor config RW.
pub const MSR_X2APIC_TIMER_DCR: u32 = MSR_X2APIC_BASE + 0x3e;
/// `MSR_X2APIC_SELF_IPI`: Self IPI. W.
pub const MSR_X2APIC_SELF_IPI: u32 = MSR_X2APIC_BASE + 0x3f;
/// `X2APIC_ID_MASK`.
pub const X2APIC_ID_MASK: u32 = 0xff;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irq_mask() {
        assert_eq!(lapic_irq_mask(0), 2);
        assert_eq!(lapic_irq_mask(3), 16);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/arch/amd64/include/i82489reg.h");
        let ours: &[(&str, i64)] = &[
            ("LAPIC_ID", LAPIC_ID as i64),
            ("LAPIC_ID_MASK", LAPIC_ID_MASK as i64),
            ("LAPIC_ID_SHIFT", LAPIC_ID_SHIFT as i64),
            ("LAPIC_VERS", LAPIC_VERS as i64),
            ("LAPIC_VERSION_MASK", LAPIC_VERSION_MASK as i64),
            ("LAPIC_VERSION_LVT_MASK", LAPIC_VERSION_LVT_MASK as i64),
            ("LAPIC_VERSION_LVT_SHIFT", LAPIC_VERSION_LVT_SHIFT as i64),
            ("LAPIC_TPRI", LAPIC_TPRI as i64),
            ("LAPIC_TPRI_MASK", LAPIC_TPRI_MASK as i64),
            ("LAPIC_TPRI_INT_MASK", LAPIC_TPRI_INT_MASK as i64),
            ("LAPIC_TPRI_SUB_MASK", LAPIC_TPRI_SUB_MASK as i64),
            ("LAPIC_APRI", LAPIC_APRI as i64),
            ("LAPIC_APRI_MASK", LAPIC_APRI_MASK as i64),
            ("LAPIC_PPRI", LAPIC_PPRI as i64),
            ("LAPIC_EOI", LAPIC_EOI as i64),
            ("LAPIC_RRR", LAPIC_RRR as i64),
            ("LAPIC_LDR", LAPIC_LDR as i64),
            ("LAPIC_DFR", LAPIC_DFR as i64),
            ("LAPIC_SVR", LAPIC_SVR as i64),
            ("LAPIC_SVR_VECTOR_MASK", LAPIC_SVR_VECTOR_MASK as i64),
            ("LAPIC_SVR_VEC_FIX", LAPIC_SVR_VEC_FIX as i64),
            ("LAPIC_SVR_VEC_PROG", LAPIC_SVR_VEC_PROG as i64),
            ("LAPIC_SVR_ENABLE", LAPIC_SVR_ENABLE as i64),
            ("LAPIC_SVR_SWEN", LAPIC_SVR_SWEN as i64),
            ("LAPIC_SVR_FOCUS", LAPIC_SVR_FOCUS as i64),
            ("LAPIC_SVR_FDIS", LAPIC_SVR_FDIS as i64),
            ("LAPIC_ISR", LAPIC_ISR as i64),
            ("LAPIC_TMR", LAPIC_TMR as i64),
            ("LAPIC_IRR", LAPIC_IRR as i64),
            ("LAPIC_ESR", LAPIC_ESR as i64),
            ("LAPIC_LVT_CMCI", LAPIC_LVT_CMCI as i64),
            ("LAPIC_ICRLO", LAPIC_ICRLO as i64),
            ("LAPIC_DLMODE_MASK", LAPIC_DLMODE_MASK as i64),
            ("LAPIC_DLMODE_FIXED", LAPIC_DLMODE_FIXED as i64),
            ("LAPIC_DLMODE_LOW", LAPIC_DLMODE_LOW as i64),
            ("LAPIC_DLMODE_SMI", LAPIC_DLMODE_SMI as i64),
            ("LAPIC_DLMODE_RR", LAPIC_DLMODE_RR as i64),
            ("LAPIC_DLMODE_NMI", LAPIC_DLMODE_NMI as i64),
            ("LAPIC_DLMODE_INIT", LAPIC_DLMODE_INIT as i64),
            ("LAPIC_DLMODE_STARTUP", LAPIC_DLMODE_STARTUP as i64),
            ("LAPIC_DLMODE_EXTINT", LAPIC_DLMODE_EXTINT as i64),
            ("LAPIC_DSTMODE_LOG", LAPIC_DSTMODE_LOG as i64),
            ("LAPIC_DLSTAT_BUSY", LAPIC_DLSTAT_BUSY as i64),
            ("LAPIC_LVL_ASSERT", LAPIC_LVL_ASSERT as i64),
            ("LAPIC_LVL_DEASSERT", LAPIC_LVL_DEASSERT as i64),
            ("LAPIC_LVL_TRIG", LAPIC_LVL_TRIG as i64),
            ("LAPIC_RRSTAT_MASK", LAPIC_RRSTAT_MASK as i64),
            ("LAPIC_RRSTAT_INPROG", LAPIC_RRSTAT_INPROG as i64),
            ("LAPIC_RRSTAT_VALID", LAPIC_RRSTAT_VALID as i64),
            ("LAPIC_DEST_MASK", LAPIC_DEST_MASK as i64),
            ("LAPIC_DEST_SELF", LAPIC_DEST_SELF as i64),
            ("LAPIC_DEST_ALLINCL", LAPIC_DEST_ALLINCL as i64),
            ("LAPIC_DEST_ALLEXCL", LAPIC_DEST_ALLEXCL as i64),
            ("LAPIC_RESV2_MASK", LAPIC_RESV2_MASK as i64),
            ("LAPIC_ICRHI", LAPIC_ICRHI as i64),
            ("LAPIC_LVTT", LAPIC_LVTT as i64),
            ("LAPIC_LVTT_VEC_MASK", LAPIC_LVTT_VEC_MASK as i64),
            ("LAPIC_LVTT_DS", LAPIC_LVTT_DS as i64),
            ("LAPIC_LVTT_M", LAPIC_LVTT_M as i64),
            ("LAPIC_LVTT_TM", LAPIC_LVTT_TM as i64),
            ("LAPIC_LVTT_TM_ONESHOT", LAPIC_LVTT_TM_ONESHOT as i64),
            ("LAPIC_LVTT_TM_PERIODIC", LAPIC_LVTT_TM_PERIODIC as i64),
            ("LAPIC_LVTT_TM_TSCDL", LAPIC_LVTT_TM_TSCDL as i64),
            ("LAPIC_LVT_THERM", LAPIC_LVT_THERM as i64),
            ("LAPIC_PCINT", LAPIC_PCINT as i64),
            ("LAPIC_LVINT0", LAPIC_LVINT0 as i64),
            ("LAPIC_LVT_PERIODIC", LAPIC_LVT_PERIODIC as i64),
            ("LAPIC_LVT_MASKED", LAPIC_LVT_MASKED as i64),
            ("LAPIC_LVT_LEVTRIG", LAPIC_LVT_LEVTRIG as i64),
            ("LAPIC_LVT_REMOTE_IRR", LAPIC_LVT_REMOTE_IRR as i64),
            ("LAPIC_INP_POL", LAPIC_INP_POL as i64),
            ("LAPIC_PEND_SEND", LAPIC_PEND_SEND as i64),
            ("LAPIC_LVINT1", LAPIC_LVINT1 as i64),
            ("LAPIC_LVERR", LAPIC_LVERR as i64),
            ("LAPIC_ICR_TIMER", LAPIC_ICR_TIMER as i64),
            ("LAPIC_CCR_TIMER", LAPIC_CCR_TIMER as i64),
            ("LAPIC_DCR_TIMER", LAPIC_DCR_TIMER as i64),
            ("LAPIC_DCRT_DIV1", LAPIC_DCRT_DIV1 as i64),
            ("LAPIC_DCRT_DIV2", LAPIC_DCRT_DIV2 as i64),
            ("LAPIC_DCRT_DIV4", LAPIC_DCRT_DIV4 as i64),
            ("LAPIC_DCRT_DIV8", LAPIC_DCRT_DIV8 as i64),
            ("LAPIC_DCRT_DIV16", LAPIC_DCRT_DIV16 as i64),
            ("LAPIC_DCRT_DIV32", LAPIC_DCRT_DIV32 as i64),
            ("LAPIC_DCRT_DIV64", LAPIC_DCRT_DIV64 as i64),
            ("LAPIC_DCRT_DIV128", LAPIC_DCRT_DIV128 as i64),
            ("LAPIC_BASE", LAPIC_BASE as i64),
            ("MSR_X2APIC_BASE", MSR_X2APIC_BASE as i64),
            ("MSR_X2APIC_END", MSR_X2APIC_END as i64),
            ("MSR_X2APIC_ID", MSR_X2APIC_ID as i64),
            ("MSR_X2APIC_VERSION", MSR_X2APIC_VERSION as i64),
            ("MSR_X2APIC_TPR", MSR_X2APIC_TPR as i64),
            ("MSR_X2APIC_PPR", MSR_X2APIC_PPR as i64),
            ("MSR_X2APIC_EOI", MSR_X2APIC_EOI as i64),
            ("MSR_X2APIC_LDR", MSR_X2APIC_LDR as i64),
            ("MSR_X2APIC_SVR", MSR_X2APIC_SVR as i64),
            ("MSR_X2APIC_ISR0", MSR_X2APIC_ISR0 as i64),
            ("MSR_X2APIC_ISR7", MSR_X2APIC_ISR7 as i64),
            ("MSR_X2APIC_TMR0", MSR_X2APIC_TMR0 as i64),
            ("MSR_X2APIC_TMR7", MSR_X2APIC_TMR7 as i64),
            ("MSR_X2APIC_IRR0", MSR_X2APIC_IRR0 as i64),
            ("MSR_X2APIC_IRR7", MSR_X2APIC_IRR7 as i64),
            ("MSR_X2APIC_ESR", MSR_X2APIC_ESR as i64),
            ("MSR_X2APIC_LVT_CMCI", MSR_X2APIC_LVT_CMCI as i64),
            ("MSR_X2APIC_ICR", MSR_X2APIC_ICR as i64),
            ("MSR_X2APIC_LVT_TIMER", MSR_X2APIC_LVT_TIMER as i64),
            ("MSR_X2APIC_LVT_THERM", MSR_X2APIC_LVT_THERM as i64),
            ("MSR_X2APIC_LVT_PCINT", MSR_X2APIC_LVT_PCINT as i64),
            ("MSR_X2APIC_LVT_LINT0", MSR_X2APIC_LVT_LINT0 as i64),
            ("MSR_X2APIC_LVT_LINT1", MSR_X2APIC_LVT_LINT1 as i64),
            ("MSR_X2APIC_LVT_ERROR", MSR_X2APIC_LVT_ERROR as i64),
            ("MSR_X2APIC_TIMER_ICR", MSR_X2APIC_TIMER_ICR as i64),
            ("MSR_X2APIC_TIMER_CCR", MSR_X2APIC_TIMER_CCR as i64),
            ("MSR_X2APIC_TIMER_DCR", MSR_X2APIC_TIMER_DCR as i64),
            ("MSR_X2APIC_SELF_IPI", MSR_X2APIC_SELF_IPI as i64),
            ("X2APIC_ID_MASK", X2APIC_ID_MASK as i64),
        ];
        assert_eq!(
            defs.len(),
            ours.len(),
            "a define of the header is not ported"
        );
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
