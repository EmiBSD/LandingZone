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
 * Copyright (c) 1982, 1986, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)kern_subr.c	8.3 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! Kernel subroutines: `kern/kern_subr.c`.
//!
//! Upstream: sys/kern/kern_subr.c @ 3ce1f3f79392
//!
//! Status: `ported`. Milestone M5 ports `hashinit` and `hashfree`; M7b (`kern_descrip.c`) adds
//! `uiomove` and `ureadc`; TD1 the hook lists (`hook_establish`, `hook_disestablish`,
//! `dohooks`, with `<sys/systm.h>`'s `startuphook_*` and `dostartuphooks`).
//!
//! ## Deviations
//! - `hashinit` returns the table as a slice (its length is `hashmask + 1`) instead of a
//!   pointer plus an out-parameter mask; callers mask with `len() - 1`.
//! - `uiomove(cp, n, uio)` takes the kernel buffer as a slice whose length is `n`.
//! - Under `PMAP_CHECK_COPYIN` the C defines `copyin` and `copyinstr` here, over the
//!   machine's `_copyin` and `_copyinstr`. `check_copyin` is here; the two wrappers are the
//!   entry points every caller already uses, `machine::copy::copyin` and `copyinstr`
//!   (`sys/machine/copy.rs`), which make the check when the machine's
//!   `pmap_check_copyin()` says so.

use core::cell::Cell;
use core::ffi::c_void;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering;

use crate::kassert;
use crate::kern::kern_malloc::{free, malloc, mallocarray};
use crate::kern::sched_bsd::preempt;
use crate::kern::subr_prf::panic;
use crate::machine::copy::{copyin, copyout, kcopy};
use crate::sys::errno::Errno;
use crate::sys::malloc::{M_DEVBUF, M_NOWAIT};
use crate::sys::proc::Proc;
use crate::sys::queue::{ListAdapter, ListHead, TailqEntry, TailqHead};
use crate::sys::sched::sched_pause;
use crate::sys::systm::{HOOK_FREE, HOOK_REMOVE, HdList, HookDesc, HookDescHead};
use crate::sys::types::Off;
use crate::sys::uio::{Uio, UioRw, UioSeg};

/// `startuphook_list`: "startup hook" types, functions, and variables.
pub static STARTUPHOOK_LIST: HookDescHead = HookDescHead::new();

/// `uio->uio_iov++; uio->uio_iovcnt--`: steps past the first iovec.
fn uio_next_iov(uio: &mut Uio<'_>) {
    let iov = core::mem::take(&mut uio.uio_iov);
    uio.uio_iov = &mut iov[1..];
}

/// `check_copyin(p, vstart, len)`: `EFAULT` if the range overlaps one of the regions the
/// process's map lists for copyin(9) (its execute-only text, under `PMAP_CHECK_COPYIN`).
pub fn check_copyin(p: &Proc, vstart: usize, len: usize) -> Result<(), Errno> {
    let map = &p.vmspace().vm_map;
    let start = vstart;
    let end = start.wrapping_add(len);

    // XXX if the array was sorted, we could shortcut
    // membar_consumer(): an entry is written before the count shows it.
    let max = map.check_copyin_count.load(Ordering::Acquire);
    for region in map.check_copyin.iter().take(max as usize) {
        let s = region.start.get();
        let e = region.end.get();
        if (start >= s && start < e) || (end > s && end < e) {
            return Err(Errno::EFAULT);
        }
    }
    Ok(())
}

/// `uiomove(cp, n, uio)`: moves `cp.len()` bytes (at most `uio_resid`) between the kernel
/// buffer `cp` and the iovecs of `uio`, in the direction `uio_rw` says (`UIO_READ`: to the
/// iovecs), advancing the iovecs, `uio_resid` and `uio_offset`.
pub fn uiomove(cp: &mut [u8], uio: &mut Uio<'_>) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if uio.uio_segflg == UioSeg::UIO_USERSPACE
        && !ptr::eq(
            uio.uio_procp.map_or(ptr::null(), ptr::from_ref),
            crate::machine::cpu::curproc().map_or(ptr::null(), ptr::from_ref),
        )
    {
        panic(format_args!("uiomove: proc"));
    }

    let mut n = cp.len().min(uio.uio_resid);
    let mut done = 0;

    while n > 0 {
        if uio.uio_iov.is_empty() {
            panic(format_args!(
                "uiomove: resid {} past the iovecs",
                uio.uio_resid
            ));
        }
        let mut cnt = uio.uio_iov[0].iov_len;
        if cnt == 0 {
            kassert!(uio.uio_iovcnt() > 0);
            uio_next_iov(uio);
            continue;
        }
        if cnt > n {
            cnt = n;
        }
        let iov = &mut uio.uio_iov[0];
        let buf = &mut cp[done..done + cnt];
        match uio.uio_segflg {
            UioSeg::UIO_USERSPACE => {
                sched_pause(preempt);
                if uio.uio_rw == UioRw::UIO_READ {
                    copyout(buf, iov.iov_base as usize)?;
                } else {
                    copyin(iov.iov_base as usize, buf)?;
                }
            }
            UioSeg::UIO_SYSSPACE => {
                if uio.uio_rw == UioRw::UIO_READ {
                    // SAFETY: a UIO_SYSSPACE iovec is a kernel buffer of at least `iov_len`
                    // bytes its builder vouches for (`struct uio`'s contract); `buf` is ours.
                    unsafe { kcopy(buf.as_ptr(), iov.iov_base.cast(), cnt) }?;
                } else {
                    // SAFETY: as above.
                    unsafe { kcopy(iov.iov_base.cast_const().cast(), buf.as_mut_ptr(), cnt) }?;
                }
            }
        }
        iov.iov_base = iov.iov_base.wrapping_byte_add(cnt);
        iov.iov_len -= cnt;
        uio.uio_resid -= cnt;
        uio.uio_offset += cnt as Off;
        done += cnt;
        n -= cnt;
    }
    Ok(())
}

/// Give next character to user as result of read.
pub fn ureadc(c: i32, uio: &mut Uio<'_>) -> Result<(), Errno> {
    if uio.uio_resid == 0 {
        #[cfg(feature = "diagnostic")]
        panic(format_args!("ureadc: zero resid"));
        #[cfg(not(feature = "diagnostic"))]
        return Err(Errno::EINVAL);
    }
    loop {
        if uio.uio_iovcnt() == 0 {
            #[cfg(feature = "diagnostic")]
            panic(format_args!("ureadc: non-positive iovcnt"));
            #[cfg(not(feature = "diagnostic"))]
            return Err(Errno::EINVAL);
        }
        if uio.uio_iov[0].iov_len == 0 {
            uio_next_iov(uio);
            continue;
        }
        break;
    }
    let iov = &mut uio.uio_iov[0];
    match uio.uio_segflg {
        UioSeg::UIO_USERSPACE => {
            let tmp = [c as u8];
            if copyout(&tmp, iov.iov_base as usize).is_err() {
                return Err(Errno::EFAULT);
            }
        }
        UioSeg::UIO_SYSSPACE => {
            // SAFETY: a UIO_SYSSPACE iovec is a kernel buffer of at least `iov_len` (> 0)
            // bytes its builder vouches for (`struct uio`'s contract).
            unsafe { iov.iov_base.cast::<u8>().write(c as u8) };
        }
    }
    iov.iov_base = iov.iov_base.wrapping_byte_add(1);
    iov.iov_len -= 1;
    uio.uio_resid -= 1;
    uio.uio_offset += 1;
    Ok(())
}

/// The power of two the hash table of `elements` rounds up to.
pub fn hashsize(elements: i32) -> usize {
    let elements = elements as usize;
    if elements & (elements - 1) == 0 {
        elements
    } else {
        let mut hashsize = 1;
        while hashsize < elements {
            hashsize <<= 1;
        }
        hashsize
    }
}

/// `hashinit`: a table of `elements` (rounded up to a power of two) empty lists, allocated
/// from `type_` with `flags`; `None` when the allocation failed.
pub fn hashinit<A: ListAdapter>(
    elements: i32,
    type_: i32,
    flags: i32,
) -> Option<&'static [ListHead<A>]> {
    if elements <= 0 {
        panic(format_args!("hashinit: bad cnt"));
    }
    let hashsize = hashsize(elements);
    let hashtbl =
        mallocarray(hashsize, size_of::<ListHead<A>>(), type_, flags)?.cast::<ListHead<A>>();
    for i in 0..hashsize {
        // SAFETY: `hashsize` heads were just allocated at `hashtbl`.
        unsafe { ptr::write(hashtbl.as_ptr().add(i), ListHead::new()) };
    }
    // SAFETY: the heads are initialised and the allocation is never freed while the table
    // is in use (`hashfree` takes it back).
    Some(unsafe { core::slice::from_raw_parts(hashtbl.as_ptr(), hashsize) })
}

/// `hashfree`: releases a table `hashinit` made for `elements`.
///
/// # Safety
///
/// `hash` came from `hashinit` with the same `elements` and `type_`, its lists are empty,
/// and nothing uses it afterwards.
pub unsafe fn hashfree<A: ListAdapter>(hash: &'static [ListHead<A>], elements: i32, type_: i32) {
    if elements <= 0 {
        panic(format_args!("hashfree: bad cnt"));
    }
    let hashsize = hashsize(elements);
    let Some(p) = NonNull::new(hash.as_ptr().cast_mut().cast::<u8>()) else {
        return;
    };
    free(p, type_, hashsize * size_of::<ListHead<A>>());
}

/// `hook_establish(head, tail, fn, arg)`: adds a hook to a list, at its tail or its head, and
/// returns it for `hook_disestablish`; `None` without memory.
pub fn hook_establish(
    head: &HookDescHead,
    tail: bool,
    func: fn(*mut c_void),
    arg: *mut c_void,
) -> Option<NonNull<HookDesc>> {
    let hdp = malloc(size_of::<HookDesc>(), M_DEVBUF, M_NOWAIT)?.cast::<HookDesc>();
    // SAFETY: a fresh malloc(9) block of the descriptor's size and alignment, written whole
    // before anything reads it; it lives until `hook_disestablish` or `dohooks` frees it.
    let hd = unsafe {
        hdp.as_ptr().write(HookDesc {
            hd_list: TailqEntry::new(),
            hd_fn: Cell::new(Some(func)),
            hd_arg: Cell::new(arg),
        });
        hdp.as_ref()
    };
    // SAFETY: the descriptor is on no list yet.
    unsafe {
        if tail {
            head.0.insert_tail(hd);
        } else {
            head.0.insert_head(hd);
        }
    }
    Some(hdp)
}

/// `hook_disestablish(head, vhook)`: takes a hook off its list and frees it. Under
/// `DIAGNOSTIC` a hook that is not on the list is left alone.
///
/// # Safety
///
/// `vhook` came from `hook_establish` on `head` and has not been freed, by this function or
/// by a `dohooks` with `HOOK_FREE`.
pub unsafe fn hook_disestablish(head: &HookDescHead, vhook: NonNull<HookDesc>) {
    #[cfg(feature = "diagnostic")]
    if !head.0.iter().any(|hdp| ptr::eq(hdp, vhook.as_ptr())) {
        return;
    }
    // SAFETY: the caller's guarantee: a live descriptor on this list.
    unsafe {
        head.0.remove(vhook.as_ref());
    }
    free(vhook.cast::<u8>(), M_DEVBUF, size_of::<HookDesc>());
}

/// `dohooks(head, flags)`: run hooks. Startup hooks are invoked right after scheduler_start
/// but before root is mounted. With `HOOK_REMOVE` each hook is taken off the list first, and
/// with `HOOK_FREE` too it is freed after it ran.
pub fn dohooks(head: &HookDescHead, flags: i32) {
    if flags & HOOK_REMOVE == 0 {
        // TAILQ_FOREACH_SAFE: a hook may take itself off the list.
        let mut hdp = head.0.first();
        while let Some(hd) = hdp {
            hdp = TailqHead::<HdList>::next(hd);
            if let Some(func) = hd.hd_fn.get() {
                func(hd.hd_arg.get());
            }
        }
    } else {
        while let Some(hd) = head.0.first() {
            // SAFETY: `hd` is the first element of the list.
            unsafe { head.0.remove(hd) };
            if let Some(func) = hd.hd_fn.get() {
                func(hd.hd_arg.get());
            }
            if flags & HOOK_FREE != 0 {
                free(
                    NonNull::from(hd).cast::<u8>(),
                    M_DEVBUF,
                    size_of::<HookDesc>(),
                );
            }
        }
    }
}

/// `startuphook_establish(fn, arg)`: a hook `main` runs before it mounts root.
pub fn startuphook_establish(func: fn(*mut c_void), arg: *mut c_void) -> Option<NonNull<HookDesc>> {
    hook_establish(&STARTUPHOOK_LIST, true, func, arg)
}

/// `startuphook_disestablish(vhook)`.
///
/// # Safety
///
/// As [`hook_disestablish`], for a hook of `startuphook_establish`.
pub unsafe fn startuphook_disestablish(vhook: NonNull<HookDesc>) {
    // SAFETY: the caller's guarantee.
    unsafe { hook_disestablish(&STARTUPHOOK_LIST, vhook) }
}

/// `dostartuphooks()`: runs every startup hook once and frees it.
pub fn dostartuphooks() {
    dohooks(&STARTUPHOOK_LIST, HOOK_REMOVE | HOOK_FREE);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `kern_subr.c`: the hash table sizes, `uiomove` in both directions and both
    // segments across several iovecs, and `ureadc`. The host double's `copyin`/`copyout` copy
    // within its one address space, so "user" buffers are ordinary arrays.

    use core::ffi::c_void;
    use std::{assert, assert_eq, vec};

    use super::*;
    use crate::sys::uio::Iovec;

    #[test]
    fn sizes_round_up_to_powers_of_two() {
        assert_eq!(hashsize(1), 1);
        assert_eq!(hashsize(2), 2);
        assert_eq!(hashsize(3), 4);
        assert_eq!(hashsize(64), 64);
        assert_eq!(hashsize(65), 128);
    }

    fn iov(buf: &mut [u8]) -> Iovec {
        Iovec {
            iov_base: buf.as_mut_ptr().cast::<c_void>(),
            iov_len: buf.len(),
        }
    }

    fn uio<'a>(iovs: &'a mut [Iovec], seg: UioSeg, rw: UioRw) -> Uio<'a> {
        let resid = iovs.iter().map(|v| v.iov_len).sum();
        Uio {
            uio_iov: iovs,
            uio_offset: 100,
            uio_resid: resid,
            uio_segflg: seg,
            uio_rw: rw,
            uio_procp: None,
        }
    }

    #[test]
    fn uiomove_reads_across_iovecs_and_skips_empty_ones() {
        let (mut a, mut b, mut c) = ([0u8; 3], [0u8; 0], [0u8; 5]);
        let mut iovs = [iov(&mut a), iov(&mut b), iov(&mut c)];
        let mut u = uio(&mut iovs, UioSeg::UIO_USERSPACE, UioRw::UIO_READ);

        let mut src = *b"abcdef";
        assert_eq!(uiomove(&mut src, &mut u), Ok(()));
        assert_eq!(u.uio_resid, 2);
        assert_eq!(u.uio_offset, 106);
        assert_eq!(u.uio_iovcnt(), 1);
        assert_eq!(u.uio_iov[0].iov_len, 2);

        // n is bounded by uio_resid.
        let mut more = *b"ghijkl";
        assert_eq!(uiomove(&mut more, &mut u), Ok(()));
        assert_eq!(u.uio_resid, 0);
        assert_eq!(uiomove(&mut more, &mut u), Ok(()));
        assert_eq!(&a, b"abc");
        assert_eq!(&c, b"defgh");
    }

    #[test]
    fn uiomove_writes_from_kernel_space() {
        let mut src = *b"hello, world";
        let (h, w) = src.split_at_mut(7);
        let mut iovs = [iov(h), iov(w)];
        let mut u = uio(&mut iovs, UioSeg::UIO_SYSSPACE, UioRw::UIO_WRITE);

        let mut dst = vec![0u8; 12];
        assert_eq!(uiomove(&mut dst[..10], &mut u), Ok(()));
        assert_eq!(&dst[..10], b"hello, wor");
        assert_eq!(u.uio_resid, 2);
        assert_eq!(uiomove(&mut dst[10..], &mut u), Ok(()));
        assert_eq!(&dst, b"hello, world");
        assert_eq!(u.uio_offset, 112);
    }

    #[test]
    fn uiomove_reports_a_fault() {
        let mut iovs = [Iovec {
            iov_base: core::ptr::null_mut(),
            iov_len: 4,
        }];
        let mut u = uio(&mut iovs, UioSeg::UIO_USERSPACE, UioRw::UIO_READ);
        assert_eq!(uiomove(&mut [1, 2, 3, 4], &mut u), Err(Errno::EFAULT));
        assert_eq!(u.uio_resid, 4);
    }

    #[test]
    fn ureadc_fills_one_character_at_a_time() {
        let (mut a, mut b) = ([0u8; 0], [0u8; 2]);
        for seg in [UioSeg::UIO_USERSPACE, UioSeg::UIO_SYSSPACE] {
            let mut iovs = [iov(&mut a), iov(&mut b)];
            let mut u = uio(&mut iovs, seg, UioRw::UIO_READ);
            assert_eq!(ureadc(i32::from(b'x'), &mut u), Ok(()));
            assert_eq!(ureadc(i32::from(b'y'), &mut u), Ok(()));
            assert_eq!(u.uio_resid, 0);
            assert_eq!(u.uio_offset, 102);
            #[cfg(not(feature = "diagnostic"))]
            assert_eq!(ureadc(i32::from(b'z'), &mut u), Err(Errno::EINVAL));
            assert!(b == *b"xy");
            b = [0; 2];
        }
    }
    std::thread_local! {
        /// The arguments the hooks of the test below were run with, in order.
        static RAN: core::cell::RefCell<std::vec::Vec<usize>> =
            const { core::cell::RefCell::new(std::vec::Vec::new()) };
    }

    fn note(arg: *mut c_void) {
        RAN.with(|r| r.borrow_mut().push(arg as usize));
    }

    #[test]
    fn hooks_run_in_order_and_are_removed_and_freed() {
        let _g = crate::kern::subr_pool::tests::setup_real_memory();
        static HEAD: HookDescHead = HookDescHead::new();
        HEAD.0.init();
        let ran = || RAN.with(|r| core::mem::take(&mut *r.borrow_mut()));

        // Tail, tail, head: 3 runs first.
        let h1 = hook_establish(&HEAD, true, note, ptr::without_provenance_mut(1)).expect("a hook");
        hook_establish(&HEAD, true, note, ptr::without_provenance_mut(2)).expect("a hook");
        hook_establish(&HEAD, false, note, ptr::without_provenance_mut(3)).expect("a hook");

        // Without HOOK_REMOVE the hooks stay and run again.
        dohooks(&HEAD, 0);
        assert_eq!(ran(), [3, 1, 2]);
        // One disestablished hook does not run any more.
        // SAFETY: `h1` is a hook of this list, established above and not freed.
        unsafe { hook_disestablish(&HEAD, h1) };
        dohooks(&HEAD, 0);
        assert_eq!(ran(), [3, 2]);

        // With HOOK_REMOVE | HOOK_FREE they run once more and the list is empty.
        dohooks(&HEAD, HOOK_REMOVE | HOOK_FREE);
        assert_eq!(ran(), [3, 2]);
        assert!(HEAD.0.first().is_none());
        dohooks(&HEAD, HOOK_REMOVE | HOOK_FREE);
        assert_eq!(ran(), std::vec::Vec::<usize>::new());
    }
}
/* </TESTS> */
