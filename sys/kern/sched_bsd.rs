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
//! The 4.4BSD scheduler: `kern/sched_bsd.c`.
//!
//! Upstream: sys/kern/sched_bsd.c @ 3ce1f3f79392
//!
//! Status: `ported`. Milestone M5 (part a) ports `roundrobin_period`, `sched_lock` with
//! `SCHED_LOCK_INIT` and `roundrobin`, the clock interrupt that `clockintr_cpu_init`
//! schedules; part b2 adds the `SCHED_LOCK`/`SCHED_UNLOCK`/`SCHED_ASSERT_*` macros of
//! `<sys/sched.h>`, the load average (`cexp`, `averunnable`, `update_loadavg`), `ccpu`,
//! `schedcpu`, `decay_aftersleep`, `yield`, `preempt`, `mi_switch`, `setrunnable`,
//! `setpriority`, `schedclock` and `scheduler_start`; TD1 the CPU throttling (`perflevel`,
//! the `perfpolicy_*` knobs, `setperf_auto`, `sysctl_hwsetperf`, `sysctl_hwperfpolicy`),
//! and the file is whole. M11a: `mi_switch` calls
//! `smr_idle` and, with `MULTIPROCESSOR`, drops every hold of the kernel lock around the
//! switch (`__mp_release_all`/`__mp_acquire_count`).
//!
//! ## Deviations
//! - `setperf_auto` keeps its per-CPU tick counts in two static arrays of `MAXCPUS` words
//!   where the C allocates them with `mallocarray(ncpusfound, ...)` at its first run (and
//!   gives up for good when that fails); `SMALL_KERNEL` is not configured.
//! - `averunnable.ldavg` are atomics (`Averunnable`, written by the softclock thread, read
//!   racily by `schedcpu` and sysctl as in C); `averunnable()` returns a `Loadavg` snapshot.
//! - `cexp` and `ccpu` are compile-time constants, like the C's constant-folded doubles; no
//!   floating point reaches the kernel.
//! - `yield` is a Rust keyword: the function is the raw identifier `r#yield`.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};

use libkern::StaticCell;

use crate::kassert;
use crate::kern::kern_clock::{STATHZ, hardclock_period};
use crate::kern::kern_clockintr::{clockintr_advance, clockintr_cancel, clockrequest_advance};
#[cfg(feature = "multiprocessor")]
use crate::kern::kern_lock::{
    __mp_acquire_count, __mp_release_all, _kernel_lock_held, KERNEL_LOCK,
};
use crate::kern::kern_lock::{mtx_enter, mtx_init, mtx_leave, pc_cons_enter, pc_cons_leave};
use crate::kern::kern_proc::ALLPROC;
use crate::kern::kern_resource::tuagg_add_runtime;
use crate::kern::kern_sched::{
    SCHED_ALL_CPUS, SCHED_IDLE_CPUS, cpu_is_online, cpuset_cardinality, cpuset_complement,
    remrunqueue, sched_chooseproc, setrunqueue,
};
use crate::kern::kern_smr::smr_idle;
use crate::kern::kern_sysctl::{
    hw_power, sysctl_int_bounded, sysctl_rdint, sysctl_rdstring, sysctl_string,
};
use crate::kern::kern_tc::nanouptime;
use crate::kern::kern_timeout::{timeout_add_msec, timeout_add_sec};
use crate::kern::subr_prf::panic;
use crate::kern::subr_prof::profclock_period;
use crate::kern::subr_xxx::assertwaitok;
use crate::machine::Machine;
use crate::machine::cpu::{Cpu, MAXCPUS, cpu_info_foreach, curcpu, curproc, need_resched};
use crate::machine::intr::{IPL_NONE, IPL_SCHED};
use crate::sys::clockintr::Clockrequest;
use crate::sys::errno::Errno;
use crate::sys::mutex::{Mutex, mutex_assert_locked, mutex_assert_unlocked};
use crate::sys::param::{FSCALE, FSHIFT, MAXPRI, NZERO, PUSER};
use crate::sys::proc::{P_INSCHED, PS_ITIMER, PS_PROFIL, Proc, SONPROC, SRUN, SSLEEP, SSTOP};
use crate::sys::resource::Loadavg;
use crate::sys::sched::{
    CP_IDLE, NICE_WEIGHT, SCHED_PPQ, SPCF_ITIMER, SPCF_PROFCLOCK, SPCF_SEENRR, SPCF_SHOULDYIELD,
    SPCF_SWITCHCLEAR, estcpulim,
};
use crate::sys::timeout::Timeout;
use crate::sys::types::Fixpt;
use crate::uvm::uvm_init::UVMEXP;

/// \[I\] `roundrobin_period`: roundrobin period (ns).
pub static ROUNDROBIN_PERIOD: AtomicU64 = AtomicU64::new(0);

/// `sched_lock`: initialised by `SCHED_LOCK_INIT()` in `main`.
pub static SCHED_LOCK: Mutex = Mutex::new(IPL_NONE);

/// `cpu_setperf`: the machine's CPU performance setter, `None` until a driver installs one
/// (acpicpu(4)'s `acpicpu_setperf`, M16e). Written while cold by the attaching driver;
/// `hw.setperf`, `hw.perfpolicy` and `setperf_auto` call it.
pub static CPU_SETPERF: StaticCell<Option<fn(i32)>> = StaticCell::new(None);

/// `cexp[3]`: constants for averages over 1, 5, and 15 minutes when sampling at 5 second
/// intervals.
static CEXP: [Fixpt; 3] = [
    (0.920_044_414_629_323_2 * FSCALE as f64) as Fixpt, /* exp(-1/12) */
    (0.983_471_453_821_617_4 * FSCALE as f64) as Fixpt, /* exp(-1/60) */
    (0.994_459_848_004_896_7 * FSCALE as f64) as Fixpt, /* exp(-1/180) */
];

/// `struct loadavg averunnable` with atomic averages (see the module's deviations).
pub struct Averunnable {
    /// `ldavg`: the 1, 5 and 15 minute averages, fixed point.
    pub ldavg: [AtomicU32; 3],
    /// `fscale`: the scale of `ldavg`.
    pub fscale: i64,
}

/// `averunnable`: the load average.
pub static AVERUNNABLE: Averunnable = Averunnable {
    ldavg: [const { AtomicU32::new(0) }; 3],
    fscale: FSCALE as i64,
};

/// `averunnable` as the C struct, for sysctl and the like.
pub fn averunnable() -> Loadavg {
    Loadavg {
        ldavg: [
            AVERUNNABLE.ldavg[0].load(Ordering::Relaxed),
            AVERUNNABLE.ldavg[1].load(Ordering::Relaxed),
            AVERUNNABLE.ldavg[2].load(Ordering::Relaxed),
        ],
        fscale: AVERUNNABLE.fscale,
    }
}

/// `ccpu`: decay 95% of `p_pctcpu` in 60 seconds; see `CCPU_SHIFT` before changing.
/// exp(-1/20): the C writes `0.95122942450071400909`, more digits than a double keeps.
pub const CCPU: Fixpt = (0.951_229_424_500_714 * FSCALE as f64) as Fixpt;

/// `CCPU_SHIFT`: if `ccpu` is not equal to `exp(-1/20)` and you still want to use the
/// faster/more-accurate formula, you'll have to estimate `CCPU_SHIFT` below and possibly
/// adjust `FSHIFT` in "param.h" so that (`FSHIFT >= CCPU_SHIFT`).
///
/// To estimate `CCPU_SHIFT` for exp(-1/20), the following formula was used:
/// 1 - exp(-1/20) ~= 0.0487 ~= 0.0488 == 1 (fixed pt, *11* bits).
///
/// If you don't want to bother with the faster/more-accurate formula, you can set
/// `CCPU_SHIFT` to (`FSHIFT + 1`) which will use a slower/less-accurate (more general)
/// method of calculating the %age of CPU used by a process.
const CCPU_SHIFT: u32 = 11;

/// `roundrobin_period`.
pub fn roundrobin_period() -> u64 {
    ROUNDROBIN_PERIOD.load(Ordering::Relaxed)
}

/// `SCHED_LOCK_INIT()`.
pub fn sched_lock_init() {
    mtx_init(&SCHED_LOCK, IPL_SCHED);
}

/// `SCHED_LOCK()`.
pub fn sched_lock() {
    mtx_enter(&SCHED_LOCK);
}

/// `SCHED_UNLOCK()`.
pub fn sched_unlock() {
    mtx_leave(&SCHED_LOCK);
}

/// `SCHED_ASSERT_LOCKED()`.
pub fn sched_assert_locked() {
    mutex_assert_locked(&SCHED_LOCK, "SCHED_ASSERT_LOCKED");
}

/// `SCHED_ASSERT_UNLOCKED()`.
pub fn sched_assert_unlocked() {
    mutex_assert_unlocked(&SCHED_LOCK, "SCHED_ASSERT_UNLOCKED");
}

/// `roundrobin`: force switch among equal priority processes every 100ms.
pub fn roundrobin(cr: &Clockrequest, _cf: *mut c_void, _arg: *mut c_void) {
    let ci = curcpu();
    let spc = Machine::ci_schedstate(ci);

    let count = clockrequest_advance(cr, roundrobin_period());

    if !Machine::ci_curproc(ci).is_null() {
        if spc.spc_schedflags.load(Ordering::Relaxed) & SPCF_SEENRR != 0 || count >= 2 {
            // The process has already been through a roundrobin without switching and may
            // be hogging the CPU. Indicate that the process should yield.
            spc.spc_schedflags
                .fetch_or(SPCF_SEENRR | SPCF_SHOULDYIELD, Ordering::Relaxed);
        } else {
            spc.spc_schedflags.fetch_or(SPCF_SEENRR, Ordering::Relaxed);
        }
    }

    if spc.spc_nrun.load(Ordering::Relaxed) != 0
        || spc.spc_schedflags.load(Ordering::Relaxed) & SPCF_SHOULDYIELD != 0
    {
        need_resched(ci);
    }
}

/// `update_loadavg`: compute a tenex style load average of a quantity on 1, 5, and 15
/// minute intervals.
pub fn update_loadavg(_unused: *mut c_void) {
    static TO: Timeout = Timeout::new(update_loadavg, ptr::null_mut());

    let set = cpuset_complement(&SCHED_IDLE_CPUS, &SCHED_ALL_CPUS);
    let mut nrun = u64::from(cpuset_cardinality(&set));
    cpu_info_foreach(&mut |ci| {
        nrun += u64::from(Machine::ci_schedstate(ci).spc_nrun.load(Ordering::Relaxed));
    });

    for (ldavg, &cexp) in AVERUNNABLE.ldavg.iter().zip(CEXP.iter()) {
        let old = u64::from(ldavg.load(Ordering::Relaxed));
        let new =
            (u64::from(cexp) * old + nrun * u64::from(FSCALE) * u64::from(FSCALE - cexp)) >> FSHIFT;
        ldavg.store(new as Fixpt, Ordering::Relaxed);
    }

    timeout_add_sec(&TO, 5);
}

/*
 * Constants for digital decay and forget:
 *	90% of (p_estcpu) usage in 5 * loadav time
 *	95% of (p_pctcpu) usage in 60 seconds (load insensitive)
 *          Note that, as ps(1) mentions, this can let percentages
 *          total over 100% (I've seen 137.9% for 3 processes).
 *
 * Note that p_estcpu and p_cpticks are updated independently.
 *
 * We wish to decay away 90% of p_estcpu in (5 * loadavg) seconds.
 * That is, the system wants to compute a value of decay such
 * that the following for loop:
 * 	for (i = 0; i < (5 * loadavg); i++)
 * 		p_estcpu *= decay;
 * will compute
 * 	p_estcpu *= 0.1;
 * for all values of loadavg:
 *
 * Mathematically this loop can be expressed by saying:
 * 	decay ** (5 * loadavg) ~= .1
 *
 * The system computes decay as:
 * 	decay = (2 * loadavg) / (2 * loadavg + 1)
 *
 * We wish to prove that the system's computation of decay
 * will always fulfill the equation:
 * 	decay ** (5 * loadavg) ~= .1
 *
 * If we compute b as:
 * 	b = 2 * loadavg
 * then
 * 	decay = b / (b + 1)
 *
 * We now need to prove two things:
 *	1) Given factor ** (5 * loadavg) ~= .1, prove factor == b/(b+1)
 *	2) Given b/(b+1) ** power ~= .1, prove power == (5 * loadavg)
 *
 * Facts:
 *         For x close to zero, exp(x) =~ 1 + x, since
 *              exp(x) = 0! + x**1/1! + x**2/2! + ... .
 *              therefore exp(-1/b) =~ 1 - (1/b) = (b-1)/b.
 *         For x close to zero, ln(1+x) =~ x, since
 *              ln(1+x) = x - x**2/2 + x**3/3 - ...     -1 < x < 1
 *              therefore ln(b/(b+1)) = ln(1 - 1/(b+1)) =~ -1/(b+1).
 *         ln(.1) =~ -2.30
 *
 * Proof of (1):
 *    Solve (factor)**(power) =~ .1 given power (5*loadav):
 *	solving for factor,
 *      ln(factor) =~ (-2.30/5*loadav), or
 *      factor =~ exp(-1/((5/2.30)*loadav)) =~ exp(-1/(2*loadav)) =
 *          exp(-1/b) =~ (b-1)/b =~ b/(b+1).                    QED
 *
 * Proof of (2):
 *    Solve (factor)**(power) =~ .1 given factor == (b/(b+1)):
 *	solving for power,
 *      power*ln(b/(b+1)) =~ -2.30, or
 *      power =~ 2.3 * (b + 1) = 4.6*loadav + 2.3 =~ 5*loadav.  QED
 *
 * Actual power values for the implemented algorithm are as follows:
 *      loadav: 1       2       3       4
 *      power:  5.68    10.32   14.94   19.55
 */

/// `loadfactor(loadav)`: calculations for digital decay to forget 90% of usage in 5*loadav
/// sec.
const fn loadfactor(loadav: Fixpt) -> Fixpt {
    2 * loadav
}

/// `decay_cpu(loadfac, cpu)`.
const fn decay_cpu(loadfac: Fixpt, cpu: u32) -> u32 {
    ((loadfac as u64 * cpu as u64) / (loadfac as u64 + FSCALE as u64)) as u32
}

/// `schedcpu`: recompute process priorities, every second.
pub fn schedcpu(_unused: *mut c_void) {
    static TO: Timeout = Timeout::new(schedcpu, ptr::null_mut());

    let loadfac = loadfactor(AVERUNNABLE.ldavg[0].load(Ordering::Relaxed));
    let stathz = STATHZ.load(Ordering::Relaxed).max(1) as u32;

    for p in ALLPROC.0.iter() {
        // Idle threads are never placed on the runqueue, therefore computing their priority
        // is pointless.
        if let Some(ci) = p.cpu()
            && ptr::eq(Machine::ci_schedstate(ci).spc_idleproc.get(), p)
        {
            continue;
        }
        // Increment sleep time (if sleeping). We ignore overflow.
        if p.p_stat.get() == SSLEEP || p.p_stat.get() == SSTOP {
            p.p_slptime.set(p.p_slptime.get().wrapping_add(1));
        }
        let mut pctcpu =
            ((u64::from(p.p_pctcpu.load(Ordering::Relaxed)) * u64::from(CCPU)) >> FSHIFT) as Fixpt;
        // If the process has slept the entire second, stop recalculating its priority until
        // it wakes up.
        if p.p_slptime.get() > 1 {
            p.p_pctcpu.store(pctcpu, Ordering::Relaxed);
            continue;
        }
        sched_lock();
        // p_pctcpu is only for diagnostic tools such as ps.
        let cpt = p.p_cpticks.get(); // READ_ONCE
        let dticks = cpt.wrapping_sub(p.p_cpticks2.get());
        // FSHIFT >= CCPU_SHIFT
        pctcpu = pctcpu.wrapping_add(if stathz == 100 {
            dticks << (FSHIFT - CCPU_SHIFT)
        } else {
            100u32.wrapping_mul(dticks << (FSHIFT - CCPU_SHIFT)) / stathz
        });
        p.p_pctcpu.store(pctcpu, Ordering::Relaxed);
        p.p_cpticks2.set(cpt);
        let newcpu = decay_cpu(loadfac, p.p_estcpu.get());
        setpriority(p, newcpu, p.process().ps_nice.get());
        if p.p_stat.get() == SRUN
            && u32::from(p.p_runpri.get()) / SCHED_PPQ != u32::from(p.p_usrpri.get()) / SCHED_PPQ
        {
            remrunqueue(p);
            setrunqueue(p.cpu(), p, p.p_usrpri.get());
        }
        sched_unlock();
    }
    timeout_add_sec(&TO, 1);
}

/// `decay_aftersleep`: recalculate the priority of a process after it has slept for a
/// while. For all load averages >= 1 and max `p_estcpu` of 255, sleeping for at least six
/// times the loadfactor will decay `p_estcpu` to zero.
pub fn decay_aftersleep(estcpu: u32, slptime: u32) -> u32 {
    let loadfac = loadfactor(AVERUNNABLE.ldavg[0].load(Ordering::Relaxed));

    if slptime > 5 * loadfac {
        return 0;
    }
    let mut newcpu = estcpu;
    let mut slptime = slptime.wrapping_sub(1); // the first time was done in schedcpu
    while newcpu != 0 {
        slptime = slptime.wrapping_sub(1);
        if slptime == 0 {
            break;
        }
        newcpu = decay_cpu(loadfac, newcpu);
    }
    newcpu
}

/// `yield`: general yield call. Puts the current process back on its run queue and performs
/// a voluntary context switch.
pub fn r#yield() {
    let p = current();

    sched_lock();
    setrunqueue(p.cpu(), p, p.p_usrpri.get());
    p.p_ru.ru_nvcsw.set(p.p_ru.ru_nvcsw.get() + 1);
    mi_switch();
}

/// `preempt`: general preemption call. Puts the current process back on its run queue and
/// performs an involuntary context switch. If a process is supplied, we switch to that
/// process. Otherwise, we use the normal process selection criteria.
pub fn preempt() {
    let p = current();

    sched_lock();
    setrunqueue(p.cpu(), p, p.p_usrpri.get());
    p.p_ru.ru_nivcsw.set(p.p_ru.ru_nivcsw.get() + 1);
    mi_switch();
}

/// `curproc`, which the scheduler's callers always have.
fn current() -> &'static Proc {
    let Some(p) = curproc() else {
        panic(format_args!("scheduler: no curproc"));
    };
    p
}

/// `mi_switch`: the machine-independent half of a context switch. Called with the scheduler
/// lock held and `p_stat` already changed from `SONPROC`; returns on the same thread once it
/// is scheduled again, with the scheduler lock released.
pub fn mi_switch() {
    let ci = curcpu();
    let spc = Machine::ci_schedstate(ci);
    let p = current();

    kassert!(p.p_stat.get() != SONPROC);
    sched_assert_locked();

    // Release the kernel_lock, as we are about to yield the CPU.
    #[cfg(feature = "multiprocessor")]
    let hold_count = if _kernel_lock_held() {
        __mp_release_all(&KERNEL_LOCK)
    } else {
        0
    };

    // Update thread runtime
    tuagg_add_runtime();

    // Stop any optional clock interrupts.
    if spc.spc_schedflags.load(Ordering::Relaxed) & SPCF_ITIMER != 0 {
        spc.spc_schedflags
            .fetch_and(!SPCF_ITIMER, Ordering::Relaxed);
        clockintr_cancel(&spc.spc_itimer);
    }
    if spc.spc_schedflags.load(Ordering::Relaxed) & SPCF_PROFCLOCK != 0 {
        spc.spc_schedflags
            .fetch_and(!SPCF_PROFCLOCK, Ordering::Relaxed);
        clockintr_cancel(&spc.spc_profclock);
    }

    // Process is about to yield the CPU; clear the appropriate scheduling flags.
    spc.spc_schedflags
        .fetch_and(!SPCF_SWITCHCLEAR, Ordering::Relaxed);

    let nextproc = sched_chooseproc();

    // preserve old IPL level so we can switch back to that
    let oldipl = SCHED_LOCK.mtx_oldipl.get();

    if !ptr::eq(p, nextproc) {
        UVMEXP.swtch.fetch_add(1, Ordering::Relaxed);
        // TRACEPOINT(sched, off__cpu, ...): dt(4), configured in GENERIC.MP, not ported yet.
        // SAFETY: `p` is the running thread, `nextproc` a runnable one the scheduler just
        // took off the run queue (or idle); both have a kernel stack and a pcb, and the
        // scheduler lock serialises the switch.
        unsafe { Machine::cpu_switchto(Some(p), nextproc) };
        // TRACEPOINT(sched, on__cpu, NULL).
    } else {
        // TRACEPOINT(sched, remain__cpu, NULL).
        p.p_stat.set(SONPROC);
    }

    Machine::clear_resched(curcpu());

    sched_assert_locked();

    // Restore proc's IPL.
    SCHED_LOCK.mtx_oldipl.set(oldipl);
    sched_unlock();

    sched_assert_unlocked();

    assertwaitok();
    smr_idle();

    // We're running again; record our new start time. We might be running on a new CPU
    // now, so refetch the schedstate_percpu pointer.
    kassert!(ptr::eq(p.p_cpu.get(), curcpu()));
    let spc = Machine::ci_schedstate(curcpu());

    // Start any optional clock interrupts needed by the thread.
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_ITIMER != 0 {
        spc.spc_schedflags.fetch_or(SPCF_ITIMER, Ordering::Relaxed);
        clockintr_advance(&spc.spc_itimer, hardclock_period());
    }
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_PROFIL != 0 {
        spc.spc_schedflags
            .fetch_or(SPCF_PROFCLOCK, Ordering::Relaxed);
        clockintr_advance(&spc.spc_profclock, profclock_period());
    }

    spc.spc_runtime.set(nanouptime());

    // Reacquire the kernel_lock now. We do this after we've released the scheduler lock to
    // avoid deadlock, and before we reacquire the interlock and the scheduler lock.
    #[cfg(feature = "multiprocessor")]
    if hold_count != 0 {
        __mp_acquire_count(&KERNEL_LOCK, hold_count);
    }
}

/// `setrunnable`: change process state to be runnable, placing it on the run queue.
pub fn setrunnable(p: &Proc) {
    let pr = p.process();

    sched_assert_locked();

    let prio = match p.p_stat.get() {
        SSTOP => {
            let prio = p.p_usrpri.get();
            // TRACEPOINT(sched, unstop, ...): dt(4), configured in GENERIC.MP, not ported yet.
            // If not yet stopped or asleep, unstop but don't add to runq
            if p.p_flag.load(Ordering::Relaxed) & P_INSCHED != 0 {
                if !p.p_wchan.get().is_null() {
                    p.p_stat.set(SSLEEP);
                } else {
                    p.p_stat.set(SONPROC);
                }
                return;
            }
            setrunqueue(None, p, prio);
            prio
        }
        SSLEEP => {
            let prio = p.p_slppri.get();
            // TRACEPOINT(sched, wakeup, ...): dt(4), configured in GENERIC.MP, not ported yet.
            // if not yet asleep, don't add to runqueue
            if p.p_flag.load(Ordering::Relaxed) & P_INSCHED != 0 {
                return;
            }
            setrunqueue(None, p, prio);
            prio
        }
        // 0, SRUN, SONPROC, SDEAD, SIDL and anything else
        _ => panic(format_args!("setrunnable")),
    };
    let _ = prio;

    if p.p_slptime.get() > 1 {
        let newcpu = decay_aftersleep(p.p_estcpu.get(), p.p_slptime.get());
        setpriority(p, newcpu, pr.ps_nice.get());
    }
    p.p_slptime.set(0);
}

/// `setpriority`: compute the priority of a process.
pub fn setpriority(p: &Proc, newcpu: u32, nice: u8) {
    let newprio = (i64::from(PUSER)
        + i64::from(newcpu)
        + i64::from(NICE_WEIGHT) * (i64::from(nice) - i64::from(NZERO)))
    .min(i64::from(MAXPRI));

    sched_assert_locked();
    p.p_estcpu.set(newcpu);
    p.p_usrpri.set(newprio.clamp(0, 255) as u8);
}

/// `schedclock`: we adjust the priority of the current process. The priority of a process
/// gets worse as it accumulates CPU time. The cpu usage estimator (`p_estcpu`) is increased
/// here. The formula for computing priorities (in `kern_synch.c`) will compute a different
/// value each time `p_estcpu` increases. This can cause a switch, but unless the priority
/// crosses a PPQ boundary the actual queue will not change. The cpu usage estimator ramps
/// up quite quickly when the process is running (linearly), and decays away exponentially,
/// at a rate which is proportionally slower when the system is busy. The basic principle is
/// that the system will 90% forget that the process used a lot of CPU time in 5 * loadav
/// seconds. This causes the system to favor processes which haven't run much recently, and
/// to round-robin among other processes.
pub fn schedclock(p: &Proc) {
    let ci = curcpu();
    let spc = Machine::ci_schedstate(ci);

    if ptr::eq(p, spc.spc_idleproc.get()) || spc.spc_spinning.load(Ordering::Relaxed) != 0 {
        return;
    }

    sched_lock();
    let newcpu = estcpulim(p.p_estcpu.get() + 1);
    setpriority(p, newcpu, p.process().ps_nice.get());
    sched_unlock();
}

/// `PERFPOL_MANUAL`: the level is what `hw.setperf` says.
const PERFPOL_MANUAL: i32 = 0;
/// `PERFPOL_AUTO`: `setperf_auto` follows the load.
const PERFPOL_AUTO: i32 = 1;
/// `PERFPOL_HIGH`: full speed.
const PERFPOL_HIGH: i32 = 2;

/// `perflevel`: the CPU performance level in use, 0 to 100.
pub static PERFLEVEL: AtomicI32 = AtomicI32::new(100);
/// `perfpolicy_on_ac`.
pub static PERFPOLICY_ON_AC: AtomicI32 = AtomicI32::new(PERFPOL_HIGH);
/// `perfpolicy_on_battery`.
pub static PERFPOLICY_ON_BATTERY: AtomicI32 = AtomicI32::new(PERFPOL_AUTO);
/// `setperf_to`: `setperf_auto`'s timeout. The code below handles CPU throttling.
static SETPERF_TO: Timeout = Timeout::new(setperf_auto, ptr::null_mut());
/// `setperf_auto`'s `idleticks[]`: each CPU's idle ticks at the last look (see the module's
/// deviations).
static SETPERF_IDLETICKS: [AtomicU64; MAXCPUS as usize] =
    [const { AtomicU64::new(0) }; MAXCPUS as usize];
/// `setperf_auto`'s `totalticks[]`.
static SETPERF_TOTALTICKS: [AtomicU64; MAXCPUS as usize] =
    [const { AtomicU64::new(0) }; MAXCPUS as usize];
/// `setperf_auto`'s `downbeats`: how many quiet looks before the level drops.
static SETPERF_DOWNBEATS: AtomicI32 = AtomicI32::new(0);

/// `cpu_setperf`, if a driver installed one.
fn cpu_setperf() -> Option<fn(i32)> {
    // SAFETY: written once, while cold, by the attaching driver.
    unsafe { CPU_SETPERF.read() }
}

/// `perfpolicy_dynamic`: whether either policy lets the level follow the load.
fn perfpolicy_dynamic() -> bool {
    PERFPOLICY_ON_AC.load(Ordering::Relaxed) == PERFPOL_AUTO
        || PERFPOLICY_ON_BATTERY.load(Ordering::Relaxed) == PERFPOL_AUTO
}

/// `current_perfpolicy`: the policy of the power source in use.
fn current_perfpolicy() -> i32 {
    if hw_power.load(Ordering::Relaxed) != 0 {
        PERFPOLICY_ON_AC.load(Ordering::Relaxed)
    } else {
        PERFPOLICY_ON_BATTERY.load(Ordering::Relaxed)
    }
}

/// `setperf_auto`: every 100 ms under an automatic policy, full speed when a CPU was idle
/// for less than a third of the time, or all of them for less than half, since the last
/// look; the lowest level after five quiet looks.
pub fn setperf_auto(_v: *mut c_void) {
    if !perfpolicy_dynamic() {
        return;
    }

    let Some(setperf) = cpu_setperf() else {
        return;
    };

    let high = current_perfpolicy() == PERFPOL_HIGH;
    let mut speedup = high;
    if !high {
        let mut j = 0;
        let (mut allidle, mut alltotal) = (0u64, 0u64);
        cpu_info_foreach(&mut |ci| {
            if !cpu_is_online(ci) || j >= MAXCPUS as usize {
                return;
            }

            let spc = Machine::ci_schedstate(ci);
            let (mut total, mut idle);
            let mut generation = 0;
            pc_cons_enter(&spc.spc_cp_time_lock, &mut generation);
            loop {
                total = 0u64;
                for c in &spc.spc_cp_time {
                    total = total.wrapping_add(c.load(Ordering::Relaxed));
                }
                idle = spc.spc_cp_time[CP_IDLE].load(Ordering::Relaxed);
                if !pc_cons_leave(&spc.spc_cp_time_lock, &mut generation) {
                    break;
                }
            }

            total = total.wrapping_sub(SETPERF_TOTALTICKS[j].load(Ordering::Relaxed));
            idle = idle.wrapping_sub(SETPERF_IDLETICKS[j].load(Ordering::Relaxed));
            if idle < total / 3 {
                speedup = true;
            }
            alltotal = alltotal.wrapping_add(total);
            allidle = allidle.wrapping_add(idle);
            SETPERF_IDLETICKS[j].fetch_add(idle, Ordering::Relaxed);
            SETPERF_TOTALTICKS[j].fetch_add(total, Ordering::Relaxed);
            j += 1;
        });
        if allidle < alltotal / 2 {
            speedup = true;
        }
        if speedup && SETPERF_DOWNBEATS.load(Ordering::Relaxed) < 5 {
            SETPERF_DOWNBEATS.fetch_add(1, Ordering::Relaxed);
        }
    }

    let perflevel = PERFLEVEL.load(Ordering::Relaxed);
    if high || (speedup && perflevel != 100) {
        // faster:
        PERFLEVEL.store(100, Ordering::Relaxed);
        setperf(100);
    } else if !speedup
        && perflevel != 0
        && SETPERF_DOWNBEATS.fetch_sub(1, Ordering::Relaxed) - 1 <= 0
    {
        PERFLEVEL.store(0, Ordering::Relaxed);
        setperf(0);
    }

    timeout_add_msec(&SETPERF_TO, 100);
}

/// `sysctl_hwsetperf`: `hw.setperf`, the performance level; it can be set under the manual
/// policy only.
pub fn sysctl_hwsetperf(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let Some(setperf) = cpu_setperf() else {
        return Err(Errno::EOPNOTSUPP);
    };

    if PERFPOLICY_ON_AC.load(Ordering::Relaxed) != PERFPOL_MANUAL {
        return sysctl_rdint(oldp, oldlenp, newp, PERFLEVEL.load(Ordering::Relaxed));
    }

    sysctl_int_bounded(oldp, oldlenp, newp, newlen, &PERFLEVEL, 0, 100)?;

    if newp != 0 {
        setperf(PERFLEVEL.load(Ordering::Relaxed));
    }

    Ok(())
}

/// `sysctl_hwperfpolicy`: `hw.perfpolicy`: "manual", "auto" or "high", or two of them
/// separated by a comma for wall power and battery ("high,auto"; "manual" on neither side
/// of a pair).
pub fn sysctl_hwperfpolicy(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let Some(setperf) = cpu_setperf() else {
        return Err(Errno::EOPNOTSUPP);
    };

    let name: &[u8] = match current_perfpolicy() {
        PERFPOL_MANUAL => b"manual",
        PERFPOL_AUTO => b"auto",
        PERFPOL_HIGH => b"high",
        _ => b"unknown",
    };
    let mut policy = [0u8; 32];
    policy[..name.len()].copy_from_slice(name);

    if newp == 0 {
        return sysctl_rdstring(oldp, oldlenp, newp, name);
    }

    sysctl_string(oldp, oldlenp, newp, newlen, &mut policy)?;

    let len = policy.iter().position(|b| *b == 0).unwrap_or(policy.len());
    let policy = &policy[..len];
    let (policy, policy_on_battery) = match policy.iter().position(|b| *b == b',') {
        Some(comma) => (&policy[..comma], Some(&policy[comma + 1..])),
        None => (policy, None),
    };

    let perfpolicy = match policy {
        b"manual" => PERFPOL_MANUAL,
        b"auto" => PERFPOL_AUTO,
        b"high" => PERFPOL_HIGH,
        _ => return Err(Errno::EINVAL),
    };

    match policy_on_battery {
        None => {
            PERFPOLICY_ON_BATTERY.store(perfpolicy, Ordering::Relaxed);
            PERFPOLICY_ON_AC.store(perfpolicy, Ordering::Relaxed);
        }
        Some(on_battery) => {
            if on_battery == b"manual" || perfpolicy == PERFPOL_MANUAL {
                // Not handled
                return Err(Errno::EINVAL);
            }
            match on_battery {
                b"auto" => PERFPOLICY_ON_BATTERY.store(PERFPOL_AUTO, Ordering::Relaxed),
                b"high" => PERFPOLICY_ON_BATTERY.store(PERFPOL_HIGH, Ordering::Relaxed),
                _ => return Err(Errno::EINVAL),
            }
            PERFPOLICY_ON_AC.store(perfpolicy, Ordering::Relaxed);
        }
    }

    if current_perfpolicy() == PERFPOL_HIGH {
        PERFLEVEL.store(100, Ordering::Relaxed);
        setperf(100);
    }

    if perfpolicy_dynamic() {
        timeout_add_msec(&SETPERF_TO, 200);
    }

    Ok(())
}

/// `scheduler_start`: start the scheduler's periodic timeouts.
pub fn scheduler_start() {
    schedcpu(ptr::null_mut());
    update_loadavg(ptr::null_mut());

    if perfpolicy_dynamic() {
        timeout_add_msec(&SETPERF_TO, 200);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_constants_match_the_c_doubles() {
        // 0.9200444146293232 * 2048, 0.9834714538216174 * 2048, 0.9944598480048967 * 2048
        assert_eq!(CEXP, [1884, 2014, 2036]);
        // 0.95122942450071400909 * 2048
        assert_eq!(CCPU, 1948);
    }

    #[test]
    fn decay_cpu_forgets_ninety_percent_in_five_loadav() {
        // loadav 1.0: loadfac 2*FSCALE; after 5 decays 100 -> about 10.
        let loadfac = loadfactor(FSCALE);
        let mut cpu = 100;
        for _ in 0..5 {
            cpu = decay_cpu(loadfac, cpu);
        }
        assert!((8..=16).contains(&cpu), "{cpu}");
    }

    #[test]
    fn decay_aftersleep_zeroes_long_sleeps() {
        AVERUNNABLE.ldavg[0].store(FSCALE, Ordering::Relaxed);
        assert_eq!(decay_aftersleep(200, 5 * loadfactor(FSCALE) + 1), 0);
        assert!(decay_aftersleep(200, 3) < 200);
        AVERUNNABLE.ldavg[0].store(0, Ordering::Relaxed);
    }

    use std::vec::Vec;

    std::thread_local! {
        /// The levels the fake `cpu_setperf` was called with.
        static LEVELS: core::cell::RefCell<Vec<i32>> = const { core::cell::RefCell::new(Vec::new()) };
    }

    fn fake_setperf(level: i32) {
        LEVELS.with(|l| l.borrow_mut().push(level));
    }

    fn levels() -> Vec<i32> {
        LEVELS.with(|l| core::mem::take(&mut *l.borrow_mut()))
    }

    /// `sysctl -w hw.perfpolicy=<policy>`.
    fn set_policy(policy: &[u8]) -> Result<(), Errno> {
        let mut len = 0usize;
        sysctl_hwperfpolicy(0, &mut len, policy.as_ptr() as usize, policy.len())
    }

    fn policies() -> (i32, i32) {
        (
            PERFPOLICY_ON_AC.load(Ordering::Relaxed),
            PERFPOLICY_ON_BATTERY.load(Ordering::Relaxed),
        )
    }

    /// One test, because the throttling is a set of globals.
    #[test]
    fn throttling_follows_the_policy_knobs() {
        use crate::kern::kern_timeout::{timeout_del, timeout_startup};
        use crate::sys::timeout::timeout_pending;

        let _t = crate::kern::kern_timeout::tests::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        timeout_startup();
        let mut len = 0usize;
        let fifty = 50i32;
        let fifty_p = ptr::from_ref(&fifty) as usize;

        // No driver gave a cpu_setperf: neither knob exists and the timeout does nothing.
        // SAFETY: the test's own hook, under the lock above; put back at the end.
        unsafe { CPU_SETPERF.write(None) };
        assert_eq!(sysctl_hwsetperf(0, &mut len, 0, 0), Err(Errno::EOPNOTSUPP));
        assert_eq!(set_policy(b"high"), Err(Errno::EOPNOTSUPP));
        setperf_auto(ptr::null_mut());
        assert!(!timeout_pending(&SETPERF_TO));

        // SAFETY: as above.
        unsafe { CPU_SETPERF.write(Some(fake_setperf)) };
        hw_power.store(1, Ordering::Relaxed);
        PERFPOLICY_ON_AC.store(PERFPOL_HIGH, Ordering::Relaxed);
        PERFPOLICY_ON_BATTERY.store(PERFPOL_AUTO, Ordering::Relaxed);
        PERFLEVEL.store(100, Ordering::Relaxed);
        SETPERF_DOWNBEATS.store(0, Ordering::Relaxed);

        // The policy of the power source in use is what is read.
        let mut out = [0xffu8; 8];
        let mut len = out.len();
        assert_eq!(
            sysctl_hwperfpolicy(out.as_mut_ptr() as usize, &mut len, 0, 0),
            Ok(())
        );
        assert_eq!((len, &out[..5]), (5, &b"high\0"[..]));
        hw_power.store(0, Ordering::Relaxed);
        let mut len = out.len();
        assert_eq!(
            sysctl_hwperfpolicy(out.as_mut_ptr() as usize, &mut len, 0, 0),
            Ok(())
        );
        assert_eq!((len, &out[..5]), (5, &b"auto\0"[..]));
        hw_power.store(1, Ordering::Relaxed);

        // hw.setperf is read-only unless the policy is manual.
        assert_eq!(sysctl_hwsetperf(0, &mut len, fifty_p, 4), Err(Errno::EPERM));
        assert_eq!(set_policy(b"manual"), Ok(()));
        assert_eq!(policies(), (PERFPOL_MANUAL, PERFPOL_MANUAL));
        assert!(levels().is_empty());
        assert!(!timeout_pending(&SETPERF_TO));
        assert_eq!(sysctl_hwsetperf(0, &mut len, fifty_p, 4), Ok(()));
        assert_eq!(PERFLEVEL.load(Ordering::Relaxed), 50);
        assert_eq!(levels(), [50]);
        let over = 101i32;
        assert_eq!(
            sysctl_hwsetperf(0, &mut len, ptr::from_ref(&over) as usize, 4),
            Err(Errno::EINVAL)
        );
        assert_eq!(PERFLEVEL.load(Ordering::Relaxed), 50);
        // A read calls nothing.
        let mut level = 0i32;
        let mut len = 4usize;
        assert_eq!(
            sysctl_hwsetperf(ptr::from_mut(&mut level) as usize, &mut len, 0, 0),
            Ok(())
        );
        assert_eq!(level, 50);
        assert!(levels().is_empty());

        // What the policy cannot be: an unknown word, or "manual" on either side of a pair.
        for bad in [
            &b"bogus"[..],
            b"manual,auto",
            b"high,manual",
            b"high,bogus",
            b"",
        ] {
            assert_eq!(set_policy(bad), Err(Errno::EINVAL), "{bad:?}");
        }
        assert_eq!(policies(), (PERFPOL_MANUAL, PERFPOL_MANUAL));
        assert!(levels().is_empty());

        // "auto": setperf_auto runs every 100 ms and picks one end or the other. From a
        // level in between, a busy look goes up and a quiet one, with no beats left, down.
        assert_eq!(set_policy(b"auto"), Ok(()));
        assert_eq!(policies(), (PERFPOL_AUTO, PERFPOL_AUTO));
        assert!(levels().is_empty());
        assert!(timeout_pending(&SETPERF_TO));
        timeout_del(&SETPERF_TO);
        setperf_auto(ptr::null_mut());
        let level = PERFLEVEL.load(Ordering::Relaxed);
        assert!(level == 0 || level == 100, "{level}");
        assert_eq!(levels(), [level]);
        assert!(timeout_pending(&SETPERF_TO));

        // A pair: wall power first. Full speed is set at once.
        PERFLEVEL.store(0, Ordering::Relaxed);
        assert_eq!(set_policy(b"high,auto"), Ok(()));
        assert_eq!(policies(), (PERFPOL_HIGH, PERFPOL_AUTO));
        assert_eq!(PERFLEVEL.load(Ordering::Relaxed), 100);
        assert_eq!(levels(), [100]);
        // The battery's policy is automatic, so the timeout still runs, and on wall power
        // it sets full speed without looking at the load.
        timeout_del(&SETPERF_TO);
        setperf_auto(ptr::null_mut());
        assert_eq!(levels(), [100]);
        assert!(timeout_pending(&SETPERF_TO));

        // "high" on both: nothing left to follow, the timeout does not re-arm.
        assert_eq!(set_policy(b"high"), Ok(()));
        assert_eq!(policies(), (PERFPOL_HIGH, PERFPOL_HIGH));
        assert_eq!(levels(), [100]);
        timeout_del(&SETPERF_TO);
        setperf_auto(ptr::null_mut());
        assert!(levels().is_empty());
        assert!(!timeout_pending(&SETPERF_TO));

        // As at boot.
        // SAFETY: as above.
        unsafe { CPU_SETPERF.write(None) };
        PERFPOLICY_ON_AC.store(PERFPOL_HIGH, Ordering::Relaxed);
        PERFPOLICY_ON_BATTERY.store(PERFPOL_AUTO, Ordering::Relaxed);
        PERFLEVEL.store(100, Ordering::Relaxed);
        SETPERF_DOWNBEATS.store(0, Ordering::Relaxed);
    }
}
/* </TESTS> */
