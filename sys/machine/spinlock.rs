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
//! `<machine/spinlock.h>` as a trait: the word user-space spinlocks are made of.
//!
//! The kernel needs only the type and `_ATOMIC_LOCK_UNLOCKED`: `__thrsleep(2)` stores the
//! unlocked value through the caller's lock once the caller is on the sleep queue
//! (`kern_synch.c`'s `thrsleep_unlock`). Each architecture defines them in
//! `arch/<arch>/include/spinlock.rs`; generic code reads them here.

use crate::machine::Machine;
use crate::machine::copy::AbiPod;

/// The user-space spinlock word of the selected architecture.
pub trait MachineSpinlock {
    /// `_atomic_lock_t`: the lock word, copied out to user space.
    type AtomicLock: AbiPod;
    /// `_ATOMIC_LOCK_UNLOCKED`: the value of a free lock.
    const ATOMIC_LOCK_UNLOCKED: Self::AtomicLock;
    /// `_ATOMIC_LOCK_LOCKED`: the value of a held lock.
    const ATOMIC_LOCK_LOCKED: Self::AtomicLock;
}

/// `_atomic_lock_t` on the selected machine.
pub type AtomicLock = <Machine as MachineSpinlock>::AtomicLock;

/// `_ATOMIC_LOCK_UNLOCKED` on the selected machine.
pub const _ATOMIC_LOCK_UNLOCKED: AtomicLock = <Machine as MachineSpinlock>::ATOMIC_LOCK_UNLOCKED;

/// `_ATOMIC_LOCK_LOCKED` on the selected machine.
pub const _ATOMIC_LOCK_LOCKED: AtomicLock = <Machine as MachineSpinlock>::ATOMIC_LOCK_LOCKED;
/* </CODE> */
