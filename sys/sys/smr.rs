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
 * Copyright (c) 2019 Visa Hankala
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
 * Copyright (c) 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)queue.h	8.5 (Berkeley) 8/20/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/smr.h>`: safe memory reclamation, see `smr_call(9)`. Readers walk shared data
//! inside a read section (`smr_read_enter`/`smr_read_leave`) without locks; writers unlink an
//! object under their lock and hand its destruction to `smr_call`, which runs it once every
//! CPU has passed a quiescent state (a context switch, or the idle loop).
//!
//! Upstream: sys/sys/smr.h @ 3ce1f3f79392
//!
//! Status: `ported`, the whole header: `struct smr_entry`, `struct smr_entry_list`,
//! `smr_init`, `smr_call`, `smr_barrier`, `smr_flush`, `SMR_ASSERT_CRITICAL`,
//! `SMR_ASSERT_NONCRITICAL`, `SMR_PTR_GET`, `SMR_PTR_GET_LOCKED` and `SMR_PTR_SET_LOCKED` (as
//! [`SmrPtr`]), and the three list families readers walk without a lock: singly-linked lists
//! (`SMR_SLIST_*`, [`SmrSlistHead`]), lists (`SMR_LIST_*`, [`SmrListHead`]) and tail queues
//! (`SMR_TAILQ_*`, [`SmrTailqHead`]). The functions the header declares (`smr_startup`,
//! `smr_startup_thread`, `smr_idle`, `smr_read_enter`, `smr_read_leave`, `smr_call_impl`,
//! `smr_barrier_impl`) are in `kern/kern_smr.rs`.
//!
//! ## Deviations
//! - The lists follow `<sys/queue.h>`'s port: a head over an adapter (`queue_adapter!`)
//!   whose entry is an [`SmrSlistEntry`], an [`SmrListEntry`] or an [`SmrTailqEntry`], and
//!   the C's names without the family prefix (`SMR_LIST_INSERT_HEAD_LOCKED(head, elm,
//!   field)` is `head.insert_head_locked(elm)`). `*_END` is `None`.
//! - The links are atomics. The forward links, which readers follow, are read with
//!   `Acquire` and published with `Release` (the C's `membar_producer` before the store
//!   that makes an element reachable). The backward links and a tail queue's `last`, which
//!   only the writer that holds the list's lock touches, are `Relaxed`: atomics there cost
//!   nothing and keep an entry `Sync` without an `unsafe impl`.
//! - A removed element keeps its forward link, as in C, so a reader standing on it goes on.
//! - `_FOREACH_LOCKED` and `_FOREACH_SAFE_LOCKED` are one iterator (`iter_locked`), which
//!   reads the next element before it yields the current one.
//! - `SMR_TAILQ_HEAD_INITIALIZER` cannot take the head's own address in a `const`: an empty
//!   head stores a null `smr_tqh_last` that means "the head's first link", as `TailqHead`
//!   does, so an empty head can be moved.
//! - `smr_call` (and `smr_call_impl`, `kern_smr.rs`) take a `&'static SmrEntry`: the entry
//!   must outlive the deferral (`smr_barrier_impl` lends its stack entry as `'static`
//!   while it waits for the call).
//! - `SMR_PTR_GET` is an `Acquire` load where the C's `READ_ONCE` relies on the hardware's
//!   dependency ordering, which Rust's memory model does not offer; `SMR_PTR_SET_LOCKED` is a
//!   `Release` store (the C's `membar_producer` then `WRITE_ONCE`).

use core::cell::Cell;
use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::queue_adapter;
use crate::sys::queue::{Adapter, SimpleqEntry};

/// `struct smr_entry`: a deferred call, embedded in the object it destroys.
pub struct SmrEntry {
    /// `smr_list`: the CPU's or the system-wide queue of deferred calls.
    pub smr_list: SimpleqEntry<SmrEntry>,
    /// `smr_func`: the deferred function, `None` while the entry is idle.
    pub smr_func: Cell<Option<fn(*mut c_void)>>,
    /// `smr_arg`: its argument.
    pub smr_arg: Cell<*mut c_void>,
}

impl SmrEntry {
    /// An idle entry (what `smr_init` leaves, and the zero the C's allocators give).
    pub const fn new() -> Self {
        Self {
            smr_list: SimpleqEntry::new(),
            smr_func: Cell::new(None),
            smr_arg: Cell::new(ptr::null_mut()),
        }
    }
}

impl Default for SmrEntry {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: an entry is written by the CPU that queues it (at splhigh, on its own queue), then
// moved between queues under `smr_lock` and read by the SMR thread once it owns it.
unsafe impl Sync for SmrEntry {}

queue_adapter!(
    /// `SIMPLEQ_HEAD(smr_entry_list, smr_entry)`.
    pub SmrEntryList: SmrEntry, smr_list => SimpleqEntry<SmrEntry>
);

/// `SMR_PTR_GET`/`SMR_PTR_GET_LOCKED`/`SMR_PTR_SET_LOCKED` over one SMR-protected pointer:
/// readers load it inside a read section, the writer stores it under its lock.
pub struct SmrPtr<T>(AtomicPtr<T>);

impl<T> SmrPtr<T> {
    /// A null pointer.
    pub const fn new() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
    }

    /// `SMR_PTR_GET(pptr)`: the pointer, for a reader inside a read section.
    pub fn get(&self) -> *mut T {
        self.0.load(Ordering::Acquire)
    }

    /// `SMR_PTR_GET_LOCKED(pptr)`: the pointer, for the writer that holds the lock.
    pub fn get_locked(&self) -> *mut T {
        self.0.load(Ordering::Relaxed)
    }

    /// `SMR_PTR_SET_LOCKED(pptr, val)`: publishes `val` once everything it points to is
    /// written (`membar_producer`).
    pub fn set_locked(&self, val: *mut T) {
        self.0.store(val, Ordering::Release);
    }
}

impl<T> Default for SmrPtr<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `SMR_SLIST_ENTRY(type)`: the link an element embeds to be in an SMR singly-linked list.
pub struct SmrSlistEntry<T> {
    /// `smr_sle_next`: next element, SMR-protected.
    smr_sle_next: AtomicPtr<T>,
}

impl<T> SmrSlistEntry<T> {
    /// An entry that is in no list (all-zero, as the C's `M_ZERO` elements).
    pub const fn new() -> Self {
        Self {
            smr_sle_next: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl<T> Default for SmrSlistEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `SMR_SLIST_HEAD(name, type)`: a singly-linked list that readers walk inside SMR read
/// sections while one writer, holding its lock, changes it. A removed element must not be
/// freed before a grace period (`smr_call`, `smr_barrier`).
pub struct SmrSlistHead<A: Adapter> {
    /// `smr_slh_first`: first element, SMR-protected.
    smr_slh_first: AtomicPtr<A::Elem>,
}

impl<A: SmrSlistAdapter> SmrSlistHead<A> {
    /// `SMR_SLIST_HEAD_INITIALIZER`: an empty list.
    pub const fn new() -> Self {
        Self {
            smr_slh_first: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// `SMR_SLIST_INIT`.
    pub fn init(&self) {
        self.smr_slh_first.store(ptr::null_mut(), Ordering::Release);
    }

    /// `SMR_SLIST_FIRST`: for a reader inside a read section.
    pub fn first(&self) -> Option<&A::Elem> {
        // SAFETY: a linked element, or one unlinked less than a grace period ago, is valid
        // while the reader stays in its read section (the writers' contract).
        unsafe { self.smr_slh_first.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_SLIST_NEXT`: for a reader inside a read section.
    pub fn next(elem: &A::Elem) -> Option<&A::Elem> {
        // SAFETY: as for `first`.
        unsafe { A::entry(elem).smr_sle_next.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_SLIST_FOREACH`: for a reader inside a read section.
    pub fn iter(&self) -> SmrSlistIter<'_, A> {
        SmrSlistIter {
            cur: self.smr_slh_first.load(Ordering::Acquire),
            _head: PhantomData,
        }
    }

    /// `SMR_SLIST_FIRST_LOCKED`: for the writer that holds the list's lock.
    pub fn first_locked(&self) -> Option<&A::Elem> {
        // SAFETY: a linked element is valid until unlinked (the mutators' contract).
        unsafe { self.smr_slh_first.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_SLIST_EMPTY_LOCKED`.
    pub fn is_empty_locked(&self) -> bool {
        self.smr_slh_first.load(Ordering::Acquire).is_null()
    }

    /// `SMR_SLIST_NEXT_LOCKED`: for the writer that holds the list's lock.
    pub fn next_locked(elem: &A::Elem) -> Option<&A::Elem> {
        // SAFETY: as for `first_locked`.
        unsafe { A::entry(elem).smr_sle_next.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_SLIST_FOREACH_LOCKED` and `SMR_SLIST_FOREACH_SAFE_LOCKED`: for the writer.
    pub fn iter_locked(&self) -> SmrSlistIter<'_, A> {
        self.iter()
    }

    /// `SMR_SLIST_INSERT_AFTER_LOCKED`: links `elem` after `slistelm`, once its link is
    /// written.
    ///
    /// # Safety
    ///
    /// The caller holds the list's lock; `slistelm` is in the list; `elem` is in no list of
    /// `A` and stays valid and in place until it is unlinked and a grace period has passed.
    pub unsafe fn insert_after_locked(slistelm: &A::Elem, elem: &A::Elem) {
        let after = A::entry(slistelm);
        A::entry(elem).smr_sle_next.store(
            after.smr_sle_next.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        after
            .smr_sle_next
            .store(ptr::from_ref(elem).cast_mut(), Ordering::Release);
    }

    /// `SMR_SLIST_INSERT_HEAD_LOCKED`: links `elem` first, once its link is written.
    ///
    /// # Safety
    ///
    /// The caller holds the list's lock; `elem` is in no list of `A` and stays valid and in
    /// place until it is unlinked and a grace period has passed.
    pub unsafe fn insert_head_locked(&self, elem: &A::Elem) {
        A::entry(elem).smr_sle_next.store(
            self.smr_slh_first.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.smr_slh_first
            .store(ptr::from_ref(elem).cast_mut(), Ordering::Release);
    }

    /// `SMR_SLIST_REMOVE_AFTER_LOCKED`: unlinks the element after `elem`.
    ///
    /// # Safety
    ///
    /// The caller holds the list's lock; `elem` is in the list and is not its last element.
    pub unsafe fn remove_after_locked(elem: &A::Elem) {
        let entry = A::entry(elem);
        // SAFETY: `elem` has a successor, a linked element, valid until unlinked.
        let next = unsafe { &*entry.smr_sle_next.load(Ordering::Relaxed) };
        entry.smr_sle_next.store(
            A::entry(next).smr_sle_next.load(Ordering::Relaxed),
            Ordering::Release,
        );
    }

    /// `SMR_SLIST_REMOVE_HEAD_LOCKED`: unlinks the first element.
    ///
    /// # Safety
    ///
    /// The caller holds the list's lock and the list is not empty.
    pub unsafe fn remove_head_locked(&self) {
        // SAFETY: the list is not empty, and a linked element is valid until unlinked.
        let first = unsafe { &*self.smr_slh_first.load(Ordering::Relaxed) };
        self.smr_slh_first.store(
            A::entry(first).smr_sle_next.load(Ordering::Relaxed),
            Ordering::Release,
        );
    }

    /// `SMR_SLIST_REMOVE_LOCKED`: unlinks `elem`, whose own link is left intact so that
    /// concurrent readers standing on it go on.
    ///
    /// # Safety
    ///
    /// The caller holds the list's lock and `elem` is in this list.
    pub unsafe fn remove_locked(&self, elem: &A::Elem) {
        let target = ptr::from_ref(elem).cast_mut();
        let next = A::entry(elem).smr_sle_next.load(Ordering::Relaxed);
        if self.smr_slh_first.load(Ordering::Relaxed) == target {
            // SAFETY: `elem` is first, so the list is not empty; the lock is held.
            unsafe { self.remove_head_locked() };
            return;
        }
        // SAFETY: `elem` is in the list and is not first, so the walk from the first element
        // reaches its predecessor through valid, linked elements.
        unsafe {
            let mut cur = &*self.smr_slh_first.load(Ordering::Relaxed);
            while A::entry(cur).smr_sle_next.load(Ordering::Relaxed) != target {
                cur = &*A::entry(cur).smr_sle_next.load(Ordering::Relaxed);
            }
            A::entry(cur).smr_sle_next.store(next, Ordering::Release);
        }
    }
}

impl<A: SmrSlistAdapter> Default for SmrSlistHead<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// Forward iterator over an [`SmrSlistHead`]; the next element is read before the current
/// one is yielded (the `_SAFE` variant).
pub struct SmrSlistIter<'a, A: Adapter> {
    cur: *mut A::Elem,
    _head: PhantomData<&'a SmrSlistHead<A>>,
}

impl<'a, A: SmrSlistAdapter> Iterator for SmrSlistIter<'a, A> {
    type Item = &'a A::Elem;

    fn next(&mut self) -> Option<&'a A::Elem> {
        // SAFETY: as for `SmrSlistHead::first`.
        let cur = unsafe { self.cur.as_ref()? };
        self.cur = A::entry(cur).smr_sle_next.load(Ordering::Acquire);
        Some(cur)
    }
}

/// `SMR_LIST_ENTRY(type)`: the links an element embeds to be in an SMR list.
#[repr(C)]
pub struct SmrListEntry<T> {
    /// `smr_le_next`: next element, SMR-protected.
    smr_le_next: AtomicPtr<T>,
    /// `smr_le_prev`: address of previous next element (or of the head's first link).
    smr_le_prev: AtomicPtr<AtomicPtr<T>>,
}

impl<T> SmrListEntry<T> {
    /// An entry that is in no list (all-zero, as the C's `M_ZERO` elements).
    pub const fn new() -> Self {
        Self {
            smr_le_next: AtomicPtr::new(ptr::null_mut()),
            smr_le_prev: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl<T> Default for SmrListEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `SMR_LIST_HEAD(name, type)`: a doubly-linked list that readers walk forward inside SMR
/// read sections while one writer, holding its lock, changes it; an element is unlinked
/// without a walk. A removed element must not be freed before a grace period (`smr_call`,
/// `smr_barrier`).
pub struct SmrListHead<A: Adapter> {
    /// `smr_lh_first`: first element, SMR-protected.
    smr_lh_first: AtomicPtr<A::Elem>,
}

impl<A: SmrListAdapter> SmrListHead<A> {
    /// `SMR_LIST_HEAD_INITIALIZER`: an empty list.
    pub const fn new() -> Self {
        Self {
            smr_lh_first: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// `SMR_LIST_INIT`.
    pub fn init(&self) {
        self.smr_lh_first.store(ptr::null_mut(), Ordering::Release);
    }

    /// `SMR_LIST_FIRST`: for a reader inside a read section.
    pub fn first(&self) -> Option<&A::Elem> {
        // SAFETY: a linked element, or one unlinked less than a grace period ago, is valid
        // while the reader stays in its read section (the writers' contract).
        unsafe { self.smr_lh_first.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_LIST_NEXT`: for a reader inside a read section.
    pub fn next(elem: &A::Elem) -> Option<&A::Elem> {
        // SAFETY: as for `first`.
        unsafe { A::entry(elem).smr_le_next.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_LIST_FOREACH`: for a reader inside a read section.
    pub fn iter(&self) -> SmrListIter<'_, A> {
        SmrListIter {
            cur: self.smr_lh_first.load(Ordering::Acquire),
            _head: PhantomData,
        }
    }

    /// `SMR_LIST_FIRST_LOCKED`: for the writer that holds the list's lock.
    pub fn first_locked(&self) -> Option<&A::Elem> {
        // SAFETY: a linked element is valid until unlinked (the mutators' contract).
        unsafe { self.smr_lh_first.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_LIST_NEXT_LOCKED`: for the writer that holds the list's lock.
    pub fn next_locked(elem: &A::Elem) -> Option<&A::Elem> {
        // SAFETY: as for `first_locked`.
        unsafe { A::entry(elem).smr_le_next.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_LIST_EMPTY_LOCKED`.
    pub fn is_empty_locked(&self) -> bool {
        self.smr_lh_first.load(Ordering::Acquire).is_null()
    }

    /// `SMR_LIST_FOREACH_LOCKED` and `SMR_LIST_FOREACH_SAFE_LOCKED`: for the writer.
    pub fn iter_locked(&self) -> SmrListIter<'_, A> {
        self.iter()
    }

    /// `SMR_LIST_INSERT_AFTER_LOCKED`: links `elem` after `listelm`, once its links are
    /// written.
    ///
    /// # Safety
    ///
    /// The caller holds the list's lock; `listelm` is in the list; `elem` is in no list of
    /// `A` and stays valid and in place until it is unlinked and a grace period has passed.
    pub unsafe fn insert_after_locked(listelm: &A::Elem, elem: &A::Elem) {
        let after = A::entry(listelm);
        let entry = A::entry(elem);
        let next = after.smr_le_next.load(Ordering::Relaxed);
        entry.smr_le_next.store(next, Ordering::Relaxed);
        // SAFETY: a linked element is valid until unlinked.
        if let Some(next) = unsafe { next.as_ref() } {
            A::entry(next)
                .smr_le_prev
                .store(link(&entry.smr_le_next), Ordering::Relaxed);
        }
        entry
            .smr_le_prev
            .store(link(&after.smr_le_next), Ordering::Relaxed);
        after
            .smr_le_next
            .store(ptr::from_ref(elem).cast_mut(), Ordering::Release);
    }

    /// `SMR_LIST_INSERT_BEFORE_LOCKED`: links `elem` before `listelm`, once its links are
    /// written.
    ///
    /// # Safety
    ///
    /// As for [`insert_after_locked`](Self::insert_after_locked).
    pub unsafe fn insert_before_locked(listelm: &A::Elem, elem: &A::Elem) {
        let before = A::entry(listelm);
        let entry = A::entry(elem);
        let prev = before.smr_le_prev.load(Ordering::Relaxed);
        entry.smr_le_prev.store(prev, Ordering::Relaxed);
        entry
            .smr_le_next
            .store(ptr::from_ref(listelm).cast_mut(), Ordering::Relaxed);
        // SAFETY: a linked element's `smr_le_prev` points at a live link: the previous
        // element's `smr_le_next` or the head's `smr_lh_first`, which stays in place while
        // the list is not empty.
        unsafe { (*prev).store(ptr::from_ref(elem).cast_mut(), Ordering::Release) };
        before
            .smr_le_prev
            .store(link(&entry.smr_le_next), Ordering::Relaxed);
    }

    /// `SMR_LIST_INSERT_HEAD_LOCKED`: links `elem` first, once its links are written.
    ///
    /// # Safety
    ///
    /// The caller holds the list's lock; `elem` is in no list of `A` and stays valid and in
    /// place until it is unlinked and a grace period has passed; the head stays in place
    /// while the list is not empty.
    pub unsafe fn insert_head_locked(&self, elem: &A::Elem) {
        let entry = A::entry(elem);
        let first = self.smr_lh_first.load(Ordering::Relaxed);
        entry.smr_le_next.store(first, Ordering::Relaxed);
        entry
            .smr_le_prev
            .store(link(&self.smr_lh_first), Ordering::Relaxed);
        // SAFETY: a linked element is valid until unlinked.
        if let Some(first) = unsafe { first.as_ref() } {
            A::entry(first)
                .smr_le_prev
                .store(link(&entry.smr_le_next), Ordering::Relaxed);
        }
        self.smr_lh_first
            .store(ptr::from_ref(elem).cast_mut(), Ordering::Release);
    }

    /// `SMR_LIST_REMOVE_LOCKED`: unlinks `elem`, whose forward link is left intact so that
    /// concurrent readers standing on it go on.
    ///
    /// # Safety
    ///
    /// The caller holds the lock of the list `elem` is in.
    pub unsafe fn remove_locked(elem: &A::Elem) {
        let entry = A::entry(elem);
        let next = entry.smr_le_next.load(Ordering::Relaxed);
        let prev = entry.smr_le_prev.load(Ordering::Relaxed);
        // SAFETY: a linked element is valid until unlinked, and its `smr_le_prev` points at a
        // live link (see `insert_before_locked`).
        unsafe {
            if let Some(next) = next.as_ref() {
                A::entry(next).smr_le_prev.store(prev, Ordering::Relaxed);
            }
            (*prev).store(next, Ordering::Release);
        }
    }
}

impl<A: SmrListAdapter> Default for SmrListHead<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// Forward iterator over an [`SmrListHead`]; the next element is read before the current
/// one is yielded (the `_SAFE` variant).
pub struct SmrListIter<'a, A: Adapter> {
    cur: *mut A::Elem,
    _head: PhantomData<&'a SmrListHead<A>>,
}

impl<'a, A: SmrListAdapter> Iterator for SmrListIter<'a, A> {
    type Item = &'a A::Elem;

    fn next(&mut self) -> Option<&'a A::Elem> {
        // SAFETY: as for `SmrListHead::first`.
        let cur = unsafe { self.cur.as_ref()? };
        self.cur = A::entry(cur).smr_le_next.load(Ordering::Acquire);
        Some(cur)
    }
}

/// `SMR_TAILQ_ENTRY(type)`: the links an element embeds to be in an SMR tail queue.
#[repr(C)]
pub struct SmrTailqEntry<T> {
    /// `smr_tqe_next`: next element, SMR-protected.
    smr_tqe_next: AtomicPtr<T>,
    /// `smr_tqe_prev`: address of previous next element (or of the head's first link).
    smr_tqe_prev: AtomicPtr<AtomicPtr<T>>,
}

impl<T> SmrTailqEntry<T> {
    /// An entry that is in no queue (all-zero, as the C's `M_ZERO` elements).
    pub const fn new() -> Self {
        Self {
            smr_tqe_next: AtomicPtr::new(ptr::null_mut()),
            smr_tqe_prev: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl<T> Default for SmrTailqEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `SMR_TAILQ_HEAD(name, type)`: a doubly-linked queue with a tail pointer that readers
/// walk forward inside SMR read sections while one writer, holding its lock, changes it.
/// A removed element must not be freed before a grace period (`smr_call`, `smr_barrier`).
pub struct SmrTailqHead<A: Adapter> {
    /// `smr_tqh_first`: first element, SMR-protected.
    smr_tqh_first: AtomicPtr<A::Elem>,
    /// `smr_tqh_last`: address of the last element's `smr_tqe_next`; null stands for
    /// `&smr_tqh_first` (empty queue).
    smr_tqh_last: AtomicPtr<AtomicPtr<A::Elem>>,
}

impl<A: SmrTailqAdapter> SmrTailqHead<A> {
    /// `SMR_TAILQ_HEAD_INITIALIZER`: an empty queue.
    pub const fn new() -> Self {
        Self {
            smr_tqh_first: AtomicPtr::new(ptr::null_mut()),
            smr_tqh_last: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// `SMR_TAILQ_INIT`.
    pub fn init(&self) {
        self.smr_tqh_first.store(ptr::null_mut(), Ordering::Release);
        self.smr_tqh_last.store(ptr::null_mut(), Ordering::Relaxed);
    }

    /// `SMR_TAILQ_FIRST`: for a reader inside a read section.
    pub fn first(&self) -> Option<&A::Elem> {
        // SAFETY: a linked element, or one unlinked less than a grace period ago, is valid
        // while the reader stays in its read section (the writers' contract).
        unsafe { self.smr_tqh_first.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_TAILQ_NEXT`: for a reader inside a read section.
    pub fn next(elem: &A::Elem) -> Option<&A::Elem> {
        // SAFETY: as for `first`.
        unsafe { A::entry(elem).smr_tqe_next.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_TAILQ_FOREACH`: for a reader inside a read section.
    pub fn iter(&self) -> SmrTailqIter<'_, A> {
        SmrTailqIter {
            cur: self.smr_tqh_first.load(Ordering::Acquire),
            _head: PhantomData,
        }
    }

    /// `SMR_TAILQ_FIRST_LOCKED`: for the writer that holds the queue's lock.
    pub fn first_locked(&self) -> Option<&A::Elem> {
        // SAFETY: a linked element is valid until unlinked (the mutators' contract).
        unsafe { self.smr_tqh_first.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_TAILQ_NEXT_LOCKED`: for the writer that holds the queue's lock.
    pub fn next_locked(elem: &A::Elem) -> Option<&A::Elem> {
        // SAFETY: as for `first_locked`.
        unsafe { A::entry(elem).smr_tqe_next.load(Ordering::Acquire).as_ref() }
    }

    /// `SMR_TAILQ_LAST_LOCKED`: the last element, for the writer that holds the queue's
    /// lock.
    pub fn last_locked(&self) -> Option<&A::Elem> {
        let last = self.smr_tqh_last.load(Ordering::Relaxed);
        if last.is_null() {
            return None;
        }
        // SAFETY: a non-null `smr_tqh_last` is the `smr_tqe_next` of the last element, the
        // first field of its `#[repr(C)]` entry, so it is that entry's address too (the
        // C reads the entry as a head for the same reason). The entry's `smr_tqe_prev`
        // points at the link that designates the last element: its predecessor's
        // `smr_tqe_next` or the head's `smr_tqh_first`, both live under the lock.
        unsafe {
            let entry = &*last.cast::<SmrTailqEntry<A::Elem>>();
            (*entry.smr_tqe_prev.load(Ordering::Relaxed))
                .load(Ordering::Relaxed)
                .as_ref()
        }
    }

    /// `SMR_TAILQ_EMPTY_LOCKED`.
    pub fn is_empty_locked(&self) -> bool {
        self.smr_tqh_first.load(Ordering::Acquire).is_null()
    }

    /// `SMR_TAILQ_FOREACH_LOCKED` and `SMR_TAILQ_FOREACH_SAFE_LOCKED`: for the writer.
    pub fn iter_locked(&self) -> SmrTailqIter<'_, A> {
        self.iter()
    }

    /// The link `smr_tqh_last` designates: the last element's `smr_tqe_next`, or
    /// `smr_tqh_first`.
    fn last_link(&self) -> *mut AtomicPtr<A::Elem> {
        let last = self.smr_tqh_last.load(Ordering::Relaxed);
        if last.is_null() {
            link(&self.smr_tqh_first)
        } else {
            last
        }
    }

    /// Stores `to` as `smr_tqh_last`, folding the head's own link back into the null
    /// sentinel.
    fn set_last(&self, to: *mut AtomicPtr<A::Elem>) {
        if ptr::eq(to, &self.smr_tqh_first) {
            self.smr_tqh_last.store(ptr::null_mut(), Ordering::Relaxed);
        } else {
            self.smr_tqh_last.store(to, Ordering::Relaxed);
        }
    }

    /// `SMR_TAILQ_INSERT_AFTER_LOCKED`: links `elem` after `listelm`, once its links are
    /// written.
    ///
    /// # Safety
    ///
    /// The caller holds the queue's lock; `listelm` is in this queue; `elem` is in no queue
    /// of `A` and stays valid and in place until it is unlinked and a grace period has
    /// passed.
    pub unsafe fn insert_after_locked(&self, listelm: &A::Elem, elem: &A::Elem) {
        let after = A::entry(listelm);
        let entry = A::entry(elem);
        let next = after.smr_tqe_next.load(Ordering::Relaxed);
        entry.smr_tqe_next.store(next, Ordering::Relaxed);
        // SAFETY: a linked element is valid until unlinked.
        match unsafe { next.as_ref() } {
            Some(next) => A::entry(next)
                .smr_tqe_prev
                .store(link(&entry.smr_tqe_next), Ordering::Relaxed),
            None => self
                .smr_tqh_last
                .store(link(&entry.smr_tqe_next), Ordering::Relaxed),
        }
        entry
            .smr_tqe_prev
            .store(link(&after.smr_tqe_next), Ordering::Relaxed);
        after
            .smr_tqe_next
            .store(ptr::from_ref(elem).cast_mut(), Ordering::Release);
    }

    /// `SMR_TAILQ_INSERT_BEFORE_LOCKED`: links `elem` before `listelm`, once its links are
    /// written.
    ///
    /// # Safety
    ///
    /// The caller holds the lock of the queue `listelm` is in; `elem` is in no queue of `A`
    /// and stays valid and in place until it is unlinked and a grace period has passed.
    pub unsafe fn insert_before_locked(listelm: &A::Elem, elem: &A::Elem) {
        let before = A::entry(listelm);
        let entry = A::entry(elem);
        let prev = before.smr_tqe_prev.load(Ordering::Relaxed);
        entry.smr_tqe_prev.store(prev, Ordering::Relaxed);
        entry
            .smr_tqe_next
            .store(ptr::from_ref(listelm).cast_mut(), Ordering::Relaxed);
        // SAFETY: a linked element's `smr_tqe_prev` points at a live link: the previous
        // element's `smr_tqe_next` or the head's `smr_tqh_first`, which stays in place while
        // the queue is not empty.
        unsafe { (*prev).store(ptr::from_ref(elem).cast_mut(), Ordering::Release) };
        before
            .smr_tqe_prev
            .store(link(&entry.smr_tqe_next), Ordering::Relaxed);
    }

    /// `SMR_TAILQ_INSERT_HEAD_LOCKED`: links `elem` first, once its links are written.
    ///
    /// # Safety
    ///
    /// The caller holds the queue's lock; `elem` is in no queue of `A` and stays valid and
    /// in place until it is unlinked and a grace period has passed; the head stays in place
    /// while the queue is not empty.
    pub unsafe fn insert_head_locked(&self, elem: &A::Elem) {
        let entry = A::entry(elem);
        let first = self.smr_tqh_first.load(Ordering::Relaxed);
        entry.smr_tqe_next.store(first, Ordering::Relaxed);
        entry
            .smr_tqe_prev
            .store(link(&self.smr_tqh_first), Ordering::Relaxed);
        // SAFETY: a linked element is valid until unlinked.
        match unsafe { first.as_ref() } {
            Some(first) => A::entry(first)
                .smr_tqe_prev
                .store(link(&entry.smr_tqe_next), Ordering::Relaxed),
            None => self
                .smr_tqh_last
                .store(link(&entry.smr_tqe_next), Ordering::Relaxed),
        }
        self.smr_tqh_first
            .store(ptr::from_ref(elem).cast_mut(), Ordering::Release);
    }

    /// `SMR_TAILQ_INSERT_TAIL_LOCKED`: links `elem` last, once its links are written.
    ///
    /// # Safety
    ///
    /// As for [`insert_head_locked`](Self::insert_head_locked).
    pub unsafe fn insert_tail_locked(&self, elem: &A::Elem) {
        let entry = A::entry(elem);
        let last = self.last_link();
        entry.smr_tqe_next.store(ptr::null_mut(), Ordering::Relaxed);
        entry.smr_tqe_prev.store(last, Ordering::Relaxed);
        // SAFETY: `last_link` is the head's `smr_tqh_first` or the `smr_tqe_next` of the last
        // element, linked and so valid.
        unsafe { (*last).store(ptr::from_ref(elem).cast_mut(), Ordering::Release) };
        self.smr_tqh_last
            .store(link(&entry.smr_tqe_next), Ordering::Relaxed);
    }

    /// `SMR_TAILQ_REMOVE_LOCKED`: unlinks `elem`, whose forward link is left intact so that
    /// concurrent readers standing on it go on.
    ///
    /// # Safety
    ///
    /// The caller holds the queue's lock and `elem` is in this queue.
    pub unsafe fn remove_locked(&self, elem: &A::Elem) {
        let entry = A::entry(elem);
        let next = entry.smr_tqe_next.load(Ordering::Relaxed);
        let prev = entry.smr_tqe_prev.load(Ordering::Relaxed);
        // SAFETY: a linked element is valid until unlinked, and its `smr_tqe_prev` points at
        // a live link (see `insert_before_locked`).
        unsafe {
            match next.as_ref() {
                Some(next) => A::entry(next).smr_tqe_prev.store(prev, Ordering::Relaxed),
                None => self.set_last(prev),
            }
            (*prev).store(next, Ordering::Release);
        }
    }
}

impl<A: SmrTailqAdapter> Default for SmrTailqHead<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// Forward iterator over an [`SmrTailqHead`]; the next element is read before the current
/// one is yielded (the `_SAFE` variant).
pub struct SmrTailqIter<'a, A: Adapter> {
    cur: *mut A::Elem,
    _head: PhantomData<&'a SmrTailqHead<A>>,
}

impl<'a, A: SmrTailqAdapter> Iterator for SmrTailqIter<'a, A> {
    type Item = &'a A::Elem;

    fn next(&mut self) -> Option<&'a A::Elem> {
        // SAFETY: as for `SmrTailqHead::first`.
        let cur = unsafe { self.cur.as_ref()? };
        self.cur = A::entry(cur).smr_tqe_next.load(Ordering::Acquire);
        Some(cur)
    }
}

/// An [`Adapter`] whose entry is an [`SmrSlistEntry`].
pub trait SmrSlistAdapter: Adapter<Entry = SmrSlistEntry<<Self as Adapter>::Elem>> {}
impl<A: Adapter<Entry = SmrSlistEntry<<A as Adapter>::Elem>>> SmrSlistAdapter for A {}

/// An [`Adapter`] whose entry is an [`SmrListEntry`].
pub trait SmrListAdapter: Adapter<Entry = SmrListEntry<<Self as Adapter>::Elem>> {}
impl<A: Adapter<Entry = SmrListEntry<<A as Adapter>::Elem>>> SmrListAdapter for A {}

/// An [`Adapter`] whose entry is an [`SmrTailqEntry`].
pub trait SmrTailqAdapter: Adapter<Entry = SmrTailqEntry<<Self as Adapter>::Elem>> {}
impl<A: Adapter<Entry = SmrTailqEntry<<A as Adapter>::Elem>>> SmrTailqAdapter for A {}

/// The address of a forward link, as the backward links and a queue's `last` hold it. A
/// link is only ever written through its atomic's shared methods.
fn link<T>(l: &AtomicPtr<T>) -> *mut AtomicPtr<T> {
    ptr::from_ref(l).cast_mut()
}

/// `smr_init(smr)`: an idle entry.
#[inline]
pub fn smr_init(smr: &SmrEntry) {
    smr.smr_func.set(None);
    smr.smr_arg.set(ptr::null_mut());
}

/// `smr_call(entry, func, arg)`: `func(arg)` once every CPU has left its read sections.
#[inline]
pub fn smr_call(entry: &'static SmrEntry, func: fn(*mut c_void), arg: *mut c_void) {
    crate::kern::kern_smr::smr_call_impl(entry, func, arg, false);
}

/// `smr_barrier()`: waits until every read section that started before it has ended.
#[inline]
pub fn smr_barrier() {
    crate::kern::kern_smr::smr_barrier_impl(false);
}

/// `smr_flush()`: `smr_barrier` without the pause between rounds.
#[inline]
pub fn smr_flush() {
    crate::kern::kern_smr::smr_barrier_impl(true);
}

/// `SMR_ASSERT_CRITICAL()` (`DIAGNOSTIC`): inside a read section.
#[inline]
pub fn smr_assert_critical() {
    #[cfg(feature = "diagnostic")]
    if !crate::kern::subr_prf::panicstr()
        && !crate::kern::init_main::DB_ACTIVE.load(Ordering::Relaxed)
    {
        crate::kassert!(smr_depth() > 0);
    }
}

/// `SMR_ASSERT_NONCRITICAL()` (`DIAGNOSTIC`): outside every read section.
#[inline]
pub fn smr_assert_noncritical() {
    #[cfg(feature = "diagnostic")]
    if !crate::kern::subr_prf::panicstr()
        && !crate::kern::init_main::DB_ACTIVE.load(Ordering::Relaxed)
    {
        crate::kassert!(smr_depth() == 0);
    }
}

/// `curcpu()->ci_schedstate.spc_smrdepth`.
#[cfg(feature = "diagnostic")]
fn smr_depth() -> u32 {
    use crate::machine::Machine;
    use crate::machine::cpu::{Cpu, curcpu};
    Machine::ci_schedstate(curcpu()).spc_smrdepth.get()
}

// `SmrTailqHead::last_locked` reads an entry through the address of its forward link.
const _: () = {
    assert!(core::mem::offset_of!(SmrTailqEntry<u8>, smr_tqe_next) == 0);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_start_idle() {
        let e = SmrEntry::new();
        assert!(e.smr_func.get().is_none());
        e.smr_func.set(Some(|_| {}));
        e.smr_arg.set(ptr::from_ref(&e).cast_mut().cast());
        smr_init(&e);
        assert!(e.smr_func.get().is_none());
        assert!(e.smr_arg.get().is_null());
    }

    #[test]
    fn smr_ptr_publishes() {
        let mut x = 5;
        let p: SmrPtr<i32> = SmrPtr::new();
        assert!(p.get().is_null());
        p.set_locked(&mut x);
        assert_eq!(p.get(), ptr::from_mut(&mut x));
        assert_eq!(p.get_locked(), p.get());
    }

    #[test]
    fn smr_slist_links_and_unlinks() {
        struct E {
            link: SmrSlistEntry<E>,
            v: i32,
        }
        queue_adapter!(EList: E, link => SmrSlistEntry<E>);
        let (a, b, c) = (
            E {
                link: SmrSlistEntry::new(),
                v: 1,
            },
            E {
                link: SmrSlistEntry::new(),
                v: 2,
            },
            E {
                link: SmrSlistEntry::new(),
                v: 3,
            },
        );
        let head: SmrSlistHead<EList> = SmrSlistHead::new();
        assert!(head.is_empty_locked());
        // SAFETY: the elements outlive the list and are linked once.
        unsafe {
            head.insert_head_locked(&a);
            head.insert_head_locked(&b);
            head.insert_head_locked(&c);
        }
        let v: std::vec::Vec<i32> = head.iter().map(|e| e.v).collect();
        assert_eq!(v, [3, 2, 1]);
        // SAFETY: `b` is in the list.
        unsafe { head.remove_locked(&b) };
        // A reader standing on the removed element still reaches the rest.
        assert!(SmrSlistHead::<EList>::next(&b).is_some_and(|e| e.v == 1));
        let v: std::vec::Vec<i32> = head.iter_locked().map(|e| e.v).collect();
        assert_eq!(v, [3, 1]);
        // SAFETY: `c` is first.
        unsafe { head.remove_locked(&c) };
        assert!(head.first().is_some_and(|e| e.v == 1));
        assert!(head.first_locked().is_some_and(|e| e.v == 1));
    }

    #[test]
    fn smr_slist_inserts_and_removes_after_an_element() {
        struct E {
            link: SmrSlistEntry<E>,
            v: i32,
        }
        queue_adapter!(EList: E, link => SmrSlistEntry<E>);
        let e = |v| E {
            link: SmrSlistEntry::new(),
            v,
        };
        let (a, b, c, d) = (e(1), e(2), e(3), e(4));
        let head: SmrSlistHead<EList> = SmrSlistHead::new();
        let order = || head.iter().map(|e| e.v).collect::<std::vec::Vec<i32>>();
        // SAFETY: the elements outlive the list, each is linked once, and what is removed
        // or named as a predecessor is in the list.
        unsafe {
            head.insert_head_locked(&a);
            SmrSlistHead::<EList>::insert_after_locked(&a, &c);
            SmrSlistHead::<EList>::insert_after_locked(&a, &b);
            SmrSlistHead::<EList>::insert_after_locked(&c, &d);
            assert_eq!(order(), [1, 2, 3, 4]);
            assert!(SmrSlistHead::<EList>::next_locked(&b).is_some_and(|e| e.v == 3));
            assert!(SmrSlistHead::<EList>::next_locked(&d).is_none());
            SmrSlistHead::<EList>::remove_after_locked(&b);
            assert_eq!(order(), [1, 2, 4]);
            // The removed element still leads a reader to the rest.
            assert!(SmrSlistHead::<EList>::next(&c).is_some_and(|e| e.v == 4));
            head.remove_head_locked();
            assert_eq!(order(), [2, 4]);
            head.remove_head_locked();
            head.remove_head_locked();
        }
        assert!(head.is_empty_locked());
        assert!(head.first().is_none());
    }

    #[test]
    fn smr_list_links_and_unlinks() {
        struct E {
            link: SmrListEntry<E>,
            v: i32,
        }
        queue_adapter!(EList: E, link => SmrListEntry<E>);
        type L = SmrListHead<EList>;
        let e = |v| E {
            link: SmrListEntry::new(),
            v,
        };
        let (a, b, c, d, f) = (e(1), e(2), e(3), e(4), e(5));
        let head: L = SmrListHead::new();
        assert!(head.is_empty_locked());
        assert!(head.first().is_none());
        let order = || head.iter().map(|e| e.v).collect::<std::vec::Vec<i32>>();
        // SAFETY: the elements and the head outlive the list and stay in place, each element
        // is linked once, and what is removed or named as a neighbour is in the list.
        unsafe {
            head.insert_head_locked(&c);
            head.insert_head_locked(&a);
            assert_eq!(order(), [1, 3]);
            L::insert_after_locked(&a, &b);
            L::insert_after_locked(&c, &f);
            L::insert_before_locked(&f, &d);
            assert_eq!(order(), [1, 2, 3, 4, 5]);
            // Before the first element: through the head's link.
            let z = e(0);
            L::insert_before_locked(&a, &z);
            assert_eq!(order(), [0, 1, 2, 3, 4, 5]);
            assert!(head.first_locked().is_some_and(|e| e.v == 0));
            L::remove_locked(&z);
            assert_eq!(order(), [1, 2, 3, 4, 5]);
            assert!(L::next_locked(&a).is_some_and(|e| e.v == 2));

            // The middle, the last and the first element, each without a walk.
            L::remove_locked(&c);
            assert_eq!(order(), [1, 2, 4, 5]);
            // A reader standing on the removed element still reaches the rest.
            assert!(L::next(&c).is_some_and(|e| e.v == 4));
            L::remove_locked(&f);
            assert_eq!(order(), [1, 2, 4]);
            L::remove_locked(&a);
            assert_eq!(order(), [2, 4]);
            // The backward links followed the changes: both ends can still be inserted at.
            L::insert_before_locked(&b, &a);
            L::insert_after_locked(&d, &f);
            assert_eq!(order(), [1, 2, 4, 5]);
            // Unlinking while iterating, as `SMR_LIST_FOREACH_SAFE_LOCKED` allows.
            for x in head.iter_locked() {
                L::remove_locked(x);
            }
        }
        assert!(head.is_empty_locked());
        head.init();
        assert!(head.first_locked().is_none());
    }

    #[test]
    fn smr_tailq_links_and_unlinks() {
        struct E {
            link: SmrTailqEntry<E>,
            v: i32,
        }
        queue_adapter!(EQueue: E, link => SmrTailqEntry<E>);
        type Q = SmrTailqHead<EQueue>;
        let e = |v| E {
            link: SmrTailqEntry::new(),
            v,
        };
        let (a, b, c, d, f) = (e(1), e(2), e(3), e(4), e(5));
        let head: Q = SmrTailqHead::new();
        assert!(head.is_empty_locked());
        assert!(head.last_locked().is_none());
        let order = || head.iter().map(|e| e.v).collect::<std::vec::Vec<i32>>();
        let last = || head.last_locked().map(|e| e.v);
        // SAFETY: the elements and the head outlive the queue and stay in place, each
        // element is linked once, and what is removed or named as a neighbour is in the
        // queue.
        unsafe {
            head.insert_tail_locked(&c);
            assert_eq!((order(), last()), (std::vec![3], Some(3)));
            head.insert_tail_locked(&d);
            head.insert_head_locked(&a);
            assert_eq!((order(), last()), (std::vec![1, 3, 4], Some(4)));
            head.insert_after_locked(&a, &b);
            assert_eq!((order(), last()), (std::vec![1, 2, 3, 4], Some(4)));
            // After the last element: the tail moves.
            head.insert_after_locked(&d, &f);
            assert_eq!((order(), last()), (std::vec![1, 2, 3, 4, 5], Some(5)));
            let z = e(0);
            Q::insert_before_locked(&a, &z);
            assert_eq!(order(), [0, 1, 2, 3, 4, 5]);
            head.remove_locked(&z);
            assert!(Q::next_locked(&d).is_some_and(|e| e.v == 5));
            assert!(Q::next_locked(&f).is_none());

            head.remove_locked(&c);
            assert_eq!((order(), last()), (std::vec![1, 2, 4, 5], Some(5)));
            // A reader standing on the removed element still reaches the rest.
            assert!(Q::next(&c).is_some_and(|e| e.v == 4));
            // The last element: the tail moves back.
            head.remove_locked(&f);
            assert_eq!((order(), last()), (std::vec![1, 2, 4], Some(4)));
            head.insert_tail_locked(&f);
            assert_eq!((order(), last()), (std::vec![1, 2, 4, 5], Some(5)));
            head.remove_locked(&a);
            assert_eq!(head.first_locked().map(|e| e.v), Some(2));
            // Unlinking while iterating, down to the empty queue and its null tail.
            for x in head.iter_locked() {
                head.remove_locked(x);
            }
            assert!(head.is_empty_locked());
            assert!(head.last_locked().is_none());
            // An emptied queue takes elements again, at either end.
            head.insert_head_locked(&b);
            assert_eq!((order(), last()), (std::vec![2], Some(2)));
            head.remove_locked(&b);
            head.insert_tail_locked(&a);
            assert_eq!((order(), last()), (std::vec![1], Some(1)));
        }
        head.init();
        assert!(head.first().is_none());
        assert!(head.last_locked().is_none());
    }
}
/* </TESTS> */
