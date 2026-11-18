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
 * Copyright (c) 1982, 1986, 1990, 1991, 1993
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
 *	@(#)kern_synch.c	8.6 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! Sleep and wakeup: `kern/kern_synch.c`.
//!
//! Upstream: sys/kern/kern_synch.c @ 3ce1f3f79392
//!
//! Status: `ported`. Milestone M5 ports the sleep queues (`sleep_queue_init`), `tsleep`,
//! `tsleep_nsec`, `msleep`, `msleep_nsec`, `sleep_setup`, `sleep_finish`, `wakeup_proc`,
//! `endtsleep`, `unsleep`, `wakeup_n`, `wakeup`, `wakeup_one`, the reference counts
//! (`refcnt_*`) and the condition variables (`cond_*`). `rwsleep[_nsec]` came with
//! `kern_rwlock.c`; `sleep_signal_check` with `kern_sig.c`. M8: `sys_sched_yield`. M11a: the
//! `MULTIPROCESSOR` kernel lock steps of `tsleep_nsec` and `msleep_nsec`. TD1: the
//! `__thrsleep(2)`/`__thrwakeup(2)` system calls (`thrsleep_unlock`, `struct tslpentry`, the
//! shared bucket and the 64 hashed ones, `tslp_init`, `thrsleep_bucket`, `thrsleep`,
//! `tslp_wakeups`). On amd64 and arm64 libc's mutexes, condition variables and `FILE` locks
//! sleep in futex(2); what calls `__thrsleep` there is librthread's `_rthread_dl_lock`, the
//! lock of `dlopen`, `dlclose` and the destructors run at exit, so only dynamically linked
//! threaded programs reach it (in the tree: cargo and rustc in the self-hosting smokes).
//!
//! ## Deviations
//! - `thrsleep` hands `sys___thrsleep`'s copied-in timeout as an argument instead of
//!   pointing `SCARG(uap, tp)` at the kernel copy, and returns `Result<(), Errno>`; the
//!   system calls still put the error in `*retval` and succeed.
//! - The two "take the entry off unless a waker did" blocks of `thrsleep` (after the sleep,
//!   and `leave:`) are one helper, `thrsleep_unwait`, and the end after `sleep_finish` is
//!   `thrsleep_settle`, so the host tests reach the race of a wakeup with a timeout.
//! - `tslp_p` (`struct proc *volatile`) is an atomic: the waker clears it with a release
//!   swap and the sleeper's unlocked reads are acquires, so the waker's last read of the
//!   entry (its next link) is ordered before the sleeper may return.
//! - KTRACE (`ktrabstimespec`) is not configured.
//! - The sleep functions return `Result<(), Errno>` (`EWOULDBLOCK` on timeout, `EINTR`/
//!   `ERESTART` from a signal) instead of an `int`.
//! - The `cold == 2` ddb stack dump is not here (`cold` is a flag, not a counter).
//! - `safepri` and `cold` are `sys/systm.rs` statics (see there).
//! - The `cold || panicstr` path's release and reacquisition of the kernel lock, written
//!   twice in C, is one helper, `kernel_lock_bounce`.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicPtr, Ordering, fence};

use crate::conf::param::TICK_NSEC;
use crate::kassert;
#[cfg(feature = "multiprocessor")]
use crate::kern::kern_lock::{
    __mp_acquire_count, __mp_release_all, _kernel_lock_held, KERNEL_LOCK,
};
use crate::kern::kern_lock::{mtx_enter, mtx_init, mtx_leave};
use crate::kern::kern_rwlock::{rw_assert_anylock, rw_enter, rw_exit, rw_status};
use crate::kern::kern_sched::setrunqueue;
use crate::kern::kern_sig::{cursig, proc_suspend_check, process_stop, process_suspend_signal};
use crate::kern::kern_time::clock_gettime;
use crate::kern::kern_timeout::{timeout_add_nsec, timeout_del};
use crate::kern::sched_bsd::{
    mi_switch, sched_assert_locked, sched_lock, sched_unlock, setrunnable,
};
#[cfg(feature = "diagnostic")]
use crate::kern::subr_prf::{Str, log};
use crate::kern::subr_prf::{panic, panicstr};
use crate::machine::Machine;
use crate::machine::copy::{copyin_obj, copyin32, copyout_obj};
use crate::machine::cpu::{Cpu, curproc};
use crate::machine::intr::{IPL_MPFLOOR, splhigh, splx};
use crate::machine::spinlock::_ATOMIC_LOCK_UNLOCKED;
use crate::queue_adapter;
use crate::sys::errno::Errno;
use crate::sys::mutex::Mutex;
use crate::sys::param::{PCATCH, PNORELOCK, PRIMASK, PWAIT};
#[cfg(feature = "diagnostic")]
use crate::sys::proc::P_CANTSLEEP;
use crate::sys::proc::{
    Cond, P_INSCHED, P_SINTR, P_SUSPSIG, P_TIMEOUT, P_TIMEOUTRAN, P_WEXIT, PS_STOPPING, Proc,
    ProcRunq, Process, SINGLE_SUSPEND, SONPROC, SSLEEP, SSTOP,
};
use crate::sys::queue::{TailqEntry, TailqHead};
use crate::sys::refcnt::Refcnt;
use crate::sys::rwlock::Rwlock;
use crate::sys::signal::sigmask;
use crate::sys::signalvar::Sigctx;
use crate::sys::syscallargs::{SysThrsleepArgs, SysThrwakeupArgs};
#[cfg(feature = "diagnostic")]
use crate::sys::syslog::LOG_WARNING;
use crate::sys::systm::{COLD, INFSLP, MAXTSLP, SAFEPRI, SysArgs, sysargs};
use crate::sys::time::{Timespec, timespec_to_nsec, timespecsub};
use crate::sys::types::Register;

/// `TABLESIZE`: we're only looking at 7 bits of the address; everything is aligned to 4,
/// lots of things are aligned to greater powers of 2. Shift right by 8, i.e. drop the bottom
/// 256 worth.
const TABLESIZE: usize = 128;

/// `LOOKUP(x)`.
fn lookup(x: *const c_void) -> usize {
    ((x as usize) >> 8) & (TABLESIZE - 1)
}

/// `slpque[TABLESIZE]`'s table, made `Sync`: touched under the scheduler lock.
struct Slpque([TailqHead<ProcRunq>; TABLESIZE]);
// SAFETY: see the type's doc.
unsafe impl Sync for Slpque {}

/// `slpque`: the sleep queues.
static SLPQUE: Slpque = Slpque([const { TailqHead::new() }; TABLESIZE]);

/// `TSLP_BUCKET_BITS`.
const TSLP_BUCKET_BITS: u32 = 6;
/// `TSLP_BUCKET_SIZE`.
const TSLP_BUCKET_SIZE: usize = 1 << TSLP_BUCKET_BITS;
/// `TSLP_BUCKET_MASK`.
const TSLP_BUCKET_MASK: i64 = TSLP_BUCKET_SIZE as i64 - 1;

/// `struct tslpentry`: a thread sleeping in `__thrsleep(2)`.
///
/// It lives on the sleeper's stack and is linked into its bucket for the length of the
/// call. `tslp_p` is the C's `struct proc *volatile`: set while the entry is queued, cleared
/// by the waker (under the bucket lock) when it takes the entry off; the sleeper reads it
/// without the lock and may return as soon as it is null, so nothing touches the entry
/// after that store.
pub struct Tslpentry {
    /// \[b\] `tslp_link`: entry on the bucket's list (or a waker's local list).
    tslp_link: TailqEntry<Tslpentry>,
    /// \[I\] `tslp_ps`: the sleeper's process.
    tslp_ps: *const Process,
    /// \[I\] `tslp_ident`: the address slept on, as a `long`.
    tslp_ident: i64,
    /// \[b\] `tslp_p`: the sleeping thread; null once a waker took the entry.
    tslp_p: AtomicPtr<Proc>,
}

impl Tslpentry {
    /// The entry of thread `p` sleeping on `ident`, on no list yet.
    fn new(p: &Proc, ident: i64) -> Self {
        Self {
            tslp_link: TailqEntry::new(),
            tslp_ps: ptr::from_ref(p.process()),
            tslp_ident: ident,
            tslp_p: AtomicPtr::new(ptr::from_ref(p).cast_mut()),
        }
    }
}

queue_adapter!(
    /// `TAILQ_HEAD(tslpqueue, tslpentry)`.
    Tslpqueue: Tslpentry, tslp_link => TailqEntry<Tslpentry>
);

/// `struct tslp_bucket`: one hash chain of `__thrsleep(2)` sleepers, cache-line aligned.
#[repr(align(64))]
struct TslpBucket {
    /// \[b\] `tsb_list`.
    tsb_list: TailqHead<Tslpqueue>,
    /// `tsb_lock`.
    tsb_lock: Mutex,
}

impl TslpBucket {
    /// A bucket before `tslp_init`.
    const fn new() -> Self {
        Self {
            tsb_list: TailqHead::new(),
            tsb_lock: Mutex::new(IPL_MPFLOOR),
        }
    }
}

// SAFETY: `tsb_list` and the entries on it are touched only under `tsb_lock` (`tslp_init`
// runs before any system call); an entry's `tslp_p`, read without the lock, is an atomic.
unsafe impl Sync for TslpBucket {}

/// `tsb_shared`: thrsleep queue shared between processes (ident -1).
static TSB_SHARED: TslpBucket = TslpBucket::new();

/// `tsb_buckets[TSLP_BUCKET_SIZE]`.
static TSB_BUCKETS: [TslpBucket; TSLP_BUCKET_SIZE] =
    [const { TslpBucket::new() }; TSLP_BUCKET_SIZE];

/// `sleep_queue_init`.
pub fn sleep_queue_init() {
    for q in SLPQUE.0.iter() {
        q.init();
    }
}

/// `nowake`: global sleep channel for threads that do not want to receive wakeup(9)
/// broadcasts.
pub static NOWAKE: AtomicI32 = AtomicI32::new(0);

/// `&nowake` as an ident.
pub fn nowake() -> *const c_void {
    ptr::from_ref(&NOWAKE).cast()
}

/// `curproc`, which every sleeper has.
fn sleeper() -> &'static Proc {
    let Some(p) = curproc() else {
        panic(format_args!("sleep: no curproc"));
    };
    p
}

/// `tsleep_nsec`: general sleep call. Suspends the current process until a wakeup is
/// performed on the specified identifier. The process will then be made runnable with the
/// specified priority. Sleeps at most `nsecs` (`INFSLP` means no timeout). If `priority`
/// includes the `PCATCH` flag, signals are checked before and after sleeping, else signals
/// are not checked. Returns `Ok` if awakened, `EWOULDBLOCK` if the timeout expires. If
/// `PCATCH` is set and a signal needs to be delivered, `ERESTART` is returned if the current
/// system call should be restarted if possible, and `EINTR` is returned if the system call
/// should be interrupted by the signal.
pub fn tsleep_nsec<T: ?Sized>(
    ident: *const T,
    priority: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let ident = ident.cast::<c_void>();
    kassert!(priority & !(PRIMASK | PCATCH) == 0);
    kassert!(!ptr::eq(ident, nowake()) || priority & PCATCH != 0 || nsecs != INFSLP);

    #[cfg(feature = "multiprocessor")]
    kassert!(ptr::eq(ident, nowake()) || nsecs != INFSLP || _kernel_lock_held());

    if COLD.load(Ordering::Relaxed) || panicstr() {
        // After a panic, or during autoconfiguration, just give interrupts a chance, then just
        // return; don't run any other procs or panic below, in case this is the idle process
        // and already asleep.
        let s = splhigh();
        splx(SAFEPRI.load(Ordering::Relaxed));
        #[cfg(feature = "multiprocessor")]
        kernel_lock_bounce();
        splx(s);
        return Ok(());
    }

    sleep_setup(ident, priority, wmesg);
    sleep_finish(nsecs, true)
}

/// The `cold || panicstr` path's `MULTIPROCESSOR` step in `tsleep_nsec` and `msleep_nsec`:
/// if this CPU holds the kernel lock, drop every hold and take them back, so another CPU
/// waiting for the lock gets a turn.
#[cfg(feature = "multiprocessor")]
fn kernel_lock_bounce() {
    if _kernel_lock_held() {
        let hold_count = __mp_release_all(&KERNEL_LOCK);
        __mp_acquire_count(&KERNEL_LOCK, hold_count);
    }
}

/// `tsleep`: `tsleep_nsec` with the timeout in ticks.
pub fn tsleep<T: ?Sized>(
    ident: *const T,
    priority: i32,
    wmesg: &'static str,
    timo: i32,
) -> Result<(), Errno> {
    let mut nsecs = INFSLP;

    if timo < 0 {
        panic(format_args!("tsleep: negative timo {timo}"));
    }
    if timo > 0 {
        nsecs = timo as u64 * TICK_NSEC.load(Ordering::Relaxed) as u64;
    }

    tsleep_nsec(ident, priority, wmesg, nsecs)
}

/// `msleep_nsec`: same as `tsleep`, but if we have a mutex provided, then once we've entered
/// the sleep queue we drop the mutex. After sleeping we re-lock.
pub fn msleep_nsec<T: ?Sized>(
    ident: *const T,
    mtx: &Mutex,
    priority: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let ident = ident.cast::<c_void>();
    kassert!(priority & !(PRIMASK | PCATCH | PNORELOCK) == 0);
    kassert!(!ptr::eq(ident, nowake()) || priority & PCATCH != 0 || nsecs != INFSLP);

    if COLD.load(Ordering::Relaxed) || panicstr() {
        // After a panic, or during autoconfiguration, just give interrupts a chance, then just
        // return; don't run any other procs or panic below, in case this is the idle process
        // and already asleep.
        let spl = mtx.mtx_oldipl.get();
        mtx.mtx_oldipl.set(SAFEPRI.load(Ordering::Relaxed));
        mtx_leave(mtx);
        #[cfg(feature = "multiprocessor")]
        kernel_lock_bounce();
        if priority & PNORELOCK == 0 {
            mtx_enter(mtx);
            mtx.mtx_oldipl.set(spl);
        } else {
            splx(spl);
        }
        return Ok(());
    }

    sleep_setup(ident, priority, wmesg);

    mtx_leave(mtx);
    // signal may stop the process, release mutex before that
    let error = sleep_finish(nsecs, true);

    if priority & PNORELOCK == 0 {
        mtx_enter(mtx);
    }

    error
}

/// `msleep`: `msleep_nsec` with the timeout in ticks.
pub fn msleep<T: ?Sized>(
    ident: *const T,
    mtx: &Mutex,
    priority: i32,
    wmesg: &'static str,
    timo: i32,
) -> Result<(), Errno> {
    let mut nsecs = INFSLP;

    if timo < 0 {
        panic(format_args!("msleep: negative timo {timo}"));
    }
    if timo > 0 {
        nsecs = timo as u64 * TICK_NSEC.load(Ordering::Relaxed) as u64;
    }

    msleep_nsec(ident, mtx, priority, wmesg, nsecs)
}

/// `rwsleep_nsec`: same as `tsleep`, but if we have a rwlock provided, then once we've
/// entered the sleep queue we drop the it. After sleeping we re-lock.
pub fn rwsleep_nsec<T: ?Sized>(
    ident: *const T,
    rwl: &Rwlock,
    priority: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let ident = ident.cast::<c_void>();
    kassert!(priority & !(PRIMASK | PCATCH | PNORELOCK) == 0);
    kassert!(!ptr::eq(ident, nowake()) || priority & PCATCH != 0 || nsecs != INFSLP);
    kassert!(!ptr::eq(ident, ptr::from_ref(rwl).cast()));
    rw_assert_anylock(rwl);
    let status = rw_status(rwl);

    sleep_setup(ident, priority, wmesg);

    rw_exit(rwl);
    // signal may stop the process, release rwlock before that
    let error = sleep_finish(nsecs, true);

    if priority & PNORELOCK == 0 {
        let _ = rw_enter(rwl, status);
    }

    error
}

/// `rwsleep`: `rwsleep_nsec` with the timeout in ticks.
pub fn rwsleep<T: ?Sized>(
    ident: *const T,
    rwl: &Rwlock,
    priority: i32,
    wmesg: &'static str,
    timo: i32,
) -> Result<(), Errno> {
    let mut nsecs = INFSLP;

    if timo < 0 {
        panic(format_args!("rwsleep: negative timo {timo}"));
    }
    if timo > 0 {
        nsecs = timo as u64 * TICK_NSEC.load(Ordering::Relaxed) as u64;
    }

    rwsleep_nsec(ident, rwl, priority, wmesg, nsecs)
}

/// `sleep_setup`: puts the current thread on the sleep queue of `ident`.
pub fn sleep_setup(ident: *const c_void, prio: i32, wmesg: &'static str) {
    let p = sleeper();
    let mut prio = prio;

    #[cfg(feature = "diagnostic")]
    {
        if p.p_flag.load(Ordering::Relaxed) & P_CANTSLEEP != 0 {
            panic(format_args!(
                "sleep: {} failed insomnia",
                Str(p.process().comm())
            ));
        }
        if p.p_flag.load(Ordering::Relaxed) & P_SINTR != 0 {
            panic(format_args!("sleep: stale P_SINTR"));
        }
        if ident.is_null() {
            panic(format_args!("sleep: no ident"));
        }
        if p.p_stat.get() != SONPROC {
            panic(format_args!("sleep: not SONPROC but {}", p.p_stat.get()));
        }
    }
    // exiting processes are not allowed to catch signals
    if p.p_flag.load(Ordering::Relaxed) & P_WEXIT != 0 {
        prio &= !PCATCH;
    }

    sched_lock();

    // TRACEPOINT(sched, sleep, NULL): dt(4), not configured.

    p.p_wchan.set(ident);
    p.p_wmesg.set(Some(wmesg));
    p.p_slptime.set(0);
    p.p_slppri.set((prio & PRIMASK) as u8);
    p.p_flag.fetch_or(P_INSCHED, Ordering::Relaxed);
    // SAFETY: `p` is on no run or sleep queue (it is SONPROC), and stays valid: a thread
    // outlives its sleeps.
    unsafe { SLPQUE.0[lookup(ident)].insert_tail(p) };
    if prio & PCATCH != 0 {
        p.p_flag.fetch_or(P_SINTR, Ordering::Relaxed);
    }
    p.p_stat.set(SSLEEP);

    sched_unlock();
}

/// `sleep_finish`: sleeps (or not) after `sleep_setup`; returns the sleep's error.
pub fn sleep_finish(nsecs: u64, do_sleep: bool) -> Result<(), Errno> {
    let p = sleeper();
    let mut do_sleep = do_sleep;
    let mut error: Result<(), Errno> = Ok(());
    let mut error1: Result<(), Errno> = Ok(());

    #[cfg(feature = "diagnostic")]
    if nsecs == 0 {
        log(
            LOG_WARNING,
            format_args!(
                "sleep_finish: {}[{}]: {}: trying to sleep zero nanoseconds\n",
                Str(p.process().comm()),
                p.process().ps_pid.get(),
                p.p_wmesg.get().unwrap_or("")
            ),
        );
    }

    if nsecs != INFSLP {
        kassert!(p.p_flag.load(Ordering::Relaxed) & (P_TIMEOUT | P_TIMEOUTRAN) == 0);
        timeout_add_nsec(&p.p_sleep_to, nsecs);
    }

    let mut catch = p.p_flag.load(Ordering::Relaxed) & P_SINTR != 0;
    if catch {
        error = sleep_signal_check(p, false);
        if error.is_err() {
            catch = false;
            do_sleep = false;
        }
    }

    sched_lock();
    // A few checks need to happen before going to sleep:
    // - If the wakeup happens while going to sleep, p->p_wchan will be NULL. In that case
    //   unwind immediately but still check for possible signals and timeouts.
    // - If the sleep is aborted call unsleep and take us of the sleep queue.
    // - If requested to stop force a switch even if the sleep condition got cleared.
    if p.p_wchan.get().is_null() {
        do_sleep = false;
    }
    if !do_sleep {
        unsleep(p);
    }
    if p.p_stat.get() == SSTOP {
        do_sleep = true;
    }
    p.p_flag.fetch_and(!P_INSCHED, Ordering::Relaxed);

    if do_sleep {
        kassert!(p.p_stat.get() == SSLEEP || p.p_stat.get() == SSTOP);
        p.p_ru.ru_nvcsw.set(p.p_ru.ru_nvcsw.get() + 1);
        mi_switch();
    } else {
        kassert!(p.p_stat.get() == SONPROC || p.p_stat.get() == SSLEEP);
        p.p_stat.set(SONPROC);
        sched_unlock();
    }

    #[cfg(feature = "diagnostic")]
    if p.p_stat.get() != SONPROC {
        panic(format_args!("sleep_finish !SONPROC"));
    }

    if let Some(ci) = p.cpu() {
        Machine::ci_schedstate(ci)
            .spc_curpriority
            .store(p.p_usrpri.get(), Ordering::Relaxed);
    }

    // Even though this belongs to the signal handling part of sleep, we need to clear it
    // before the ktrace.
    p.p_flag.fetch_and(!P_SINTR, Ordering::Relaxed);

    // There are three situations to handle when cancelling the p_sleep_to timeout:
    //
    // 1. The timeout has not fired yet
    // 2. The timeout is running
    // 3. The timeout has run
    //
    // If timeout_del succeeds then the timeout won't run and situation 1 is dealt with.
    //
    // If timeout_del does not remove the timeout, then we're handling 2 or 3, but it won't
    // tell us which one. Instead, the P_TIMEOUTRAN flag is used to figure out when we move
    // from 2 to 3. endtsleep() (the p_sleep_to handler) sets the flag when it's finished
    // running, so we spin waiting for it.
    //
    // We spin instead of sleeping because endtsleep() takes the sched lock to do all it's
    // work. If we wanted to go to sleep to wait for endtsleep to run, we'd also have to take
    // the sched lock, so we'd be spinning against it anyway.
    if nsecs != INFSLP && !timeout_del(&p.p_sleep_to) {
        // Wait for endtsleep timeout to finish running
        let mut flag;
        loop {
            flag = p.p_flag.load(Ordering::Relaxed);
            if flag & P_TIMEOUTRAN != 0 {
                break;
            }
            core::hint::spin_loop(); // CPU_BUSY_CYCLE()
        }
        p.p_flag
            .fetch_and(!(P_TIMEOUT | P_TIMEOUTRAN), Ordering::Relaxed);

        if flag & P_TIMEOUT != 0 {
            error1 = Err(Errno::EWOULDBLOCK);
        }
    }

    // Check if thread was woken up because of a unwind or signal but ignore any pending stop
    // condition.
    if catch {
        error = sleep_signal_check(p, true);
    }

    // Signal errors are higher priority than timeouts.
    if error.is_ok() && error1.is_err() {
        error = error1;
    }

    error
}

/// `sleep_signal_check`: check and handle signals and suspensions around a sleep cycle. The
/// 2nd call in `sleep_finish()` sets `after_sleep`. In this case any pending suspend event
/// came in after the wakeup / unsleep and can therefor be ignored. Once the process hits
/// userret the event will be picked up again.
pub fn sleep_signal_check(p: &Proc, after_sleep: bool) -> Result<(), Errno> {
    let pr = p.process();
    let mut ctx = Sigctx::default();

    if let Err(err) = proc_suspend_check(p, true) {
        if err != Errno::EWOULDBLOCK {
            return Err(err);
        }

        // requested to stop
        if !after_sleep {
            mtx_enter(&pr.ps_mtx);
            process_suspend_signal(pr);

            sched_lock();
            p.p_stat.set(SSTOP);
            sched_unlock();
            mtx_leave(&pr.ps_mtx);
        }
    }

    let sig = cursig(p, &mut ctx, true);
    if sig != 0 {
        if ctx.sig_stop {
            if !after_sleep {
                mtx_enter(&pr.ps_mtx);
                pr.ps_xsig.set(sig);
                // This is for stop signals delivered before sleep_setup() was called. We need
                // to do the full dance here before going to sleep.
                p.p_siglist.fetch_and(!sigmask(sig), Ordering::Relaxed);
                pr.ps_flags.fetch_or(PS_STOPPING, Ordering::Relaxed);
                sched_lock();
                process_stop(pr, P_SUSPSIG, SINGLE_SUSPEND);
                sched_unlock();
                p.p_flag.fetch_or(P_SUSPSIG, Ordering::Relaxed);
                process_suspend_signal(pr);
                sched_lock();
                p.p_stat.set(SSTOP);
                sched_unlock();
                mtx_leave(&pr.ps_mtx);
            }
        } else if ctx.sig_intr && !ctx.sig_ignore {
            return Err(Errno::EINTR);
        } else {
            return Err(Errno::ERESTART);
        }
    }

    Ok(())
}

/// `wakeup_proc`: if process hasn't been awakened (wchan non-zero), undo the sleep. If proc is
/// stopped, just unsleep so it will remain stopped.
pub fn wakeup_proc(p: &Proc) -> bool {
    let mut awakened = false;

    sched_assert_locked();

    if !p.p_wchan.get().is_null() {
        awakened = true;
        #[cfg(feature = "diagnostic")]
        if p.p_stat.get() != SSLEEP && p.p_stat.get() != SSTOP {
            panic(format_args!(
                "thread {} p_stat is {}",
                p.p_tid.get(),
                p.p_stat.get()
            ));
        }
        unsleep(p);
        if p.p_stat.get() == SSLEEP {
            setrunnable(p);
        }
    }

    awakened
}

/// `endtsleep`: this is the timeout handler that wakes up procs that only want to sleep for a
/// period of time rather than forever (until they get a wakeup from somewhere else). It is
/// only scheduled and used by `sleep_finish()`, which coordinates with this handler via the
/// `P_TIMEOUT` and `P_TIMEOUTRAN` flags.
pub fn endtsleep(arg: *mut c_void) {
    // SAFETY: `arg` is the thread that armed its `p_sleep_to`, alive while it sleeps.
    let p = unsafe { &*arg.cast::<Proc>() };

    sched_lock();
    let awakened = wakeup_proc(p);
    sched_unlock();

    let mut flags = P_TIMEOUTRAN;
    if awakened {
        flags |= P_TIMEOUT;
    }

    // Let sleep_finish() proceed.
    p.p_flag.fetch_or(flags, Ordering::Relaxed);
    // Do not alter the proc after this point.
}

/// `unsleep`: remove a process from its wait queue.
pub fn unsleep(p: &Proc) {
    sched_assert_locked();

    if !p.p_wchan.get().is_null() {
        // SAFETY: a thread with a wchan is on that channel's sleep queue, under the
        // scheduler lock.
        unsafe { SLPQUE.0[lookup(p.p_wchan.get())].remove(p) };
        p.p_wchan.set(ptr::null());
        p.p_wmesg.set(None);
        // TRACEPOINT(sched, unsleep, ...): dt(4), not configured.
    }
}

/// `wakeup_n`: make a number of processes sleeping on the specified identifier runnable.
pub fn wakeup_n<T: ?Sized>(ident: *const T, n: i32) {
    let ident = ident.cast::<c_void>();
    let wakeq: TailqHead<ProcRunq> = TailqHead::new();
    let mut n = n;

    sched_lock();
    let qp = &SLPQUE.0[lookup(ident)];
    let mut next = qp.first();
    while let Some(p) = next
        && n != 0
    {
        next = TailqHead::<ProcRunq>::next(p);
        #[cfg(feature = "diagnostic")]
        if p.p_stat.get() != SSLEEP && p.p_stat.get() != SSTOP {
            panic(format_args!(
                "thread {} p_stat is {}",
                p.p_tid.get(),
                p.p_stat.get()
            ));
        }
        kassert!(!p.p_wchan.get().is_null());
        if ptr::eq(p.p_wchan.get(), ident) {
            // SAFETY: `p` is on `qp`, under the scheduler lock; it moves to `wakeq`, a local
            // queue that is emptied below before it goes out of scope.
            unsafe {
                qp.remove(p);
                p.p_wchan.set(ptr::null());
                p.p_wmesg.set(None);
                wakeq.insert_tail(p);
            }
            n -= 1;
        }
    }
    while let Some(p) = wakeq.first() {
        // SAFETY: `p` is on `wakeq`.
        unsafe { wakeq.remove(p) };
        // TRACEPOINT(sched, unsleep, ...): dt(4), not configured.
        if p.p_stat.get() == SSLEEP {
            setrunnable(p);
        }
    }
    sched_unlock();
}

/// `wakeup`: make all processes sleeping on the specified identifier runnable.
pub fn wakeup<T: ?Sized>(chan: *const T) {
    wakeup_n(chan, -1);
}

/// `sched_yield(2)`: give the CPU up; a thread of a multi-threaded process drops to the
/// lowest priority among its siblings so that they can make some progress.
pub fn sys_sched_yield(p: &Proc, _v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    // If one of the threads of a multi-threaded process called sched_yield(2), drop its
    // priority to ensure its siblings can make some progress.
    let pr = p.process();
    mtx_enter(&pr.ps_mtx);
    let mut newprio = p.p_usrpri.get();
    for q in pr.ps_threads.iter() {
        newprio = newprio.max(q.p_runpri.get());
    }
    mtx_leave(&pr.ps_mtx);

    sched_lock();
    setrunqueue(p.cpu(), p, newprio);
    p.p_ru.ru_nvcsw.set(p.p_ru.ru_nvcsw.get() + 1);
    mi_switch();

    Ok(())
}

/// `wakeup_one(c)`: `wakeup_n((c), 1)` (`<sys/systm.h>`).
pub fn wakeup_one<T: ?Sized>(chan: *const T) {
    wakeup_n(chan, 1);
}

/// `thrsleep_unlock`: frees the caller's user-space spinlock, if it named one, by storing
/// `_ATOMIC_LOCK_UNLOCKED` through it.
fn thrsleep_unlock(atomiclock: usize) -> Result<(), Errno> {
    if atomiclock == 0 {
        return Ok(());
    }

    copyout_obj(&_ATOMIC_LOCK_UNLOCKED, atomiclock)
}

/// `tslp_init`: the shared bucket and the hashed ones, empty, with their locks.
pub fn tslp_init() {
    TSB_SHARED.tsb_list.init();
    mtx_init(&TSB_SHARED.tsb_lock, IPL_MPFLOOR);

    for tsb in &TSB_BUCKETS {
        tsb.tsb_list.init();
        mtx_init(&tsb.tsb_lock, IPL_MPFLOOR);
    }
}

/// `thrsleep_bucket`: the bucket of `ident` (any but 0 and -1). The low three bits of an
/// address say little; the next twelve are folded into six.
fn thrsleep_bucket(ident: i64) -> &'static TslpBucket {
    let mut ident = ident >> 3;
    ident ^= ident >> TSLP_BUCKET_BITS;
    ident &= TSLP_BUCKET_MASK;

    &TSB_BUCKETS[ident as usize]
}

/// Takes `entry` off `tsb` unless a waker already did; whether it was still queued. The
/// C writes this check twice in `thrsleep`, after the sleep and on the `leave:` path.
fn thrsleep_unwait(tsb: &TslpBucket, entry: &Tslpentry) -> bool {
    mtx_enter(&tsb.tsb_lock);
    let queued = !entry.tslp_p.load(Ordering::Acquire).is_null();
    if queued {
        // SAFETY: `entry` is on `tsb`'s list (its `tslp_p` is set, and only the holder of
        // the lock clears it when taking it off), and the lock is held.
        unsafe { tsb.tsb_list.remove(entry) };
    }
    mtx_leave(&tsb.tsb_lock);

    queued
}

/// The end of `thrsleep` after `sleep_finish`: a sleeper still queued (timed out,
/// signalled, or never asleep) takes itself off and keeps the sleep's error; one a waker
/// already took was woken, which wins over a timeout or a signal that raced it. An
/// interrupted sleep that would restart is `ECANCELED`.
fn thrsleep_settle(
    tsb: &TslpBucket,
    entry: &Tslpentry,
    error: Result<(), Errno>,
) -> Result<(), Errno> {
    let mut error = error;

    if error.is_err() || !entry.tslp_p.load(Ordering::Acquire).is_null() {
        if !thrsleep_unwait(tsb, entry) {
            error = Ok(());
        }

        if error == Err(Errno::ERESTART) {
            error = Err(Errno::ECANCELED);
        }
    }

    error
}

/// `thrsleep`: `__thrsleep(2)` once `sys___thrsleep` has copied in and checked the
/// timeout, which is `tsp` (an absolute time on `clock_id`).
fn thrsleep(p: &Proc, uap: &SysThrsleepArgs, tsp: Option<Timespec>) -> Result<(), Errno> {
    let ident = uap.ident.get() as usize as i64;
    let lock = uap.lock.get() as usize;
    let abortp = uap.abort.get() as usize;
    let clock_id = uap.clock_id.get();
    let mut nsecs = INFSLP;

    if ident == 0 {
        return Err(Errno::EINVAL);
    }
    if let Some(tsp) = tsp {
        let now = clock_gettime(p, clock_id)?;
        // KTRACE (ktrabstimespec): not configured.

        if tsp <= now {
            // already passed: still do the unlock
            thrsleep_unlock(lock)?;
            return Err(Errno::EWOULDBLOCK);
        }

        let tsp = timespecsub(&tsp, &now);
        // MAX(1, MIN(TIMESPEC_TO_NSEC(tsp), MAXTSLP))
        nsecs = timespec_to_nsec(&tsp).clamp(1, MAXTSLP);
    }

    let tsb = if ident == -1 {
        &TSB_SHARED
    } else {
        thrsleep_bucket(ident)
    };

    // Interlock with wakeup.
    let entry = Tslpentry::new(p, ident);

    mtx_enter(&tsb.tsb_lock);
    // SAFETY: `entry` is on no list; it stays in this frame until it is off the list again
    // (every return below goes through `leave` or `thrsleep_settle`, or a waker took it).
    unsafe { tsb.tsb_list.insert_tail(&entry) };
    mtx_leave(&tsb.tsb_lock);

    // leave: an early exit takes the entry off unless a waker already did.
    let leave = |error: Errno| -> Result<(), Errno> {
        if !entry.tslp_p.load(Ordering::Acquire).is_null() {
            thrsleep_unwait(tsb, &entry);
        }
        Err(error)
    };

    if let Err(error) = thrsleep_unlock(lock) {
        return leave(error);
    }

    if abortp != 0 {
        match copyin32(abortp) {
            Err(error) => return leave(error),
            Ok(abort) if abort != 0 => return leave(Errno::EINTR),
            Ok(_) => {}
        }
    }

    sleep_setup(ptr::from_ref(&entry).cast(), PWAIT | PCATCH, "thrsleep");
    let error = sleep_finish(nsecs, !entry.tslp_p.load(Ordering::Acquire).is_null());
    thrsleep_settle(tsb, &entry, error)
}

/// `__thrsleep(2)`: sleeps on `ident` until `__thrwakeup(2)`, the absolute timeout `tp`, or
/// a signal. The error is the return value (`*retval`); the system call itself succeeds.
#[allow(non_snake_case)] // the C name: sys___thrsleep
pub fn sys___thrsleep(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysThrsleepArgs = sysargs(v);
    let mut ts = None;

    let tp = uap.tp.get() as usize;
    if tp != 0 {
        let t: Timespec = match copyin_obj(tp) {
            Ok(t) => t,
            Err(error) => {
                retval[0] = error.as_i32() as Register;
                return Ok(());
            }
        };
        if !t.is_valid() {
            retval[0] = Errno::EINVAL.as_i32() as Register;
            return Ok(());
        }
        ts = Some(t);
    }

    retval[0] = match thrsleep(p, uap, ts) {
        Ok(()) => 0,
        Err(error) => error.as_i32() as Register,
    };
    Ok(())
}

/// `tslp_wakeups`: wakes every sleeper on `tslpq`, which the caller took off its bucket (or
/// is the shared bucket, emptied after). Clearing `tslp_p` hands the entry back to its
/// sleeper, which may return at once, so the next link is read first.
fn tslp_wakeups(tslpq: &TailqHead<Tslpqueue>) {
    sched_lock();
    let mut entry = tslpq.first();
    while let Some(cur) = entry {
        entry = TailqHead::<Tslpqueue>::next(cur);
        let p = cur.tslp_p.swap(ptr::null_mut(), Ordering::Release);
        // `cur` may be gone now.
        // SAFETY: `tslp_p` named the sleeping thread, alive while it sleeps; it is never
        // null on a queued entry (only the lock holder clears it, taking the entry off).
        if let Some(p) = unsafe { p.as_ref() } {
            wakeup_proc(p);
        }
    }
    sched_unlock();
}

/// `__thrwakeup(2)`: wakes at most `n` (all when `n <= 0`) of the caller's process's
/// threads sleeping on `ident`; ident -1 wakes every sleeper on it, of any process. The
/// error (`ESRCH` when none matched) is the return value; the system call succeeds.
#[allow(non_snake_case)] // the C name: sys___thrwakeup
pub fn sys___thrwakeup(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysThrwakeupArgs = sysargs(v);
    let ident = uap.ident.get() as usize as i64;
    let n = uap.n.get();
    let mut found: i32 = 0;
    let wq: TailqHead<Tslpqueue> = TailqHead::new();
    wq.init();

    if ident == 0 {
        retval[0] = Errno::EINVAL.as_i32() as Register;
        return Ok(());
    }

    if ident == -1 {
        // Wake up all waiters with ident -1. This is needed because ident -1 can be shared
        // by multiple userspace lock state machines concurrently. The implementation has no
        // way to direct the wakeup to a particular state machine.
        mtx_enter(&TSB_SHARED.tsb_lock);
        tslp_wakeups(&TSB_SHARED.tsb_list);
        TSB_SHARED.tsb_list.init();
        mtx_leave(&TSB_SHARED.tsb_lock);

        retval[0] = 0;
        return Ok(());
    }

    let tsb = thrsleep_bucket(ident);

    mtx_enter(&tsb.tsb_lock);
    let mut entry = tsb.tsb_list.first();
    while let Some(cur) = entry {
        entry = TailqHead::<Tslpqueue>::next(cur);
        if cur.tslp_ident == ident && ptr::eq(cur.tslp_ps, p.process()) {
            // SAFETY: `cur` is on `tsb`'s list, whose lock is held; `wq` is local, and
            // `tslp_wakeups` hands every entry on it back to its sleeper before the lock
            // is released, after which `wq` is not read again.
            unsafe {
                tsb.tsb_list.remove(cur);
                wq.insert_tail(cur);
            }

            found += 1;
            if found == n {
                break;
            }
        }
    }

    if found != 0 {
        tslp_wakeups(&wq);
    }
    mtx_leave(&tsb.tsb_lock);

    retval[0] = if found != 0 {
        0
    } else {
        Errno::ESRCH.as_i32() as Register
    };
    Ok(())
}

/// `refcnt_init`.
pub fn refcnt_init(r: &Refcnt) {
    refcnt_init_trace(r, 0);
}

/// `refcnt_init_trace`.
pub fn refcnt_init_trace(r: &Refcnt, trace: i32) {
    r.r_traceidx.set(trace);
    r.r_refs.store(1, Ordering::Relaxed);
    // TRACEINDEX(refcnt, ...): dt(4), not configured.
}

/// `refcnt_take`.
pub fn refcnt_take(r: &Refcnt) {
    let refs = r.r_refs.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    kassert!(refs > 1);
}

/// `refcnt_rele`: drops a reference; `true` when it was the last.
pub fn refcnt_rele(r: &Refcnt) -> bool {
    fence(Ordering::Release); // membar_exit_before_atomic()
    let refs = r.r_refs.fetch_sub(1, Ordering::Relaxed).wrapping_sub(1);
    kassert!(refs != u32::MAX);
    if refs == 0 {
        fence(Ordering::Acquire); // membar_enter_after_atomic()
        return true;
    }
    false
}

/// `refcnt_rele_wake`.
pub fn refcnt_rele_wake(r: &Refcnt) {
    if refcnt_rele(r) {
        wakeup_one(ptr::from_ref(r));
    }
}

/// `refcnt_finalize`: drops the caller's reference and sleeps until every other one is gone.
pub fn refcnt_finalize(r: &Refcnt, wmesg: &'static str) {
    fence(Ordering::Release); // membar_exit_before_atomic()
    let mut refs = r.r_refs.fetch_sub(1, Ordering::Relaxed).wrapping_sub(1);
    kassert!(refs != u32::MAX);
    while refs != 0 {
        sleep_setup(ptr::from_ref(r).cast(), PWAIT, wmesg);
        refs = r.r_refs.load(Ordering::Relaxed);
        let _ = sleep_finish(INFSLP, refs != 0);
    }
    // Order subsequent loads and stores after refs == 0 load.
    fence(Ordering::SeqCst); // membar_sync()
}

/// `refcnt_read`.
pub fn refcnt_read(r: &Refcnt) -> u32 {
    r.r_refs.load(Ordering::Relaxed)
}

/// `refcnt_shared(_r)`.
pub fn refcnt_shared(r: &Refcnt) -> bool {
    refcnt_read(r) > 1
}

/// `cond_init`.
pub fn cond_init(c: &Cond) {
    c.c_wait.store(1, Ordering::Relaxed);
}

/// `cond_signal_handler`: the timeout/task handler that signals a `cond`.
pub fn cond_signal_handler(arg: *mut c_void) {
    // SAFETY: `arg` is the `cond` its waiter armed the handler with, alive while it waits.
    let c = unsafe { &*arg.cast::<Cond>() };

    c.c_wait.store(0, Ordering::Relaxed);

    wakeup_one(ptr::from_ref(c));
}

/// `cond_wait`: sleeps until the cond is signalled.
pub fn cond_wait(c: &Cond, wmesg: &'static str) {
    let mut wait = c.c_wait.load(Ordering::Relaxed);
    while wait != 0 {
        sleep_setup(ptr::from_ref(c).cast(), PWAIT, wmesg);
        wait = c.c_wait.load(Ordering::Relaxed);
        let _ = sleep_finish(INFSLP, wait != 0);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `__thrsleep(2)` and `__thrwakeup(2)`. The host double cannot switch
    // threads (`cpu_switchto` panics), so nothing here really sleeps: the system calls are
    // driven through every path that returns before `sleep_finish` (EINVAL, a timeout already
    // passed, the abort word, a fault), the wakeups run against entries queued as `thrsleep`
    // queues them (their threads are not asleep, so `wakeup_proc` finds no wait channel and
    // only the bucket bookkeeping shows), and the end of a sleep is `thrsleep_settle` with the
    // error `sleep_finish` would have returned.

    use std::boxed::Box;
    use std::sync::{Mutex as StdMutex, MutexGuard};

    use super::*;
    use crate::machine::spinlock::_ATOMIC_LOCK_LOCKED;
    use crate::sys::_time::CLOCK_MONOTONIC;

    /// The buckets are global: one test at a time, each on fresh ones.
    static LOCK: StdMutex<()> = StdMutex::new(());

    fn setup() -> MutexGuard<'static, ()> {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tslp_init();
        guard
    }

    /// A thread of a process of its own.
    fn thread() -> &'static Proc {
        let pr: &'static Process = Box::leak(Box::new(Process::new()));
        let p: &'static Proc = Box::leak(Box::new(Proc::new()));
        p.p_p.set(pr);
        p
    }

    /// A second thread of `p`'s process.
    fn sibling(p: &Proc) -> &'static Proc {
        let q: &'static Proc = Box::leak(Box::new(Proc::new()));
        q.p_p.set(p.process());
        q
    }

    fn reg<T>(ptr: *const T) -> Register {
        ptr as usize as Register
    }

    /// `__thrsleep(ident, CLOCK_MONOTONIC, tp, lock, abort)`: the value it returns.
    fn thrsleep_call(
        p: &Proc,
        ident: Register,
        tp: Option<&Timespec>,
        lock: Option<&mut i32>,
        abort: Register,
    ) -> Register {
        let v: SysArgs = [
            ident,
            CLOCK_MONOTONIC as Register,
            tp.map_or(0, |t| reg(t)),
            lock.map_or(0, |l| reg(ptr::from_mut(l))),
            abort,
            0,
        ];
        let mut retval = [-1; 2];
        assert_eq!(sys___thrsleep(p, &v, &mut retval), Ok(()));
        retval[0]
    }

    /// `__thrwakeup(ident, n)`: the value it returns.
    fn thrwakeup_call(p: &Proc, ident: Register, n: i32) -> Register {
        let v: SysArgs = [ident, n as Register, 0, 0, 0, 0];
        let mut retval = [-1; 2];
        assert_eq!(sys___thrwakeup(p, &v, &mut retval), Ok(()));
        retval[0]
    }

    fn errno(e: Errno) -> Register {
        e.as_i32() as Register
    }

    /// Queues `entry` on its bucket as `thrsleep` does before unlocking.
    fn enqueue(entry: &Tslpentry) -> &'static TslpBucket {
        let tsb = if entry.tslp_ident == -1 {
            &TSB_SHARED
        } else {
            thrsleep_bucket(entry.tslp_ident)
        };
        mtx_enter(&tsb.tsb_lock);
        // SAFETY: a test entry on no list, taken off (by a wakeup or `thrsleep_unwait`)
        // before the test returns.
        unsafe { tsb.tsb_list.insert_tail(entry) };
        mtx_leave(&tsb.tsb_lock);
        tsb
    }

    fn queued(entry: &Tslpentry) -> bool {
        !entry.tslp_p.load(Ordering::Acquire).is_null()
    }

    fn bucket_len(tsb: &TslpBucket) -> usize {
        let mut n = 0;
        let mut e = tsb.tsb_list.first();
        while let Some(cur) = e {
            n += 1;
            e = TailqHead::<Tslpqueue>::next(cur);
        }
        n
    }

    #[test]
    fn bucket_hash_is_the_c_fold() {
        // (ident >> 3) ^ ((ident >> 3) >> 6), masked with 63
        for ident in [
            8i64,
            0x1000,
            0x7f00_1234_5678,
            -8,
            -0x1000,
            i64::MAX,
            i64::MIN,
        ] {
            let h = ident >> 3;
            let want = ((h ^ (h >> 6)) & 63) as usize;
            assert!(
                ptr::eq(thrsleep_bucket(ident), &TSB_BUCKETS[want]),
                "{ident:#x}"
            );
        }
        assert!(ptr::eq(thrsleep_bucket(0x1000), &TSB_BUCKETS[8]));
        assert!(ptr::eq(thrsleep_bucket(-8), &TSB_BUCKETS[0]));
    }

    #[test]
    fn ident_zero_is_einval() {
        let _g = setup();
        let p = thread();
        let mut lock = _ATOMIC_LOCK_LOCKED;
        assert_eq!(
            thrsleep_call(p, 0, None, Some(&mut lock), 0),
            errno(Errno::EINVAL)
        );
        // nothing was unlocked
        assert_eq!(lock, _ATOMIC_LOCK_LOCKED);
        assert_eq!(thrwakeup_call(p, 0, 1), errno(Errno::EINVAL));
    }

    #[test]
    fn invalid_timespec_is_einval() {
        let _g = setup();
        let p = thread();
        let mut lock = _ATOMIC_LOCK_LOCKED;
        let ts = Timespec::new(0, 1_000_000_000);
        assert_eq!(
            thrsleep_call(p, 0x1000, Some(&ts), Some(&mut lock), 0),
            errno(Errno::EINVAL)
        );
        assert_eq!(lock, _ATOMIC_LOCK_LOCKED);
        let ts = Timespec::new(0, -1);
        assert_eq!(
            thrsleep_call(p, 0x1000, Some(&ts), None, 0),
            errno(Errno::EINVAL)
        );
    }

    #[test]
    fn past_timeout_unlocks_and_would_block() {
        let _g = setup();
        let p = thread();
        let mut lock = _ATOMIC_LOCK_LOCKED;
        // the epoch of the monotonic clock has always passed
        let ts = Timespec::new(0, 0);
        assert_eq!(
            thrsleep_call(p, 0x1000, Some(&ts), Some(&mut lock), 0),
            errno(Errno::EWOULDBLOCK)
        );
        assert_eq!(lock, _ATOMIC_LOCK_UNLOCKED);
        // never queued
        assert_eq!(bucket_len(thrsleep_bucket(0x1000)), 0);
        // and without a lock
        assert_eq!(
            thrsleep_call(p, -1, Some(&ts), None, 0),
            errno(Errno::EWOULDBLOCK)
        );
        assert_eq!(bucket_len(&TSB_SHARED), 0);
    }

    #[test]
    fn abort_word_is_eintr_after_the_unlock() {
        let _g = setup();
        let p = thread();
        let mut lock = _ATOMIC_LOCK_LOCKED;
        let abort: u32 = 1;
        assert_eq!(
            thrsleep_call(p, 0x2000, None, Some(&mut lock), reg(&abort)),
            errno(Errno::EINTR)
        );
        // the lock was released before the abort word was read, and the entry is gone
        assert_eq!(lock, _ATOMIC_LOCK_UNLOCKED);
        assert_eq!(bucket_len(thrsleep_bucket(0x2000)), 0);
        assert_eq!(thrwakeup_call(p, 0x2000, 1), errno(Errno::ESRCH));
    }

    #[test]
    fn abort_fault_leaves_the_queue() {
        let _g = setup();
        let p = thread();
        // a misaligned word is a fault for the host's copyin32
        let words = [0u32; 2];
        let bad = reg(words.as_ptr()) + 1;
        assert_eq!(thrsleep_call(p, -1, None, None, bad), errno(Errno::EFAULT));
        assert_eq!(bucket_len(&TSB_SHARED), 0);
    }

    #[test]
    fn wakeup_without_sleepers_is_esrch() {
        let _g = setup();
        let p = thread();
        assert_eq!(thrwakeup_call(p, 0x3000, 1), errno(Errno::ESRCH));
        assert_eq!(thrwakeup_call(p, 0x3000, 0), errno(Errno::ESRCH));
    }

    #[test]
    fn broadcast_wakes_every_process() {
        let _g = setup();
        let p = thread();
        // none waiting: still 0
        assert_eq!(thrwakeup_call(p, -1, 1), 0);

        let q = thread();
        let a = Tslpentry::new(p, -1);
        let b = Tslpentry::new(q, -1);
        let c = Tslpentry::new(sibling(p), -1);
        enqueue(&a);
        enqueue(&b);
        enqueue(&c);
        assert_eq!(bucket_len(&TSB_SHARED), 3);

        // n is ignored for -1, and so is the process
        assert_eq!(thrwakeup_call(p, -1, 1), 0);
        assert!(!queued(&a) && !queued(&b) && !queued(&c));
        assert_eq!(bucket_len(&TSB_SHARED), 0);
    }

    #[test]
    fn wakeup_n_matches_ident_and_process() {
        let _g = setup();
        let p = thread();
        let other = thread();
        let ident: i64 = 0x4000;
        // same bucket, other ident: bits 15 and up are not hashed
        let alias: i64 = ident + (1 << 15);
        assert!(ptr::eq(thrsleep_bucket(ident), thrsleep_bucket(alias)));

        let s1 = Tslpentry::new(p, ident);
        let s2 = Tslpentry::new(sibling(p), ident);
        let s3 = Tslpentry::new(sibling(p), ident);
        let foreign = Tslpentry::new(other, ident);
        let aliased = Tslpentry::new(p, alias);
        let tsb = enqueue(&s1);
        enqueue(&foreign);
        enqueue(&s2);
        enqueue(&aliased);
        enqueue(&s3);
        assert_eq!(bucket_len(tsb), 5);

        // at most two, in queue order
        assert_eq!(thrwakeup_call(p, ident as Register, 2), 0);
        assert!(!queued(&s1) && !queued(&s2) && queued(&s3));
        assert_eq!(bucket_len(tsb), 3);

        // n <= 0 wakes every match left: s3 only
        assert_eq!(thrwakeup_call(p, ident as Register, 0), 0);
        assert!(!queued(&s3));
        assert!(queued(&foreign) && queued(&aliased));
        assert_eq!(bucket_len(tsb), 2);

        // nothing of p's left on ident
        assert_eq!(
            thrwakeup_call(p, ident as Register, -1),
            errno(Errno::ESRCH)
        );

        // the other process wakes its own, and the alias its own
        assert_eq!(thrwakeup_call(other, ident as Register, 5), 0);
        assert!(!queued(&foreign));
        assert_eq!(thrwakeup_call(p, alias as Register, 1), 0);
        assert!(!queued(&aliased));
        assert_eq!(bucket_len(tsb), 0);
    }

    #[test]
    fn wakeup_wins_over_a_racing_timeout() {
        let _g = setup();
        let p = thread();
        let ident: i64 = 0x5000;

        // the waker took the entry before the sleeper looked: the timeout is forgotten
        let woken = Tslpentry::new(p, ident);
        let tsb = enqueue(&woken);
        assert_eq!(thrwakeup_call(p, ident as Register, 1), 0);
        assert_eq!(
            thrsleep_settle(tsb, &woken, Err(Errno::EWOULDBLOCK)),
            Ok(())
        );
        // and so is a signal
        let woken = Tslpentry::new(p, ident);
        enqueue(&woken);
        assert_eq!(thrwakeup_call(p, ident as Register, 1), 0);
        assert_eq!(thrsleep_settle(tsb, &woken, Err(Errno::EINTR)), Ok(()));

        // still queued when the timeout ran: the sleeper takes itself off and times out
        let late = Tslpentry::new(p, ident);
        enqueue(&late);
        assert_eq!(
            thrsleep_settle(tsb, &late, Err(Errno::EWOULDBLOCK)),
            Err(Errno::EWOULDBLOCK)
        );
        // off the bucket (`tslp_p` stays set: only a waker clears it, as in C)
        assert!(queued(&late));
        assert_eq!(bucket_len(tsb), 0);

        // a restartable signal is ECANCELED
        let sig = Tslpentry::new(p, ident);
        enqueue(&sig);
        assert_eq!(
            thrsleep_settle(tsb, &sig, Err(Errno::ERESTART)),
            Err(Errno::ECANCELED)
        );
        assert_eq!(bucket_len(tsb), 0);

        // a sleep that ended with no error while still queued (a stop, a spurious wakeup)
        // returns 0 off the queue
        let spurious = Tslpentry::new(p, ident);
        enqueue(&spurious);
        assert_eq!(thrsleep_settle(tsb, &spurious, Ok(())), Ok(()));
        assert_eq!(bucket_len(tsb), 0);
        assert_eq!(thrwakeup_call(p, ident as Register, 1), errno(Errno::ESRCH));
    }
}
/* </TESTS> */
