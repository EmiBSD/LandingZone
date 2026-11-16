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
//! amd64 `<machine/pic.h>`: structure common to all PIC softcs.
//!
//! Upstream: sys/arch/amd64/include/pic.h @ 3ce1f3f79392
//!
//! Status: `ported`. `struct pic` and the `PIC_*` types. The four pics the header declares
//! are defined where the C defines them: `i8259_pic` in `amd64/i8259.rs` (`I8259_PIC`),
//! `local_pic` in `amd64/lapic.rs` (`LOCAL_PIC`), `msi_pic` in `pci/pci_machdep.rs`
//! (`MSI_PIC`) and `softintr_pic` in `amd64/intr.rs` (`SOFTINTR_PIC`).
//!
//! ## Deviations
//! - `pic_dev` (`struct device`, autoconfiguration) is reduced to its name, `pic_name`, which
//!   is all the C's static pics set of it and all anybody reads through the `pic_name` macro.
//!   The one pic that is a device, an I/O APIC's, keeps its `struct device` beside the pic in
//!   its softc (`i82093var.rs`).
//! - The stub tables live in assembly (`vector.S`), so they are reached through a function
//!   returning the slice instead of a pointer in the struct.

use crate::arch::amd64::include::cpu::CpuInfo;
use crate::arch::amd64::include::intr::Intrstub;
#[cfg(feature = "multiprocessor")]
use crate::sys::mutex::Mutex;

/// `pic_addroute`/`pic_delroute`: `(pic, ci, pin, idtvec, type)`.
pub type PicRouteFn = fn(&Pic, &CpuInfo, i32, i32, i32);
/// `pic_allocidtvec`: `(pic, pin, low, high)`, 0 when none is free.
pub type PicAllocIdtvecFn = fn(&Pic, i32, i32, i32) -> i32;

/// `struct pic`.
pub struct Pic {
    /// `pic_dev.dv_xname`.
    pub pic_name: &'static str,
    /// `pic_type`: `PIC_*`.
    pub pic_type: i32,
    /// `pic_mutex` (`MULTIPROCESSOR`): `ioapic_lock` takes it around an I/O APIC's registers,
    /// and acpidmar(4) initialises its own pic's. A pic that never locks keeps the free
    /// `IPL_NONE` mutex its initialiser gives it, the C's zeroes.
    #[cfg(feature = "multiprocessor")]
    pub pic_mutex: Mutex,
    /// `pic_hwmask(pic, pin)`.
    pub pic_hwmask: Option<fn(&Pic, i32)>,
    /// `pic_hwunmask(pic, pin)`.
    pub pic_hwunmask: Option<fn(&Pic, i32)>,
    /// `pic_addroute(pic, ci, pin, idtvec, type)`.
    pub pic_addroute: Option<PicRouteFn>,
    /// `pic_delroute(pic, ci, pin, idtvec, type)`.
    pub pic_delroute: Option<PicRouteFn>,
    /// `pic_allocidtvec(pic, pin, low, high)`.
    pub pic_allocidtvec: Option<PicAllocIdtvecFn>,
    /// `pic_level_stubs`: the stubs for level-triggered pins.
    pub pic_level_stubs: Option<fn() -> &'static [Intrstub]>,
    /// `pic_edge_stubs`: the stubs for edge-triggered pins.
    pub pic_edge_stubs: Option<fn() -> &'static [Intrstub]>,
}

/// `PIC_I8259`.
pub const PIC_I8259: i32 = 0;
/// `PIC_IOAPIC`.
pub const PIC_IOAPIC: i32 = 1;
/// `PIC_LAPIC`.
pub const PIC_LAPIC: i32 = 2;
/// `PIC_MSI`.
pub const PIC_MSI: i32 = 3;
/// `PIC_SOFT`.
pub const PIC_SOFT: i32 = 4;
/* </CODE> */
