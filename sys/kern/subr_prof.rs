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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)subr_prof.c	8.3 (Berkeley) 9/23/93
 */
/* </LICENSES> */

/* <CODE> */
//! Profiling support: `kern/subr_prof.c`.
//!
//! Upstream: sys/kern/subr_prof.c @ 3ce1f3f79392
//!
//! Status: `ported`: `profclock_period`, `sys_profil`, `prof_fork`, `prof_exec`,
//! `prof_write`, `profclock`, `PC_TO_INDEX`, `addupc_intr` and `addupc_task`. A process
//! linked for profiling hands the kernel a buffer with `profil(2)`; the profiling clock
//! notes where each tick found it (`addupc_intr`), the next AST adds the tick to the
//! buffer's counter (`addupc_task`), and `exit1` writes the buffer to `gmon.<name>.<pid>.out`
//! in the directory and with the credentials of the first `profil(2)` call (`prof_write`).
//!
//! ## Deviations
//! - The kernel's own profiling (`GPROF`, `DDBPROF`: `prof_init`, `prof_state_toggle`,
//!   `sysctl_doprof`, `gmonclock`, `gmoninit`, `gmon_cpu_count`) is not configured.
//! - `sys_profil` gives `ENOENT` to a process that has no current directory and names no
//!   `dirfd`. The C's processes always have one; here the first process runs from the boot
//!   module until a root file system is mounted.
//! - The `#if 0` block that would make the buffer immutable is not carried over.

use core::ffi::c_void;
use core::mem::offset_of;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::kern::kern_clock::{startprofclock, stopprofclock};
use crate::kern::kern_clockintr::clockrequest_advance;
use crate::kern::kern_descrip::fd_getfile;
use crate::kern::kern_prot::{crdup, crfree, crhold};
use crate::kern::subr_prf::{Str, snprintf};
use crate::kern::vfs_lookup::ndinit;
use crate::kern::vfs_subr::{vattr_null, vref, vrele};
use crate::kern::vfs_syscalls::NameiBuf;
use crate::kern::vfs_vnops::{vn_close, vn_open, vn_rdwr};
use crate::kern::vfs_vops::{VOP_GETATTR, VOP_SETATTR, VOP_UNLOCK};
use crate::machine::Machine;
use crate::machine::copy::{copyin, copyout};
use crate::machine::cpu::need_resched;
use crate::machine::cpu::{ClockFrame, Cpu, curcpu};
use crate::machine::intr::{splstatclock, splx};
use crate::sys::clockintr::Clockrequest;
use crate::sys::errno::Errno;
use crate::sys::fcntl::{FWRITE, O_CREAT, O_NOFOLLOW, O_NONBLOCK};
use crate::sys::file::{DTYPE_VNODE, frele};
use crate::sys::gmon::Gmonhdr;
use crate::sys::namei::{BYPASSUNVEIL, KERNELPATH, NiDirp};
use crate::sys::param::MAXPATHLEN;
use crate::sys::proc::{P_OWEUPC, PS_PROFIL, PSI_PROFILE, Proc, Process, Uprof};
use crate::sys::stat::{S_IRUSR, S_IWUSR};
use crate::sys::syscallargs::SysProfilArgs;
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::Register;
use crate::sys::uio::{UioRw, UioSeg};
use crate::sys::vnode::{IO_UNIT, VDIR, VREAD, VREG, VWRITE, Vattr, Vnode};

/// `profclock_period`.
pub static PROFCLOCK_PERIOD: AtomicU64 = AtomicU64::new(0);

/// `profclock_period`.
pub fn profclock_period() -> u64 {
    PROFCLOCK_PERIOD.load(Ordering::Relaxed)
}

/// Profiling system call.
///
/// The scale factor is a fixed point number with 16 bits of fraction, so that 1.0 is
/// represented as 0x10000. A scale factor of 0 turns off profiling.
pub fn sys_profil(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysProfilArgs = sysargs(v);
    let pr = p.process();
    let buf = uap.buf.get() as usize;

    // Only binaries linked for profiling can do profil()
    if pr.ps_iflags.get() & PSI_PROFILE == 0 {
        return Err(Errno::EPERM);
    }

    if uap.scale.get() > (1 << 16) {
        return Err(Errno::EINVAL);
    }
    if uap.scale.get() == 0 {
        stopprofclock(pr);
        need_resched(curcpu());
        return Ok(());
    }
    let upp = &pr.ps_prof;

    // May not change to a new buffer
    if upp.pr_buf.get() != 0 && (buf != upp.pr_buf.get() || uap.buflen.get() != upp.pr_buflen.get())
    {
        return Err(Errno::EALREADY);
    }

    // First call determines where the output files will go
    if upp.pr_cdir.get().is_none() {
        let dirvp: &'static Vnode;

        if uap.dirfd.get() != -1 {
            let Some(fp) = fd_getfile(p.fd(), uap.dirfd.get()) else {
                return Err(Errno::EBADF);
            };
            if fp.f_type.get() != DTYPE_VNODE || fp.vnode().v_type.get() != VDIR {
                let _ = frele(fp, p);
                return Err(Errno::ENOTDIR);
            }
            dirvp = fp.vnode();
            vref(dirvp);
            let _ = frele(fp, p);
        } else {
            // See the deviations: no current directory before a root file system.
            let Some(cdir) = p.fd().fd_cdir.get() else {
                return Err(Errno::ENOENT);
            };
            dirvp = cdir;
            vref(dirvp);
        }
        // fd_getfile() and FRELE() can both sleep, so a sibling thread may have published
        // while we were in here. First one to arrive wins; drop what this one took.
        if upp.pr_cdir.get().is_none() {
            upp.pr_cdir.set(Some(dirvp));
            upp.pr_ucred.set(Some(crhold(p.ucred())));
        } else {
            vrele(dirvp);
        }
    }

    // Block profile interrupts while changing state.
    let s = splstatclock();
    upp.pr_off.set(uap.offset.get());
    upp.pr_scale.set(uap.scale.get());
    upp.pr_base.set(buf.wrapping_add(size_of::<Gmonhdr>()));
    upp.pr_size.set(uap.samplesize.get());
    upp.pr_buf.set(buf);
    upp.pr_buflen.set(uap.buflen.get());
    startprofclock(pr);
    splx(s);
    need_resched(curcpu());

    Ok(())
}

/// `prof_fork`: the child's copy of `ps_prof` takes its own references.
pub fn prof_fork(pr: &Process) {
    let upp = &pr.ps_prof;

    if let Some(cdir) = upp.pr_cdir.get() {
        vref(cdir);
    }
    if let Some(cred) = upp.pr_ucred.get() {
        crhold(cred);
    }
}

/// `prof_exec`: clear profiling state in new image.
pub fn prof_exec(pr: &Process) {
    let upp = &pr.ps_prof;

    if let Some(cdir) = upp.pr_cdir.take() {
        vrele(cdir);
    }
    if let Some(cred) = upp.pr_ucred.take() {
        crfree(cred);
    }
    upp.pr_buf.set(0);
}

/// `prof_write`: the exiting process's profile, written to `gmon.<name>.<pid>.out` in the
/// directory and with the credentials of its first `profil(2)` call.
pub fn prof_write(p: &Proc) {
    let pr = p.process();
    let upp = &pr.ps_prof;

    // Is this process profiling?
    if upp.pr_buf.get() == 0 {
        return;
    }

    let fdp = p.fd();
    let mut cdir = upp.pr_cdir.get();
    let cred = upp.pr_ucred.get();

    'out: {
        // Set with pr_buf, by sys_profil.
        let Some(cred) = cred else {
            break 'out;
        };

        // Process sets hdr.totarc to indicate space used by the arc tables
        let mut hdr = [0u8; size_of::<Gmonhdr>()];
        if copyin(upp.pr_buf.get(), &mut hdr).is_err() {
            break 'out;
        }
        let at = offset_of!(Gmonhdr, totarc);
        let totarc = i32::from_ne_bytes([hdr[at], hdr[at + 1], hdr[at + 2], hdr[at + 3]]);
        if totarc <= 0 {
            break 'out;
        }
        let size = size_of::<Gmonhdr>()
            .wrapping_add(upp.pr_size.get())
            .wrapping_add(totarc as usize);
        if size > upp.pr_buflen.get() {
            break 'out;
        }

        let Ok(name) = NameiBuf::get() else {
            break 'out;
        };
        let len = snprintf(
            name.as_mut(),
            format_args!("gmon.{}.{}.out", Str(pr.comm()), pr.ps_pid.get()),
        );
        if len >= MAXPATHLEN {
            break 'out;
        }

        // Swap back to the original directory
        let old_cdir = fdp.fd_cdir.get();
        fdp.fd_cdir.set(cdir);
        if let Some(old) = old_cdir {
            vrele(old);
        }
        cdir = None; // will be released as fdp->fd_cdir

        // Swap back to the original cred
        let old_cred = p.ucred();
        p.p_ucred.set(crdup(cred));
        crfree(old_cred);

        let mut nd = ndinit(0, BYPASSUNVEIL | KERNELPATH, NiDirp::Sys(name.as_str()), p);
        if vn_open(
            &mut nd,
            O_CREAT | FWRITE | O_NOFOLLOW | O_NONBLOCK,
            S_IRUSR | S_IWUSR,
        )
        .is_err()
        {
            break 'out;
        }
        let Some(vp) = nd.ni_vp else {
            break 'out;
        };

        // Don't dump to non-regular files, files with links, or files owned by someone
        // else.
        let mut vattr = Vattr::new();
        if VOP_GETATTR(vp, &mut vattr, cred, p).is_err() {
            let _ = VOP_UNLOCK(vp);
            let _ = vn_close(vp, FWRITE, cred, Some(p));
            break 'out;
        }
        if vp.v_type.get() != VREG
            || vattr.va_nlink != 1
            || vattr.va_mode & (((VREAD | VWRITE) >> 3 | (VREAD | VWRITE) >> 6) as u32) != 0
            || vattr.va_uid != cred.cr_uid.get()
        {
            let _ = VOP_UNLOCK(vp);
            let _ = vn_close(vp, FWRITE, cred, Some(p));
            break 'out;
        }
        vattr_null(&mut vattr);
        vattr.va_size = 0;
        let _ = VOP_SETATTR(vp, &mut vattr, cred, p);

        let _ = VOP_UNLOCK(vp);
        vref(vp);
        if vn_close(vp, FWRITE, cred, Some(p)).is_ok() {
            let _ = vn_rdwr(
                UioRw::UIO_WRITE,
                vp,
                upp.pr_buf.get() as *mut c_void,
                size,
                0,
                UioSeg::UIO_USERSPACE,
                IO_UNIT,
                cred,
                None,
                Some(p),
            );
        }
        vrele(vp);
    }

    // out:
    if let Some(cred) = cred {
        crfree(cred);
    }
    if let Some(cdir) = cdir {
        vrele(cdir);
    }
}

/// `profclock`: the profiling clock interrupt.
pub fn profclock(cr: &Clockrequest, cf: *mut c_void, _arg: *mut c_void) {
    let p = Machine::ci_curproc(curcpu());

    // if (count > ULONG_MAX) count = ULONG_MAX: a no-op on LP64.
    let count = clockrequest_advance(cr, profclock_period());

    // SAFETY: the dispatcher passes the clock frame the interrupt entry built, and `curproc`
    // is null or the running thread, which cannot go away under its own interrupt.
    let (frame, p) = unsafe { (cf.cast::<ClockFrame>().as_ref(), p.as_ref()) };
    let Some(p) = p else {
        return;
    };
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_PROFIL == 0 {
        return;
    }
    match frame {
        Some(frame) if Machine::clkf_usermode(frame) => {
            addupc_intr(p, Machine::clkf_pc(frame) as u64, count);
        }
        _ => addupc_intr(p, Machine::proc_pc(p) as u64, count),
    }
}

/// `PC_TO_INDEX(pc, prof)`: the offset of `pc`'s counter in the sample buffer.
///
/// Scale is a fixed-point number with the binary point 16 bits into the value, and is
/// <= 1.0. pc is at most 32 bits, so the intermediate result is at most 48 bits.
fn pc_to_index(pc: u64, prof: &Uprof) -> i32 {
    let scaled = pc
        .wrapping_sub(prof.pr_off.get())
        .wrapping_mul(u64::from(prof.pr_scale.get()));
    ((scaled >> 16) as i32) & !1
}

/// Collect user-level profiling statistics; called on a profiling tick, when a process is
/// running in user-mode. This routine may be called from an interrupt context. Schedule an
/// AST that will vector us to trap() with a context in which copyin and copyout will work.
/// Trap will then call `addupc_task`.
pub fn addupc_intr(p: &Proc, pc: u64, nticks: u64) {
    let prof = &p.process().ps_prof;
    // The `int` index is compared as a `size_t`: a negative one is out of range too.
    if pc < prof.pr_off.get() || pc_to_index(pc, prof) as isize as usize >= prof.pr_size.get() {
        return; // out of range; ignore
    }

    p.p_prof_addr.set(pc);
    p.p_prof_ticks
        .set(p.p_prof_ticks.get().wrapping_add(nticks));
    p.p_flag.fetch_or(P_OWEUPC, Ordering::Relaxed);
    Machine::need_proftick(p);
}

/// Much like before, but we can afford to take faults here. If the update fails, we simply
/// turn off profiling.
pub fn addupc_task(p: &Proc, pc: u64, nticks: u32) {
    let pr = p.process();

    // Testing PS_PROFIL may be unnecessary, but is certainly safe.
    if pr.ps_flags.load(Ordering::Relaxed) & PS_PROFIL == 0 || nticks == 0 {
        return;
    }

    let prof = &pr.ps_prof;
    // The C's `u_int i`.
    let i = pc_to_index(pc, prof) as u32 as usize;
    if pc < prof.pr_off.get() || i >= prof.pr_size.get() {
        return;
    }

    let addr = prof.pr_base.get().wrapping_add(i);
    let mut v = [0u8; 2];
    if copyin(addr, &mut v).is_ok() {
        let n = u16::from_ne_bytes(v).wrapping_add(nticks as u16);
        if copyout(&n.to_ne_bytes(), addr).is_ok() {
            return;
        }
    }
    stopprofclock(pr);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    extern crate std;

    use std::boxed::Box;

    use super::*;

    /// A thread of a process that profiles `size` bytes of counters at `base`, for the text
    /// from `off`, one counter for every `1 / scale` (16.16) bytes.
    fn profiled(base: usize, size: usize, off: u64, scale: u32) -> &'static Proc {
        let pr: &'static Process = Box::leak(Box::new(Process::new()));
        let p: &'static Proc = Box::leak(Box::new(Proc::new()));
        p.p_p.set(pr);
        pr.ps_prof.pr_base.set(base);
        pr.ps_prof.pr_size.set(size);
        pr.ps_prof.pr_off.set(off);
        pr.ps_prof.pr_scale.set(scale);
        pr.ps_flags.fetch_or(PS_PROFIL, Ordering::Relaxed);
        p
    }

    #[test]
    fn a_pc_maps_to_an_even_offset_of_the_buffer() {
        let prof = Uprof::new();
        prof.pr_off.set(0x1000);
        // Scale 1.0: one byte of counter per byte of text, so one counter per two bytes.
        prof.pr_scale.set(0x10000);
        assert_eq!(pc_to_index(0x1000, &prof), 0);
        assert_eq!(pc_to_index(0x1001, &prof), 0);
        assert_eq!(pc_to_index(0x1002, &prof), 2);
        assert_eq!(pc_to_index(0x1235, &prof), 0x234);
        // Scale 0.5: one counter per four bytes.
        prof.pr_scale.set(0x8000);
        assert_eq!(pc_to_index(0x1004, &prof), 2);
        assert_eq!(pc_to_index(0x1007, &prof), 2);
        assert_eq!(pc_to_index(0x1008, &prof), 4);
        // An index past INT_MAX is negative, as the C's `int`.
        prof.pr_scale.set(0x10000);
        assert_eq!(pc_to_index(0x1000 + 0x8000_0000, &prof), i32::MIN);
    }

    #[test]
    fn a_tick_in_range_is_owed_at_the_next_ast() {
        let p = profiled(0, 16, 0x1000, 0x10000);
        // Below the offset, past the buffer, and an index that is negative as an `int`.
        addupc_intr(p, 0x0fff, 3);
        addupc_intr(p, 0x2000, 3);
        addupc_intr(p, 0x1000 + 0x8000_0000, 3);
        assert_eq!(p.p_flag.load(Ordering::Relaxed) & P_OWEUPC, 0);
        assert_eq!(p.p_prof_ticks.get(), 0);

        addupc_intr(p, 0x1004, 3);
        addupc_intr(p, 0x1006, 2);
        assert_ne!(p.p_flag.load(Ordering::Relaxed) & P_OWEUPC, 0);
        assert_eq!(p.p_prof_addr.get(), 0x1006);
        assert_eq!(p.p_prof_ticks.get(), 5);
    }

    #[test]
    fn the_ast_adds_the_ticks_to_the_counter() {
        let mut counters = [0u16; 8];
        counters[3] = 0xfffe;
        let base = counters.as_mut_ptr() as usize;
        let p = profiled(base, 16, 0x1000, 0x10000);

        addupc_task(p, 0x1004, 5);
        addupc_task(p, 0x1005, 1);
        // A counter wraps, as the C's u_short does.
        addupc_task(p, 0x1006, 3);
        // Out of range, and no ticks: nothing.
        addupc_task(p, 0x1010, 1);
        addupc_task(p, 0x1000, 0);
        // SAFETY: only `addupc_task` wrote the array, through `base`.
        let seen = unsafe { core::ptr::read_volatile(&raw const counters) };
        assert_eq!(seen, [0, 0, 6, 1, 0, 0, 0, 0]);
        assert_ne!(p.process().ps_flags.load(Ordering::Relaxed) & PS_PROFIL, 0);
    }

    #[test]
    fn a_fault_on_the_buffer_turns_profiling_off() {
        // The host's copyin refuses address 0.
        let p = profiled(0, 16, 0x1000, 0x10000);
        crate::kern::kern_clock::PROFPROCS.fetch_add(1, Ordering::Relaxed);
        addupc_task(p, 0x1000, 1);
        assert_eq!(p.process().ps_flags.load(Ordering::Relaxed) & PS_PROFIL, 0);
    }

    #[test]
    fn exec_forgets_the_buffer() {
        let p = profiled(0x4000, 16, 0x1000, 0x10000);
        let pr = p.process();
        pr.ps_prof.pr_buf.set(0x3fd8);
        prof_fork(pr);
        prof_exec(pr);
        assert_eq!(pr.ps_prof.pr_buf.get(), 0);
        assert!(pr.ps_prof.pr_cdir.get().is_none());
        assert!(pr.ps_prof.pr_ucred.get().is_none());
        // Not profiling: nothing to write, nothing touched.
        prof_write(p);
    }

    #[test]
    fn an_exiting_process_writes_its_profile() {
        let (_g, p) = crate::tmpfs::tmpfs_vnops::tests::setup_root();
        let pr = p.process();
        pr.set_comm(b"prog");
        pr.ps_pid.set(77);
        // No resource limits (the file's write checks RLIMIT_FSIZE).
        let limit: &'static crate::sys::resourcevar::Plimit =
            Box::leak(Box::new(crate::sys::resourcevar::Plimit::new()));
        for l in &limit.pl_rlimit {
            l.set(crate::sys::resource::Rlimit {
                rlim_cur: crate::sys::resource::RLIM_INFINITY,
                rlim_max: crate::sys::resource::RLIM_INFINITY,
            });
        }
        pr.ps_limit.set(limit);
        p.p_limit.set(limit);

        // The buffer a profiled program hands over: the header, 8 bytes of counters, room
        // for arcs, of which it has used 16 bytes by the time it exits.
        let mut buf = [0u8; 96];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = i as u8;
        }
        let at = offset_of!(Gmonhdr, totarc);
        buf[at..at + 4].copy_from_slice(&16i32.to_ne_bytes());
        let base = buf.as_mut_ptr() as usize;
        let args = |scale: u32, len: usize| -> SysArgs {
            [
                base as Register,
                len as Register,
                8,
                0x1000,
                scale as Register,
                -1,
            ]
        };
        let mut retval = [0; 2];

        // Only a binary linked for profiling; no scale above 1.0.
        assert_eq!(
            sys_profil(p, &args(0x10000, 96), &mut retval),
            Err(Errno::EPERM)
        );
        pr.ps_iflags.set(PSI_PROFILE);
        assert_eq!(
            sys_profil(p, &args(0x10001, 96), &mut retval),
            Err(Errno::EINVAL)
        );
        sys_profil(p, &args(0x10000, 96), &mut retval).unwrap();
        assert_ne!(pr.ps_flags.load(Ordering::Relaxed) & PS_PROFIL, 0);
        let upp = &pr.ps_prof;
        assert_eq!(upp.pr_base.get(), base + size_of::<Gmonhdr>());
        assert_eq!((upp.pr_size.get(), upp.pr_off.get()), (8, 0x1000));
        assert!(upp.pr_cdir.get().is_some() && upp.pr_ucred.get().is_some());
        // The buffer cannot change; scale 0 stops and a later call starts again.
        assert_eq!(
            sys_profil(p, &args(0x10000, 95), &mut retval),
            Err(Errno::EALREADY)
        );
        sys_profil(p, &args(0, 96), &mut retval).unwrap();
        assert_eq!(pr.ps_flags.load(Ordering::Relaxed) & PS_PROFIL, 0);
        sys_profil(p, &args(0x8000, 96), &mut retval).unwrap();

        // A tick at 0x1008 with scale 0.5 is the counter at offset 4.
        addupc_task(p, 0x1008, 2);
        // SAFETY: only the kernel wrote the array since it was filled, through `base`.
        let mut seen = unsafe { core::ptr::read_volatile(&raw const buf) };
        let c = size_of::<Gmonhdr>() + 4;
        assert_eq!(
            u16::from_ne_bytes([seen[c], seen[c + 1]]),
            u16::from_ne_bytes([c as u8, c as u8 + 1]) + 2
        );

        // exit1: stopprofclock, then prof_write. A program that recorded no arcs (totarc 0)
        // leaves no file; the references of the first profil(2) call are given back.
        // (The host's tmpfs cannot take file data, so the dump itself is not run here.)
        buf[at..at + 4].copy_from_slice(&0i32.to_ne_bytes());
        let cred = upp.pr_ucred.get().unwrap();
        let refs = cred.cr_refcnt.r_refs.load(Ordering::Relaxed);
        stopprofclock(pr);
        prof_write(p);
        assert_eq!(
            crate::tmpfs::tmpfs_vnops::tests::stat(p, b"/gmon.prog.77.out").err(),
            Some(Errno::ENOENT)
        );
        assert_eq!(cred.cr_refcnt.r_refs.load(Ordering::Relaxed), refs - 1);
        // SAFETY: as above.
        seen = unsafe { core::ptr::read_volatile(&raw const buf) };
        assert_eq!(seen[at..at + 4], [0; 4]);
    }
}
/* </TESTS> */
