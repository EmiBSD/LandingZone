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

/*
 * Copyright (c) 2016 Patrick Wildt <patrick@blueri.se>
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
//! arm64 `<machine/fdt.h>`: the device-tree attach arguments and the `fdt_intr_*` names.
//!
//! Upstream: sys/arch/arm64/include/fdt.h @ 3ce1f3f79392
//!
//! Status: `ported`. `struct fdt_attach_args` and the fifteen `fdt_intr_*` aliases of the
//! `arm_intr_*` functions (`arm64/intr.rs`). What the header declares besides is defined
//! where the C defines it: `stdout_node`, `stdout_speed` and `fdt_find_cons` in
//! `arm64/machdep.rs` (`STDOUT_NODE`, `STDOUT_SPEED`), `fdt_cons_bs_tag` in
//! `arm64/bus_space.rs` (`FDT_CONS_BS_TAG`); generic code reaches them through the
//! `machine::fdt` contract.
//!
//! ## Deviations
//! - `fa_reg`/`fa_nreg` and `fa_intr`/`fa_nintr` are slices; `fa_name` is a byte string.
//! - `_ARM32_BUS_DMA_PRIVATE`, which the header defines before `<machine/bus.h>` to get the
//!   private half of the DMA tag, has no counterpart: `bus.rs` has one definition of the tag.

pub use crate::arch::arm64::arm64::intr::{
    arm_intr_disable as fdt_intr_disable, arm_intr_disestablish_fdt as fdt_intr_disestablish,
    arm_intr_enable as fdt_intr_enable, arm_intr_establish_fdt as fdt_intr_establish,
    arm_intr_establish_fdt_cpu as fdt_intr_establish_cpu,
    arm_intr_establish_fdt_idx as fdt_intr_establish_idx,
    arm_intr_establish_fdt_idx_cpu as fdt_intr_establish_idx_cpu,
    arm_intr_establish_fdt_imap as fdt_intr_establish_imap,
    arm_intr_establish_fdt_imap_cpu as fdt_intr_establish_imap_cpu,
    arm_intr_establish_fdt_msi as fdt_intr_establish_msi,
    arm_intr_establish_fdt_msi_cpu as fdt_intr_establish_msi_cpu,
    arm_intr_get_parent as fdt_intr_get_parent,
    arm_intr_parent_disestablish_fdt as fdt_intr_parent_disestablish,
    arm_intr_parent_establish_fdt as fdt_intr_parent_establish,
    arm_intr_register_fdt as fdt_intr_register,
};
use crate::dev::ofw::fdt::FdtReg;
use crate::machine::bus::{BusDmaTag, BusSpaceTag};

/// `struct fdt_attach_args`: what a device-tree node's driver is attached with.
pub struct FdtAttachArgs<'a> {
    /// `fa_name`.
    pub fa_name: &'a [u8],
    /// `fa_node`: the node handle.
    pub fa_node: i32,
    /// `fa_iot`.
    pub fa_iot: BusSpaceTag,
    /// `fa_dmat`: the node's DMA tag (mainbus's, or a `dma-coherent` copy of it).
    pub fa_dmat: BusDmaTag,
    /// `fa_reg`: the node's `reg` entries (`fa_nreg` of them).
    pub fa_reg: &'a [FdtReg],
    /// `fa_intr`: the node's `interrupts` cells (`fa_nintr` of them).
    pub fa_intr: &'a [u32],
    /// `fa_acells`.
    pub fa_acells: i32,
    /// `fa_scells`.
    pub fa_scells: i32,
}
/* </CODE> */
