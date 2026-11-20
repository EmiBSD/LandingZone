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
 * Copyright (c) 1996, 1997, 1998 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Charles M. Hannum and by Jason R. Thorpe of the Numerical Aerospace
 * Simulation Facility, NASA Ames Research Center.
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
//! amd64 `bus_space(9)`: port I/O and memory space, `arch/amd64/amd64/bus_space.c`.
//!
//! Upstream: sys/arch/amd64/amd64/bus_space.c @ 3ce1f3f79392
//!
//! Status: `wip`. Milestone M2 ports the single-register accessors, `bus_space_map`/`unmap`
//! for I/O space and `bus_space_subregion`; M7b (virtio's BARs) the memory space:
//! `bus_space_map`/`bus_space_unmap` for it, `x86_mem_add_mapping`, `atdevbase` and the ISA
//! hole; M13 (efifb) `bus_space_vaddr`; M13 (vga) `bus_space_copy_2`. The multi/region accessors
//! and the other copy widths,
//! `bus_space_mmap` and the SEV-ES variants arrive with the buses that need them. The
//! extent maps `ioport_ex` and `iomem_ex` (`x86_bus_space_init`, `x86_bus_space_mallocok`)
//! guard every `bus_space_map` and `bus_space_alloc`.
//!
//! ## Deviations
//! - The tag is the enum [`X86BusSpace`] instead of a pointer to an ops table; the accessors
//!   dispatch on it. `X86_BUS_SPACE_IO`/`X86_BUS_SPACE_MEM` are its two values.
//! - `<machine/bus.h>` is not ported (one of its licence blocks has an advertising clause, see
//!   `ports.toml`): the `BUS_SPACE_MAP_*` flags and `bus_space_barrier`, which live there in C,
//!   are defined here from `bus_space(9)`.
//! - `ioport_ex` and `iomem_ex` are `AtomicPtr`s set by `x86_bus_space_init`, read through
//!   functions of their names; their fixed storage is 8-byte aligned, as the C's `long`
//!   arrays. `x86_bus_space_init` selects no ops tables: the tag enum dispatches to the
//!   default accessors, and the SEV-ES GHCB variants are not there yet (above).
//! - `atdevbase`, the kernel virtual address of the ISA hole, is written by the C's
//!   `locore0.S`, which maps the hole after the kernel; the bootloader's direct map covers it
//!   here, so [`atdevbase`] is the direct-map address of `IOM_BEGIN`. Limine's direct map
//!   (base revision 3) only covers its memory map's regions, though, and OVMF's leaves the
//!   VGA window (0xa0000-0xbffff) out: `bus_space_map` takes the ISA-hole shortcut only for
//!   pages mapped there ([`isa_hole_mapped`]) and maps the others like any device memory,
//!   uncached; their handles are outside the hole, so `bus_space_unmap` unmaps them as such
//!   (M13, vga(4)'s probe of the legacy text memory).

use core::arch::asm;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use libkern::StaticCell;

use crate::arch::amd64::amd64::pmap::{
    pmap_direct_map, pmap_direct_mapped, pmap_direct_unmap, pmap_extract, pmap_find_pte_direct,
    pmap_initialized, pmap_kenter_pa, pmap_kernel, pmap_kremove,
};
use crate::arch::amd64::include::pio::{inb, inl, inw, outb, outl, outw};
use crate::arch::amd64::include::pmap::{PMAP_NOCACHE, PMAP_WC};
use crate::arch::amd64::include::pte::PG_V;
use crate::dev::isa::isareg::{IOM_BEGIN, IOM_END};
use crate::kern::subr_extent::{
    extent_alloc_region, extent_alloc_subregion, extent_create, extent_free,
};
use crate::kern::subr_prf::{panic, printf};
use crate::machine::bus::{BUS_SPACE_BARRIER_READ, BUS_SPACE_BARRIER_WRITE, BusAddr, BusSize};
use crate::machine::pmap::pmap_update;
use crate::sys::errno::Errno;
use crate::sys::extent::{
    EX_FAST, EX_MALLOCOK, EX_NOCOALESCE, EX_NOWAIT, Extent, extent_fixed_storage_size,
};
use crate::sys::malloc::M_DEVBUF;
use crate::sys::mman::{PROT_READ, PROT_WRITE};
use crate::sys::param::{PAGE_SIZE, PGOFSET};
use crate::sys::types::{Paddr, Vaddr, Vsize};
use crate::uvm::uvm_km::{KD_NOWAIT, KP_NONE, KV_ANY, km_alloc, km_free};
use crate::uvm::uvm_param::{round_page, trunc_page};

/// `BUS_SPACE_MAP_CACHEABLE`.
pub const BUS_SPACE_MAP_CACHEABLE: u32 = 0x0001;
/// `BUS_SPACE_MAP_LINEAR`.
pub const BUS_SPACE_MAP_LINEAR: u32 = 0x0002;
/// `BUS_SPACE_MAP_PREFETCHABLE`.
pub const BUS_SPACE_MAP_PREFETCHABLE: u32 = 0x0008;

/// `bus_space_tag_t`: which of the two x86 spaces a handle lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X86BusSpace {
    /// `X86_BUS_SPACE_IO`: port I/O, reached with `in`/`out`.
    Io,
    /// `X86_BUS_SPACE_MEM`: memory-mapped, reached through a kernel virtual address.
    Mem,
}

/// `X86_BUS_SPACE_IO`: space is i/o space.
pub const X86_BUS_SPACE_IO: X86BusSpace = X86BusSpace::Io;
/// `X86_BUS_SPACE_MEM`: space is mem space.
pub const X86_BUS_SPACE_MEM: X86BusSpace = X86BusSpace::Mem;

/// `bus_space_handle_t`: a port base (I/O space) or a kernel virtual address (memory space),
/// only ever produced by [`bus_space_map`] or [`bus_space_subregion`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BusSpaceHandle(usize);

/// The storage of a fixed extent of 16 region descriptors, on `long` boundaries as the C's
/// `long` arrays.
#[repr(C, align(8))]
struct ExStorage([u8; extent_fixed_storage_size(16)]);

/// `ioport_ex_storage`: the descriptors of `ioport_ex`. Extent `ioport_ex` and `iomem_ex`
/// manage I/O and memory space. Since the cpu_info structures and the PCI code may allocate
/// I/O space before `malloc(9)` works, they start with fixed region descriptors.
static IOPORT_EX_STORAGE: StaticCell<ExStorage> =
    StaticCell::new(ExStorage([0; extent_fixed_storage_size(16)]));
/// `iomem_ex_storage`: the descriptors of `iomem_ex`.
static IOMEM_EX_STORAGE: StaticCell<ExStorage> =
    StaticCell::new(ExStorage([0; extent_fixed_storage_size(16)]));
/// `ioport_ex`.
static IOPORT_EX: AtomicPtr<Extent> = AtomicPtr::new(ptr::null_mut());
/// `iomem_ex`. N.B. At least two regions are always allocated from it: (0 -> ISA hole) and
/// (end of ISA hole -> end of RAM). The extent maps are not static: machine-dependent ISA
/// and EISA routines need access to them for bus address space allocation.
static IOMEM_EX: AtomicPtr<Extent> = AtomicPtr::new(ptr::null_mut());
/// `ioport_malloc_safe`: `malloc(9)` may give the extents more descriptors.
static IOPORT_MALLOC_SAFE: AtomicBool = AtomicBool::new(false);

/// `atdevbase`: the kernel virtual address of the ISA I/O memory hole (see the module's
/// deviations).
pub fn atdevbase() -> usize {
    pmap_direct_map(Paddr::new(IOM_BEGIN)).as_usize()
}

/// `ISA_HOLE_VADDR(p)`: the kernel virtual address of physical address `p` in the ISA hole.
fn isa_hole_vaddr(p: BusAddr) -> usize {
    p - IOM_BEGIN + atdevbase()
}

/// Whether the pages of `[bpa, bpa + size)` in the ISA hole are mapped at their
/// `ISA_HOLE_VADDR`: `locore0.S` maps the whole hole after a boot by boot(8), but Limine's
/// direct map (base revision 3) covers only the regions of its memory map, which on OVMF
/// leaves out the VGA window at 0xa0000 (see the module's deviations).
fn isa_hole_mapped(bpa: BusAddr, size: BusSize) -> bool {
    let first = bpa & !PGOFSET;
    (first..bpa + size).step_by(PAGE_SIZE).all(|pa| {
        let (_, table, index) = pmap_find_pte_direct(pmap_kernel(), isa_hole_vaddr(pa));
        // SAFETY: `pmap_find_pte_direct` returns the direct-map address of a page-table page
        // of the kernel pmap and an index into it (below 512); reading the entry has no side
        // effect.
        let e = unsafe { ptr::read_volatile((table as *const u64).add(index)) };
        e & PG_V != 0
    })
}

/// `ISA_PHYSADDR(v)`: the physical address of a kernel virtual address in the ISA hole.
fn isa_physaddr(v: usize) -> BusAddr {
    v.wrapping_sub(atdevbase()).wrapping_add(IOM_BEGIN)
}

/// `ioport_ex`: the I/O port extent, `None` before [`x86_bus_space_init`].
pub fn ioport_ex() -> Option<&'static Extent> {
    // SAFETY: null or an `extent_create` result, which lives for ever.
    unsafe { IOPORT_EX.load(Ordering::Acquire).as_ref() }
}

/// `iomem_ex`: the I/O memory extent, `None` before [`x86_bus_space_init`].
pub fn iomem_ex() -> Option<&'static Extent> {
    // SAFETY: as for `ioport_ex`.
    unsafe { IOMEM_EX.load(Ordering::Acquire).as_ref() }
}

/// The extent of space `t`.
fn bus_ex(t: X86BusSpace) -> Option<&'static Extent> {
    match t {
        X86BusSpace::Io => ioport_ex(),
        X86BusSpace::Mem => iomem_ex(),
    }
}

/// The extent flags of a claim: `EX_NOWAIT`, and `EX_MALLOCOK` once
/// [`x86_bus_space_mallocok`] said so.
fn ex_flags() -> i32 {
    EX_NOWAIT
        | if IOPORT_MALLOC_SAFE.load(Ordering::Relaxed) {
            EX_MALLOCOK
        } else {
            0
        }
}

/// `x86_bus_space_init`: makes the I/O port and I/O memory extent maps.
pub fn x86_bus_space_init() {
    // Initialize the I/O port and I/O mem extent maps. Note: we don't have to check the
    // return value since creation of a fixed extent map will never fail (since descriptor
    // storage has already been allocated).
    //
    // N.B. The iomem extent manages _all_ physical addresses on the machine. When the amount
    // of RAM is found, the two extents of RAM are allocated from the map (0 -> ISA hole and
    // end of ISA hole -> end of RAM).
    if !IOPORT_EX.load(Ordering::Acquire).is_null() {
        return;
    }
    // SAFETY: once, on the boot CPU before anything maps a bus space; the storage is the
    // extent's alone from now on.
    let storage = unsafe { &mut IOPORT_EX_STORAGE.get_mut().0 };
    if let Some(ex) = extent_create(
        b"ioport",
        0x0,
        0xffff,
        M_DEVBUF,
        Some(storage),
        EX_NOCOALESCE | EX_NOWAIT,
    ) {
        IOPORT_EX.store(ptr::from_ref(ex).cast_mut(), Ordering::Release);
    }
    // SAFETY: as above.
    let storage = unsafe { &mut IOMEM_EX_STORAGE.get_mut().0 };
    if let Some(ex) = extent_create(
        b"iomem",
        0x0,
        0xffff_ffff_ffff,
        M_DEVBUF,
        Some(storage),
        EX_NOCOALESCE | EX_NOWAIT,
    ) {
        IOMEM_EX.store(ptr::from_ref(ex).cast_mut(), Ordering::Release);
    }

    // AMDSEV: x86_bus_space_mem_ops/x86_bus_space_io_ops, the GHCB ones under SEV-ES, else
    // the default ones: the tag dispatches (see the module's deviations).
}

/// `x86_bus_space_mallocok`: from now on the extents may take descriptors from `malloc(9)`.
pub fn x86_bus_space_mallocok() {
    IOPORT_MALLOC_SAFE.store(true, Ordering::Relaxed);
}

/// `bus_space_map`: claims `[bpa, bpa + size)` in space `t`. I/O space needs no mapping, so
/// the handle is the port base; memory space is mapped into kernel virtual space, uncached
/// unless `flags` say otherwise (the ISA hole and, before `pmap_init`, memory below 4 GB
/// through the mappings that already exist).
///
/// # Safety
///
/// As for `machine::bus::BusSpace::bus_space_map`.
pub unsafe fn bus_space_map(
    t: X86BusSpace,
    bpa: BusAddr,
    size: BusSize,
    flags: u32,
) -> Result<BusSpaceHandle, Errno> {
    // Pick the appropriate extent map.
    if t == X86BusSpace::Io && flags & BUS_SPACE_MAP_LINEAR != 0 {
        return Err(Errno::EINVAL);
    }
    let ex = bus_ex(t);

    // Before we go any further, let's make sure that this region is available.
    if let Some(ex) = ex {
        extent_alloc_region(ex, bpa as u64, size as u64, ex_flags())?;
    }

    // For I/O space, that's all she wrote.
    if t == X86BusSpace::Io {
        return Ok(BusSpaceHandle(bpa));
    }

    if bpa >= IOM_BEGIN && bpa + size <= IOM_END && isa_hole_mapped(bpa, size) {
        return Ok(BusSpaceHandle(isa_hole_vaddr(bpa)));
    }

    if !pmap_initialized() && bpa < 0x1_0000_0000 {
        return Ok(BusSpaceHandle(pmap_direct_map(Paddr::new(bpa)).as_usize()));
    }

    // For memory space, map the bus physical address to a kernel virtual address.
    // SAFETY: the caller vouches for the device range.
    let r = unsafe { x86_mem_add_mapping(bpa, size, flags) };
    if r.is_err()
        && let Some(ex) = ex
        && extent_free(ex, bpa as u64, size as u64, ex_flags()).is_err()
    {
        printf(format_args!(
            "bus_space_map: pa 0x{:x}, size 0x{:x}\n",
            bpa, size
        ));
        printf(format_args!("bus_space_map: can't free region\n"));
    }
    r
}

/// `_bus_space_map`: [`bus_space_map`] for an extra extent manager: no kernel built-in
/// extent is asked, I/O space just fills in the handle and memory space is always mapped
/// into fresh kernel virtual space.
///
/// # Safety
///
/// As for [`bus_space_map`]: the range is a device the caller owns.
pub unsafe fn _bus_space_map(
    t: X86BusSpace,
    bpa: BusAddr,
    size: BusSize,
    flags: u32,
) -> Result<BusSpaceHandle, Errno> {
    // For I/O space, just fill in the handle.
    if t == X86BusSpace::Io {
        return Ok(BusSpaceHandle(bpa));
    }

    // For memory space, map the bus physical address to a kernel virtual address.
    // SAFETY: the caller vouches for the device range.
    unsafe { x86_mem_add_mapping(bpa, size, flags) }
}

/// `bus_space_alloc`: allocates `size` bytes of space `t` within `[rstart, rend]`, aligned
/// and not crossing `boundary`, and maps it: the bus address and the handle.
///
/// # Safety
///
/// As for [`bus_space_map`]: the space is a device's the caller will own.
pub unsafe fn bus_space_alloc(
    t: X86BusSpace,
    rstart: BusAddr,
    rend: BusAddr,
    size: BusSize,
    alignment: BusSize,
    boundary: BusSize,
    flags: u32,
) -> Result<(BusAddr, BusSpaceHandle), Errno> {
    // Pick the appropriate extent map.
    let Some(ex) = bus_ex(t) else {
        return Err(Errno::EINVAL);
    };

    // Sanity check the allocation against the extent's boundaries.
    if (rstart as u64) < ex.ex_start || rend as u64 > ex.ex_end {
        panic(format_args!("bus_space_alloc: bad region start/end"));
    }

    // Do the requested allocation.
    let bpa = extent_alloc_subregion(
        ex,
        rstart as u64,
        rend as u64,
        size as u64,
        alignment as u64,
        0,
        boundary as u64,
        EX_FAST | ex_flags(),
    )? as BusAddr;

    // For I/O space, that's all she wrote.
    if t == X86BusSpace::Io {
        return Ok((bpa, BusSpaceHandle(bpa)));
    }

    // For memory space, map the bus physical address to a kernel virtual address.
    // SAFETY: the caller's guarantee; the range was just claimed for it.
    match unsafe { x86_mem_add_mapping(bpa, size, flags) } {
        Ok(bsh) => Ok((bpa, bsh)),
        Err(e) => {
            if let Some(iomem) = iomem_ex()
                && extent_free(iomem, bpa as u64, size as u64, ex_flags()).is_err()
            {
                printf(format_args!(
                    "bus_space_alloc: pa 0x{:x}, size 0x{:x}\n",
                    bpa, size
                ));
                printf(format_args!("bus_space_alloc: can't free region\n"));
            }
            Err(e)
        }
    }
}

/// `x86_mem_add_mapping`: maps `[bpa, bpa + size)` into fresh kernel virtual space, page by
/// page, uncached unless `BUS_SPACE_MAP_CACHEABLE` (write-back) or
/// `BUS_SPACE_MAP_PREFETCHABLE` (write-combining) asks otherwise.
///
/// # Safety
///
/// As for [`bus_space_map`]: the range is a device the caller owns.
pub unsafe fn x86_mem_add_mapping(
    bpa: BusAddr,
    size: BusSize,
    flags: u32,
) -> Result<BusSpaceHandle, Errno> {
    let mut pa = trunc_page(bpa);
    let endpa = round_page(bpa + size);

    #[cfg(feature = "diagnostic")]
    if endpa <= pa && endpa != 0 {
        crate::kern::subr_prf::panic(format_args!("bus_mem_add_mapping: overflow"));
    }

    let mut map_size = endpa - pa;

    let Some(va) = km_alloc(map_size, &KV_ANY, &KP_NONE, &KD_NOWAIT) else {
        return Err(Errno::ENOMEM);
    };
    let mut va = va.as_ptr() as usize;

    let bsh = BusSpaceHandle(va + (bpa & PGOFSET));

    let pmap_flags = if flags & BUS_SPACE_MAP_CACHEABLE != 0 {
        0
    } else if flags & BUS_SPACE_MAP_PREFETCHABLE != 0 {
        PMAP_WC as usize
    } else {
        PMAP_NOCACHE as usize
    };

    while map_size > 0 {
        // SAFETY: `va` is fresh kernel virtual space from km_alloc(kv_any, kp_none), nothing
        // else maps it; `pa` is the caller's device page.
        unsafe {
            pmap_kenter_pa(
                Vaddr::new(va),
                Paddr::new(pa | pmap_flags),
                PROT_READ | PROT_WRITE,
            )
        };
        pa += PAGE_SIZE;
        va += PAGE_SIZE;
        map_size -= PAGE_SIZE;
    }
    pmap_update(pmap_kernel());

    Ok(bsh)
}

/// `_bus_space_unmap`: the mapping of a handle from [`_bus_space_map`] released, without the
/// kernel built-in extents, and the bus physical address it was (the C's `*adrp`).
pub fn _bus_space_unmap(t: X86BusSpace, bsh: BusSpaceHandle, size: BusSize) -> BusAddr {
    // Find the correct bus physical address.
    match t {
        X86BusSpace::Io => bsh.0,
        X86BusSpace::Mem => {
            let bpa = isa_physaddr(bsh.0);
            if (IOM_BEGIN..=IOM_END).contains(&bpa) {
                return bpa;
            }

            let va = trunc_page(bsh.0);
            let endva = round_page(bsh.0 + size);

            #[cfg(feature = "diagnostic")]
            if endva <= va {
                crate::kern::subr_prf::panic(format_args!("_bus_space_unmap: overflow"));
            }

            let bpa = pmap_extract(pmap_kernel(), Vaddr::new(va)).map_or(0, Paddr::as_usize)
                + (bsh.0 & PGOFSET);

            // SAFETY: `[va, endva)` is a mapping `x86_mem_add_mapping` made for this handle,
            // which the caller gives up.
            unsafe { pmap_kremove(Vaddr::new(va), Vsize::new(endva - va)) };
            pmap_update(pmap_kernel());

            // Free the kernel virtual mapping.
            if let Some(v) = core::ptr::NonNull::new(va as *mut u8) {
                km_free(v, endva - va, &KV_ANY, &KP_NONE);
            }
            bpa
        }
    }
}

/// `bus_space_unmap`: releases a mapping.
pub fn bus_space_unmap(t: X86BusSpace, bsh: BusSpaceHandle, size: BusSize) {
    // Find the correct extent and bus physical address.
    let bpa: BusAddr = match t {
        X86BusSpace::Io => bsh.0,
        X86BusSpace::Mem => 'mem: {
            let bpa = isa_physaddr(bsh.0);
            if (IOM_BEGIN..=IOM_END).contains(&bpa) {
                break 'mem bpa;
            }

            if pmap_direct_mapped(Vaddr::new(bsh.0)) {
                break 'mem pmap_direct_unmap(Vaddr::new(bsh.0)).as_usize();
            }

            let va = trunc_page(bsh.0);
            let endva = round_page(bsh.0 + size);

            #[cfg(feature = "diagnostic")]
            if endva <= va {
                crate::kern::subr_prf::panic(format_args!("bus_space_unmap: overflow"));
            }

            let bpa = pmap_extract(pmap_kernel(), Vaddr::new(va)).map_or(0, Paddr::as_usize)
                + (bsh.0 & PGOFSET);

            // SAFETY: `[va, endva)` is a mapping `x86_mem_add_mapping` made for this handle,
            // which the caller gives up.
            unsafe { pmap_kremove(Vaddr::new(va), Vsize::new(endva - va)) };
            pmap_update(pmap_kernel());

            // Free the kernel virtual mapping.
            if let Some(v) = core::ptr::NonNull::new(va as *mut u8) {
                km_free(v, endva - va, &KV_ANY, &KP_NONE);
            }
            bpa
        }
    };

    // ok:
    if let Some(ex) = bus_ex(t)
        && extent_free(ex, bpa as u64, size as u64, ex_flags()).is_err()
    {
        printf(format_args!(
            "bus_space_unmap: {} 0x{:x}, size 0x{:x}\n",
            if t == X86BusSpace::Io { "port" } else { "pa" },
            bpa,
            size
        ));
        printf(format_args!("bus_space_unmap: can't free region\n"));
    }
}

/// `bus_space_free`: gives back space from [`bus_space_alloc`].
pub fn bus_space_free(t: X86BusSpace, bsh: BusSpaceHandle, size: BusSize) {
    // bus_space_unmap() does all that we need to do.
    bus_space_unmap(t, bsh, size);
}

/// `bus_space_subregion`: a handle for `[offset, offset + size)` of an existing mapping.
pub fn bus_space_subregion(
    _t: X86BusSpace,
    bsh: BusSpaceHandle,
    offset: BusSize,
    _size: BusSize,
) -> Result<BusSpaceHandle, Errno> {
    Ok(BusSpaceHandle(bsh.0 + offset))
}

/// `bus_space_vaddr`: the handle of a memory mapping is its kernel virtual address; I/O
/// space has none.
pub fn bus_space_vaddr(t: X86BusSpace, h: BusSpaceHandle) -> *mut u8 {
    if t == X86_BUS_SPACE_MEM {
        h.0 as *mut u8
    } else {
        core::ptr::null_mut()
    }
}

/// `x86_bus_space_io_read_1`.
pub fn x86_bus_space_io_read_1(h: BusSpaceHandle, o: BusSize) -> u8 {
    // SAFETY: `h` came from `bus_space_map` in I/O space, so `h + o` is a port of a device
    // this driver owns.
    unsafe { inb((h.0 + o) as u16) }
}

/// `x86_bus_space_io_read_2`.
pub fn x86_bus_space_io_read_2(h: BusSpaceHandle, o: BusSize) -> u16 {
    // SAFETY: as for `x86_bus_space_io_read_1`.
    unsafe { inw((h.0 + o) as u16) }
}

/// `x86_bus_space_io_read_4`.
pub fn x86_bus_space_io_read_4(h: BusSpaceHandle, o: BusSize) -> u32 {
    // SAFETY: as for `x86_bus_space_io_read_1`.
    unsafe { inl((h.0 + o) as u16) }
}

/// `x86_bus_space_io_read_8`: there are no 64-bit ports.
pub fn x86_bus_space_io_read_8(_h: BusSpaceHandle, _o: BusSize) -> u64 {
    crate::kern::subr_prf::panic(format_args!("bus_space_read_8: invalid bus space tag"));
}

/// `x86_bus_space_io_write_1`.
pub fn x86_bus_space_io_write_1(h: BusSpaceHandle, o: BusSize, v: u8) {
    // SAFETY: as for `x86_bus_space_io_read_1`.
    unsafe { outb((h.0 + o) as u16, v) }
}

/// `x86_bus_space_io_write_2`.
pub fn x86_bus_space_io_write_2(h: BusSpaceHandle, o: BusSize, v: u16) {
    // SAFETY: as for `x86_bus_space_io_read_1`.
    unsafe { outw((h.0 + o) as u16, v) }
}

/// `x86_bus_space_io_write_4`.
pub fn x86_bus_space_io_write_4(h: BusSpaceHandle, o: BusSize, v: u32) {
    // SAFETY: as for `x86_bus_space_io_read_1`.
    unsafe { outl((h.0 + o) as u16, v) }
}

/// `x86_bus_space_io_write_8`: there are no 64-bit ports.
pub fn x86_bus_space_io_write_8(_h: BusSpaceHandle, _o: BusSize, _v: u64) {
    crate::kern::subr_prf::panic(format_args!("bus_space_write_8: invalid bus space tag"));
}

/// `x86_bus_space_mem_read_1`.
pub fn x86_bus_space_mem_read_1(h: BusSpaceHandle, o: BusSize) -> u8 {
    // SAFETY: `h` came from `bus_space_map` in memory space, which mapped the device at it.
    unsafe { ptr::read_volatile((h.0 + o) as *const u8) }
}

/// `x86_bus_space_mem_read_2`.
pub fn x86_bus_space_mem_read_2(h: BusSpaceHandle, o: BusSize) -> u16 {
    // SAFETY: as for `x86_bus_space_mem_read_1`.
    unsafe { ptr::read_volatile((h.0 + o) as *const u16) }
}

/// `x86_bus_space_mem_read_4`.
pub fn x86_bus_space_mem_read_4(h: BusSpaceHandle, o: BusSize) -> u32 {
    // SAFETY: as for `x86_bus_space_mem_read_1`.
    unsafe { ptr::read_volatile((h.0 + o) as *const u32) }
}

/// `x86_bus_space_mem_read_8`.
pub fn x86_bus_space_mem_read_8(h: BusSpaceHandle, o: BusSize) -> u64 {
    // SAFETY: as for `x86_bus_space_mem_read_1`.
    unsafe { ptr::read_volatile((h.0 + o) as *const u64) }
}

/// `x86_bus_space_mem_write_1`.
pub fn x86_bus_space_mem_write_1(h: BusSpaceHandle, o: BusSize, v: u8) {
    // SAFETY: as for `x86_bus_space_mem_read_1`.
    unsafe { ptr::write_volatile((h.0 + o) as *mut u8, v) }
}

/// `x86_bus_space_mem_write_2`.
pub fn x86_bus_space_mem_write_2(h: BusSpaceHandle, o: BusSize, v: u16) {
    // SAFETY: as for `x86_bus_space_mem_read_1`.
    unsafe { ptr::write_volatile((h.0 + o) as *mut u16, v) }
}

/// `x86_bus_space_mem_write_4`.
pub fn x86_bus_space_mem_write_4(h: BusSpaceHandle, o: BusSize, v: u32) {
    // SAFETY: as for `x86_bus_space_mem_read_1`.
    unsafe { ptr::write_volatile((h.0 + o) as *mut u32, v) }
}

/// `x86_bus_space_mem_write_8`.
pub fn x86_bus_space_mem_write_8(h: BusSpaceHandle, o: BusSize, v: u64) {
    // SAFETY: as for `x86_bus_space_mem_read_1`.
    unsafe { ptr::write_volatile((h.0 + o) as *mut u64, v) }
}

/// `bus_space_read_1`: dispatches on the space.
pub fn bus_space_read_1(t: X86BusSpace, h: BusSpaceHandle, o: BusSize) -> u8 {
    match t {
        X86BusSpace::Io => x86_bus_space_io_read_1(h, o),
        X86BusSpace::Mem => x86_bus_space_mem_read_1(h, o),
    }
}

/// `bus_space_read_2`.
pub fn bus_space_read_2(t: X86BusSpace, h: BusSpaceHandle, o: BusSize) -> u16 {
    match t {
        X86BusSpace::Io => x86_bus_space_io_read_2(h, o),
        X86BusSpace::Mem => x86_bus_space_mem_read_2(h, o),
    }
}

/// `bus_space_read_4`.
pub fn bus_space_read_4(t: X86BusSpace, h: BusSpaceHandle, o: BusSize) -> u32 {
    match t {
        X86BusSpace::Io => x86_bus_space_io_read_4(h, o),
        X86BusSpace::Mem => x86_bus_space_mem_read_4(h, o),
    }
}

/// `bus_space_read_8`: memory space only.
pub fn bus_space_read_8(t: X86BusSpace, h: BusSpaceHandle, o: BusSize) -> u64 {
    match t {
        X86BusSpace::Io => x86_bus_space_io_read_8(h, o),
        X86BusSpace::Mem => x86_bus_space_mem_read_8(h, o),
    }
}

/// `bus_space_write_1`.
pub fn bus_space_write_1(t: X86BusSpace, h: BusSpaceHandle, o: BusSize, v: u8) {
    match t {
        X86BusSpace::Io => x86_bus_space_io_write_1(h, o, v),
        X86BusSpace::Mem => x86_bus_space_mem_write_1(h, o, v),
    }
}

/// `bus_space_write_2`.
pub fn bus_space_write_2(t: X86BusSpace, h: BusSpaceHandle, o: BusSize, v: u16) {
    match t {
        X86BusSpace::Io => x86_bus_space_io_write_2(h, o, v),
        X86BusSpace::Mem => x86_bus_space_mem_write_2(h, o, v),
    }
}

/// `bus_space_write_4`.
pub fn bus_space_write_4(t: X86BusSpace, h: BusSpaceHandle, o: BusSize, v: u32) {
    match t {
        X86BusSpace::Io => x86_bus_space_io_write_4(h, o, v),
        X86BusSpace::Mem => x86_bus_space_mem_write_4(h, o, v),
    }
}

/// `bus_space_write_8`: memory space only.
pub fn bus_space_write_8(t: X86BusSpace, h: BusSpaceHandle, o: BusSize, v: u64) {
    match t {
        X86BusSpace::Io => x86_bus_space_io_write_8(h, o, v),
        X86BusSpace::Mem => x86_bus_space_mem_write_8(h, o, v),
    }
}

/// `x86_bus_space_io_copy_2`: `c` 16-bit ports from `h1 + o1` to `h2 + o2`, forward when
/// the source is at or after the destination, backwards otherwise, so overlapping ranges
/// copy as a move.
pub fn x86_bus_space_io_copy_2(
    h1: BusSpaceHandle,
    o1: BusSize,
    h2: BusSpaceHandle,
    o2: BusSize,
    c: usize,
) {
    let (addr1, addr2) = (h1.0 + o1, h2.0 + o2);

    if addr1 >= addr2 {
        // src after dest: copy forward
        for i in 0..c {
            x86_bus_space_io_write_2(BusSpaceHandle(addr2), 2 * i, {
                x86_bus_space_io_read_2(BusSpaceHandle(addr1), 2 * i)
            });
        }
    } else {
        // dest after src: copy backwards
        for i in (0..c).rev() {
            x86_bus_space_io_write_2(BusSpaceHandle(addr2), 2 * i, {
                x86_bus_space_io_read_2(BusSpaceHandle(addr1), 2 * i)
            });
        }
    }
}

/// `x86_bus_space_mem_copy_2`: as [`x86_bus_space_io_copy_2`], in memory space.
pub fn x86_bus_space_mem_copy_2(
    h1: BusSpaceHandle,
    o1: BusSize,
    h2: BusSpaceHandle,
    o2: BusSize,
    c: usize,
) {
    let (addr1, addr2) = (h1.0 + o1, h2.0 + o2);

    if addr1 >= addr2 {
        // src after dest: copy forward
        for i in 0..c {
            x86_bus_space_mem_write_2(BusSpaceHandle(addr2), 2 * i, {
                x86_bus_space_mem_read_2(BusSpaceHandle(addr1), 2 * i)
            });
        }
    } else {
        // dest after src: copy backwards
        for i in (0..c).rev() {
            x86_bus_space_mem_write_2(BusSpaceHandle(addr2), 2 * i, {
                x86_bus_space_mem_read_2(BusSpaceHandle(addr1), 2 * i)
            });
        }
    }
}

/// `bus_space_copy_2`: dispatches on the space.
pub fn bus_space_copy_2(
    t: X86BusSpace,
    h1: BusSpaceHandle,
    o1: BusSize,
    h2: BusSpaceHandle,
    o2: BusSize,
    c: usize,
) {
    match t {
        X86BusSpace::Io => x86_bus_space_io_copy_2(h1, o1, h2, o2, c),
        X86BusSpace::Mem => x86_bus_space_mem_copy_2(h1, o1, h2, o2, c),
    }
}

/// `bus_space_barrier`: `mfence` for read and write, `sfence` for write only, `lfence`
/// otherwise.
pub fn bus_space_barrier(
    _t: X86BusSpace,
    _h: BusSpaceHandle,
    _offset: BusSize,
    _length: BusSize,
    flags: u32,
) {
    // SAFETY: fences order memory accesses and touch nothing else.
    unsafe {
        if flags == BUS_SPACE_BARRIER_READ | BUS_SPACE_BARRIER_WRITE {
            asm!("mfence", options(nostack, preserves_flags));
        } else if flags == BUS_SPACE_BARRIER_WRITE {
            asm!("sfence", options(nostack, preserves_flags));
        } else {
            asm!("lfence", options(nostack, preserves_flags));
        }
    }
}
/* </CODE> */
