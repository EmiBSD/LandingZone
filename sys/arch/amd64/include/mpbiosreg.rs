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
 * Copyright (c) 2000 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by RedBack Networks Inc.
 *
 * Author: Bill Sommerfeld
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
//! amd64 `<machine/mpbiosreg.h>`: the Intel MultiProcessor Specification's table formats,
//! whose interrupt-entry flags the ACPI MADT and `_PRT` reuse.
//!
//! Upstream: sys/arch/amd64/include/mpbiosreg.h @ 3ce1f3f79392
//!
//! Status: `ported`. The whole header: where the BIOS is mapped (`BIOS_*`), the types of a
//! configuration table entry (`MPS_MCT_*`), the interrupt types and flags (`MPS_INTTYPE_*`,
//! `MPS_INTPO_*`, `MPS_INTTR_*`, `MPS_INT`), and the six structures of the tables with the
//! constants the header defines among their members: the floating pointer structure
//! (`struct mpbios_fps`), the configuration table header (`mpbios_cth`) and the processor,
//! bus, I/O APIC and interrupt entries. `acpimadt`, `acpiprt`, `ioapic.c` and `lapic.c` use
//! the interrupt flags; the structures are read by `mpbios.c`, which is not ported (TD6).
//!
//! ## Deviations
//! - `MPS_INT(p,t)` is a `const fn` with a lower-case name.
//! - The structures are `#[repr(C)]` with the C's member names; their sizes, which the
//!   MultiProcessor Specification fixes, are checked at compile time.

/// `BIOS_BASE`: where the BIOS ROM is mapped.
pub const BIOS_BASE: usize = 0xf0000;
/// `BIOS_SIZE`: its size.
pub const BIOS_SIZE: usize = 0x10000;
/// `BIOS_COUNT`.
pub const BIOS_COUNT: usize = BIOS_SIZE;

// Multiprocessor config table entry types.

/// `MPS_MCT_CPU`: a processor entry.
pub const MPS_MCT_CPU: i32 = 0;
/// `MPS_MCT_BUS`: a bus entry.
pub const MPS_MCT_BUS: i32 = 1;
/// `MPS_MCT_IOAPIC`: an I/O APIC entry.
pub const MPS_MCT_IOAPIC: i32 = 2;
/// `MPS_MCT_IOINT`: an I/O interrupt assignment entry.
pub const MPS_MCT_IOINT: i32 = 3;
/// `MPS_MCT_LINT`: a local interrupt assignment entry.
pub const MPS_MCT_LINT: i32 = 4;
/// `MPS_MCT_NTYPES`: how many entry types there are.
pub const MPS_MCT_NTYPES: i32 = 5;

// Interrupt types

/// `MPS_INTTYPE_INT`.
pub const MPS_INTTYPE_INT: i32 = 0;
/// `MPS_INTTYPE_NMI`.
pub const MPS_INTTYPE_NMI: i32 = 1;
/// `MPS_INTTYPE_SMI`.
pub const MPS_INTTYPE_SMI: i32 = 2;
/// `MPS_INTTYPE_ExtINT`.
#[allow(non_upper_case_globals)] // the C name
pub const MPS_INTTYPE_ExtINT: i32 = 3;

/// `MPS_INTPO_DEF`: polarity conforms to the bus.
pub const MPS_INTPO_DEF: i32 = 0;
/// `MPS_INTPO_ACTHI`.
pub const MPS_INTPO_ACTHI: i32 = 1;
/// `MPS_INTPO_ACTLO`.
pub const MPS_INTPO_ACTLO: i32 = 3;
/// `MPS_INTPO_SHIFT`.
pub const MPS_INTPO_SHIFT: i32 = 0;
/// `MPS_INTPO_MASK`.
pub const MPS_INTPO_MASK: i32 = 3;

/// `MPS_INTTR_DEF`: trigger mode conforms to the bus.
pub const MPS_INTTR_DEF: i32 = 0;
/// `MPS_INTTR_EDGE`.
pub const MPS_INTTR_EDGE: i32 = 1;
/// `MPS_INTTR_LEVEL`.
pub const MPS_INTTR_LEVEL: i32 = 3;
/// `MPS_INTTR_SHIFT`.
pub const MPS_INTTR_SHIFT: i32 = 2;
/// `MPS_INTTR_MASK`.
pub const MPS_INTTR_MASK: i32 = 3;

/// `MPS_INT(p,t)`: the flags of polarity `p` and trigger mode `t`.
pub const fn mps_int(p: i32, t: i32) -> i32 {
    ((p & MPS_INTPO_MASK) << MPS_INTPO_SHIFT) | ((t & MPS_INTTR_MASK) << MPS_INTTR_SHIFT)
}

/// `MPS_ALL_APICS`: an interrupt entry for every (local or I/O) APIC.
pub const MPS_ALL_APICS: i32 = 0xff;

/// `MP_FP_SIG`: string defined by the Intel MP Spec as identifying the MP table (`_MP_`).
pub const MP_FP_SIG: u32 = 0x5f50_4d5f;
/// `MPFPS_FLAG_IMCR`: IMCR present (`mpfb2`).
pub const MPFPS_FLAG_IMCR: u8 = 0x80;

/// `struct mpbios_fps`: MP Floating Pointer Structure.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MpbiosFps {
    /// `signature`: [`MP_FP_SIG`].
    pub signature: u32,
    /// `pap`: the physical address of the configuration table.
    pub pap: u32,
    /// `length`.
    pub length: u8,
    /// `spec_rev`.
    pub spec_rev: u8,
    /// `checksum`.
    pub checksum: u8,
    /// `mpfb1`: system configuration.
    pub mpfb1: u8,
    /// `mpfb2`: flags.
    pub mpfb2: u8,
    /// `mpfb3`: unused.
    pub mpfb3: u8,
    /// `mpfb4`: unused.
    pub mpfb4: u8,
    /// `mpfb5`: unused.
    pub mpfb5: u8,
}

/// `MP_CT_SIG`: the configuration table's signature (`PCMP`).
pub const MP_CT_SIG: u32 = 0x504d_4350;

/// `struct mpbios_cth`: MP Configuration Table Header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MpbiosCth {
    /// `signature`: [`MP_CT_SIG`].
    pub signature: u32,
    /// `base_len`.
    pub base_len: u16,
    /// `spec_rev`.
    pub spec_rev: u8,
    /// `checksum`.
    pub checksum: u8,
    /// `oem_id`.
    pub oem_id: [u8; 8],
    /// `product_id`.
    pub product_id: [u8; 12],
    /// `oem_table_pointer`.
    pub oem_table_pointer: u32,
    /// `oem_table_size`.
    pub oem_table_size: u16,
    /// `entry_count`.
    pub entry_count: u16,
    /// `apic_address`.
    pub apic_address: u32,
    /// `ext_len`.
    pub ext_len: u16,
    /// `ext_cksum`.
    pub ext_cksum: u8,
    /// `reserved`.
    pub reserved: u8,
}

/// `PROCENTRY_FLAG_EN`: the processor is usable (`cpu_flags`).
pub const PROCENTRY_FLAG_EN: u8 = 0x01;
/// `PROCENTRY_FLAG_BP`: the processor is the bootstrap processor (`cpu_flags`).
pub const PROCENTRY_FLAG_BP: u8 = 0x02;

/// `struct mpbios_proc`: a processor entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MpbiosProc {
    /// `type`: [`MPS_MCT_CPU`].
    pub type_: u8,
    /// `apic_id`.
    pub apic_id: u8,
    /// `apic_version`.
    pub apic_version: u8,
    /// `cpu_flags`: `PROCENTRY_FLAG_*`.
    pub cpu_flags: u8,
    /// `cpu_signature`.
    pub cpu_signature: u32,
    /// `feature_flags`.
    pub feature_flags: u32,
    /// `reserved1`.
    pub reserved1: u32,
    /// `reserved2`.
    pub reserved2: u32,
}

/// `struct mpbios_bus`: a bus entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MpbiosBus {
    /// `type`: [`MPS_MCT_BUS`].
    pub type_: u8,
    /// `bus_id`.
    pub bus_id: u8,
    /// `bus_type`: the bus's name, padded with spaces.
    pub bus_type: [u8; 6],
}

/// `IOAPICENTRY_FLAG_EN`: the I/O APIC is usable (`apic_flags`).
pub const IOAPICENTRY_FLAG_EN: u8 = 0x01;

/// `struct mpbios_ioapic`: an I/O APIC entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MpbiosIoapic {
    /// `type`: [`MPS_MCT_IOAPIC`].
    pub type_: u8,
    /// `apic_id`.
    pub apic_id: u8,
    /// `apic_version`.
    pub apic_version: u8,
    /// `apic_flags`: `IOAPICENTRY_FLAG_*`.
    pub apic_flags: u8,
    /// `apic_address`.
    pub apic_address: u32,
}

/// `struct mpbios_int`: an interrupt assignment entry, I/O or local.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MpbiosInt {
    /// `type`: [`MPS_MCT_IOINT`] or [`MPS_MCT_LINT`].
    pub type_: u8,
    /// `int_type`: `MPS_INTTYPE_*`.
    pub int_type: u8,
    /// `int_flags`: [`mps_int`]'s polarity and trigger.
    pub int_flags: u16,
    /// `src_bus_id`.
    pub src_bus_id: u8,
    /// `src_bus_irq`.
    pub src_bus_irq: u8,
    /// `dst_apic_id`: an APIC's id, or [`MPS_ALL_APICS`].
    pub dst_apic_id: u8,
    /// `dst_apic_int`.
    pub dst_apic_int: u8,
}

// The sizes the MultiProcessor Specification gives the structures.
const _: () = {
    assert!(size_of::<MpbiosFps>() == 16);
    assert!(size_of::<MpbiosCth>() == 44);
    assert!(size_of::<MpbiosProc>() == 20);
    assert!(size_of::<MpbiosBus>() == 8);
    assert!(size_of::<MpbiosIoapic>() == 8);
    assert!(size_of::<MpbiosInt>() == 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_flags() {
        assert_eq!(mps_int(MPS_INTPO_ACTLO, MPS_INTTR_LEVEL), 0xf);
        assert_eq!(mps_int(MPS_INTPO_ACTHI, MPS_INTTR_EDGE), 0x5);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/arch/amd64/include/mpbiosreg.h");
        let ours: &[(&str, i64)] = &[
            ("MPS_INTTYPE_ExtINT", i64::from(MPS_INTTYPE_ExtINT)),
            ("MPS_INTPO_ACTLO", i64::from(MPS_INTPO_ACTLO)),
            ("MPS_INTTR_LEVEL", i64::from(MPS_INTTR_LEVEL)),
            ("MPS_INTTR_SHIFT", i64::from(MPS_INTTR_SHIFT)),
            ("MPS_ALL_APICS", i64::from(MPS_ALL_APICS)),
            ("MPS_MCT_CPU", i64::from(MPS_MCT_CPU)),
            ("MPS_MCT_BUS", i64::from(MPS_MCT_BUS)),
            ("MPS_MCT_IOAPIC", i64::from(MPS_MCT_IOAPIC)),
            ("MPS_MCT_IOINT", i64::from(MPS_MCT_IOINT)),
            ("MPS_MCT_LINT", i64::from(MPS_MCT_LINT)),
            ("MPS_MCT_NTYPES", i64::from(MPS_MCT_NTYPES)),
            ("BIOS_BASE", BIOS_BASE as i64),
            ("BIOS_SIZE", BIOS_SIZE as i64),
            ("MP_FP_SIG", i64::from(MP_FP_SIG)),
            ("MP_CT_SIG", i64::from(MP_CT_SIG)),
            ("MPFPS_FLAG_IMCR", i64::from(MPFPS_FLAG_IMCR)),
            ("PROCENTRY_FLAG_EN", i64::from(PROCENTRY_FLAG_EN)),
            ("PROCENTRY_FLAG_BP", i64::from(PROCENTRY_FLAG_BP)),
            ("IOAPICENTRY_FLAG_EN", i64::from(IOAPICENTRY_FLAG_EN)),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
