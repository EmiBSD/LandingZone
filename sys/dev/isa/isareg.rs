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
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * William Jolitz.
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
 *	@(#)isa.h	5.7 (Berkeley) 5/9/91
 */
/* </LICENSES> */

/* <CODE> */
//! ISA bus conventions: `<dev/isa/isareg.h>`.
//!
//! Upstream: sys/dev/isa/isareg.h @ 3ce1f3f79392
//!
//! Status: `ported`. The whole header: the input/output port assignments, the port sizes
//! and the physical addresses of the I/O memory hole.
//!
//! ## Deviations
//! - The `#ifndef` guards around the three groups (`IO_ISABEGIN`, `IO_ISASIZES`,
//!   `IOM_BEGIN`), which let a machine header define its own, are not carried: amd64 has
//!   none. `IO_ISASIZES`, defined empty as the second group's guard, has no counterpart.
//! - Ports and port sizes are `u16`, the memory addresses `usize`.

// ISA Bus conventions

// Input / Output Port Assignments

/// `IO_ISABEGIN`: Beginning of I/O Registers.
pub const IO_ISABEGIN: u16 = 0x000;
// CPU Board
/// `IO_DMA1`: 8237A DMA Controller #1.
pub const IO_DMA1: u16 = 0x000;
/// `IO_ICU1`: 8259A Interrupt Controller #1.
pub const IO_ICU1: u16 = 0x020;
/// `IO_PMP1`: 82347 Power Management Peripheral.
pub const IO_PMP1: u16 = 0x026;
/// `IO_TIMER1`: 8253 Timer #1.
pub const IO_TIMER1: u16 = 0x040;
/// `IO_TIMER2`: 8253 Timer #2 (EISA only).
pub const IO_TIMER2: u16 = 0x048;
/// `IO_KBD`: 8042 Keyboard.
pub const IO_KBD: u16 = 0x060;
/// `IO_PPI`: Programmable Peripheral Interface.
pub const IO_PPI: u16 = 0x061;
/// `IO_RTC`: RTC.
pub const IO_RTC: u16 = 0x070;
/// `IO_NMI`: NMI Control.
pub const IO_NMI: u16 = IO_RTC;
/// `IO_DMAPG`: DMA Page Registers.
pub const IO_DMAPG: u16 = 0x080;
/// `IO_ICU2`: 8259A Interrupt Controller #2.
pub const IO_ICU2: u16 = 0x0a0;
/// `IO_DMA2`: 8237A DMA Controller #2.
pub const IO_DMA2: u16 = 0x0c0;
/// `IO_NPX`: Numeric Coprocessor.
pub const IO_NPX: u16 = 0x0f0;
// Cards
// 0x100 - 0x16F Open
/// `IO_WD2`: Secondary Fixed Disk Controller.
pub const IO_WD2: u16 = 0x170;
/// `IO_PMP2`: 82347 Power Management Peripheral.
pub const IO_PMP2: u16 = 0x178;
// 0x17A - 0x1EF Open
/// `IO_WD1`: Primary Fixed Disk Controller.
pub const IO_WD1: u16 = 0x1f0;
/// `IO_GAME`: Game Controller.
pub const IO_GAME: u16 = 0x200;
// 0x208 - 0x237 Open
/// `IO_BMS2`: Secondary InPort Bus Mouse.
pub const IO_BMS2: u16 = 0x238;
/// `IO_BMS1`: Primary InPort Bus Mouse.
pub const IO_BMS1: u16 = 0x23c;
// 0x240 - 0x277 Open
/// `IO_LPT2`: Parallel Port #2.
pub const IO_LPT2: u16 = 0x278;
// 0x280 - 0x2E7 Open
/// `IO_COM4`: COM4 i/o address.
pub const IO_COM4: u16 = 0x2e8;
// 0x2F0 - 0x2F7 Open
/// `IO_COM2`: COM2 i/o address.
pub const IO_COM2: u16 = 0x2f8;
// 0x300 - 0x32F Open
/// `IO_BT0`: Bustek 742a default addr.
pub const IO_BT0: u16 = 0x330;
/// `IO_AHA0`: Adaptec 1542 default addr.
pub const IO_AHA0: u16 = 0x330;
/// `IO_UHA0`: Ultrastore 14f default addr.
pub const IO_UHA0: u16 = 0x330;
/// `IO_BT1`: Bustek 742a default addr.
pub const IO_BT1: u16 = 0x334;
/// `IO_AHA1`: Adaptec 1542 default addr.
pub const IO_AHA1: u16 = 0x334;
// 0x338 - 0x34F Open
/// `IO_WDS`: WD7000 scsi.
pub const IO_WDS: u16 = 0x350;
// 0x354 - 0x36F Open
/// `IO_FD2`: Secondary base i/o address.
pub const IO_FD2: u16 = 0x370;
/// `IO_LPT1`: Parallel Port #1.
pub const IO_LPT1: u16 = 0x378;
// 0x380 - 0x3AF Open
/// `IO_MDA`: Monochrome Adapter.
pub const IO_MDA: u16 = 0x3b0;
/// `IO_LPT3`: Monochrome Adapter Printer Port.
pub const IO_LPT3: u16 = 0x3bc;
/// `IO_VGA`: E/VGA Ports.
pub const IO_VGA: u16 = 0x3c0;
/// `IO_CGA`: CGA Ports.
pub const IO_CGA: u16 = 0x3d0;
// 0x3E0 - 0x3E7 Open
/// `IO_COM3`: COM3 i/o address.
pub const IO_COM3: u16 = 0x3e8;
/// `IO_FD1`: Primary base i/o address.
pub const IO_FD1: u16 = 0x3f0;
/// `IO_COM1`: COM1 i/o address.
pub const IO_COM1: u16 = 0x3f8;
/// `IO_ISAEND`: End of I/O Registers.
pub const IO_ISAEND: u16 = 0x3ff;

// Input / Output Port Sizes - these are from several sources, and tend
// to be the larger of what was found, ie COM ports can be 4, but some
// boards do not fully decode the address, thus 8 ports are used.

/// `IO_COMSIZE`: 8250, 16X50 com controllers.
pub const IO_COMSIZE: u16 = 8;
/// `IO_CGASIZE`: CGA controllers.
pub const IO_CGASIZE: u16 = 16;
/// `IO_DMASIZE`: 8237 DMA controllers.
pub const IO_DMASIZE: u16 = 16;
/// `IO_DPGSIZE`: 74LS612 DMA page registers.
pub const IO_DPGSIZE: u16 = 32;
/// `IO_FDCSIZE`: Nec765 floppy controllers.
pub const IO_FDCSIZE: u16 = 8;
/// `IO_WDCSIZE`: WD compatible disk controller.
pub const IO_WDCSIZE: u16 = 8;
/// `IO_GAMSIZE`: AT compatible game controller.
pub const IO_GAMSIZE: u16 = 16;
/// `IO_ICUSIZE`: 8259A interrupt controllers.
pub const IO_ICUSIZE: u16 = 16;
/// `IO_KBDSIZE`: 8042 Keyboard controllers.
pub const IO_KBDSIZE: u16 = 16;
/// `IO_LPTSIZE`: LPT controllers.
pub const IO_LPTSIZE: u16 = 8;
/// `IO_MDASIZE`: Monochrome display controller.
pub const IO_MDASIZE: u16 = 16;
/// `IO_RTCSIZE`: CMOS real time clock, NMI con.
pub const IO_RTCSIZE: u16 = 16;
/// `IO_TMRSIZE`: 8253 programmable timers.
pub const IO_TMRSIZE: u16 = 16;
/// `IO_NPXSIZE`: 80387/80487 NPX registers.
pub const IO_NPXSIZE: u16 = 16;
/// `IO_VGASIZE`: VGA controllers.
pub const IO_VGASIZE: u16 = 16;
/// `IO_PMPSIZE`: 82347 Power Management Peripheral.
pub const IO_PMPSIZE: u16 = 2;

// Input / Output Memory Physical Addresses

/// `IOM_BEGIN`: Start of I/O Memory "hole".
pub const IOM_BEGIN: usize = 0x0a0000;
/// `IOM_END`: End of I/O Memory "hole".
pub const IOM_END: usize = 0x100000;
/// `IOM_SIZE`: The size of the I/O memory hole.
pub const IOM_SIZE: usize = IOM_END - IOM_BEGIN;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/dev/isa/isareg.h");
        let ours: &[(&str, i64)] = &[
            ("IO_ISABEGIN", IO_ISABEGIN as i64),
            ("IO_DMA1", IO_DMA1 as i64),
            ("IO_ICU1", IO_ICU1 as i64),
            ("IO_PMP1", IO_PMP1 as i64),
            ("IO_TIMER1", IO_TIMER1 as i64),
            ("IO_TIMER2", IO_TIMER2 as i64),
            ("IO_KBD", IO_KBD as i64),
            ("IO_PPI", IO_PPI as i64),
            ("IO_RTC", IO_RTC as i64),
            ("IO_NMI", IO_NMI as i64),
            ("IO_DMAPG", IO_DMAPG as i64),
            ("IO_ICU2", IO_ICU2 as i64),
            ("IO_DMA2", IO_DMA2 as i64),
            ("IO_NPX", IO_NPX as i64),
            ("IO_WD2", IO_WD2 as i64),
            ("IO_PMP2", IO_PMP2 as i64),
            ("IO_WD1", IO_WD1 as i64),
            ("IO_GAME", IO_GAME as i64),
            ("IO_BMS2", IO_BMS2 as i64),
            ("IO_BMS1", IO_BMS1 as i64),
            ("IO_LPT2", IO_LPT2 as i64),
            ("IO_COM4", IO_COM4 as i64),
            ("IO_COM2", IO_COM2 as i64),
            ("IO_BT0", IO_BT0 as i64),
            ("IO_AHA0", IO_AHA0 as i64),
            ("IO_UHA0", IO_UHA0 as i64),
            ("IO_BT1", IO_BT1 as i64),
            ("IO_AHA1", IO_AHA1 as i64),
            ("IO_WDS", IO_WDS as i64),
            ("IO_FD2", IO_FD2 as i64),
            ("IO_LPT1", IO_LPT1 as i64),
            ("IO_MDA", IO_MDA as i64),
            ("IO_LPT3", IO_LPT3 as i64),
            ("IO_VGA", IO_VGA as i64),
            ("IO_CGA", IO_CGA as i64),
            ("IO_COM3", IO_COM3 as i64),
            ("IO_FD1", IO_FD1 as i64),
            ("IO_COM1", IO_COM1 as i64),
            ("IO_ISAEND", IO_ISAEND as i64),
            ("IO_COMSIZE", IO_COMSIZE as i64),
            ("IO_CGASIZE", IO_CGASIZE as i64),
            ("IO_DMASIZE", IO_DMASIZE as i64),
            ("IO_DPGSIZE", IO_DPGSIZE as i64),
            ("IO_FDCSIZE", IO_FDCSIZE as i64),
            ("IO_WDCSIZE", IO_WDCSIZE as i64),
            ("IO_GAMSIZE", IO_GAMSIZE as i64),
            ("IO_ICUSIZE", IO_ICUSIZE as i64),
            ("IO_KBDSIZE", IO_KBDSIZE as i64),
            ("IO_LPTSIZE", IO_LPTSIZE as i64),
            ("IO_MDASIZE", IO_MDASIZE as i64),
            ("IO_RTCSIZE", IO_RTCSIZE as i64),
            ("IO_TMRSIZE", IO_TMRSIZE as i64),
            ("IO_NPXSIZE", IO_NPXSIZE as i64),
            ("IO_VGASIZE", IO_VGASIZE as i64),
            ("IO_PMPSIZE", IO_PMPSIZE as i64),
            ("IOM_BEGIN", IOM_BEGIN as i64),
            ("IOM_END", IOM_END as i64),
            ("IOM_SIZE", IOM_SIZE as i64),
        ];
        // Every define of the header but `IO_ISASIZES`, the guard of the sizes.
        assert_eq!(defs.len() - 1, ours.len());
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
