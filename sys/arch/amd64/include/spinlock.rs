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
//! amd64 `<machine/spinlock.h>`: the word user-space spinlocks are made of.
//!
//! Upstream: sys/arch/amd64/include/spinlock.h @ 3ce1f3f79392
//!
//! Status: `ported`. The kernel reads the type and `_ATOMIC_LOCK_UNLOCKED` through the
//! [`MachineSpinlock`] implementation at the bottom (`__thrsleep(2)` stores the unlocked
//! value through the caller's lock).
//!
//! ## Deviations
//! - The typedef `_atomic_lock_t` is the type alias [`AtomicLock`].

use crate::machine::MachineSpinlock;

/// `_ATOMIC_LOCK_UNLOCKED`: the value of a free lock.
pub const _ATOMIC_LOCK_UNLOCKED: AtomicLock = 0;
/// `_ATOMIC_LOCK_LOCKED`: the value of a held lock.
pub const _ATOMIC_LOCK_LOCKED: AtomicLock = 1;

/// `_atomic_lock_t`: an `int`.
pub type AtomicLock = i32;

impl MachineSpinlock for super::super::Machine {
    type AtomicLock = AtomicLock;
    const ATOMIC_LOCK_UNLOCKED: AtomicLock = _ATOMIC_LOCK_UNLOCKED;
    const ATOMIC_LOCK_LOCKED: AtomicLock = _ATOMIC_LOCK_LOCKED;
}
/* </CODE> */
