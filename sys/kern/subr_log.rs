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
 *	@(#)subr_log.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! Error log buffer for kernel printf's: `kern/subr_log.c`.
//!
//! Upstream: sys/kern/subr_log.c @ 3ce1f3f79392
//!
//! Status: `ported`. Milestone M2 ports the message buffer itself: `initmsgbuf`,
//! `msgbuf_putchar`, `msgbuf_putchar_locked`, `logwakeup`, `initconsbuf` and the globals
//! `log_open`, `msgbufmapped`, `msgbufp`, `consbufp`. M8: `sendsyslog(2)`: the log stash
//! (`logstash_full`, `logstash_increment`, `logstash_insert`, `logstash_remove`,
//! `logstash_sendsyslog`), `sys_sendsyslog` and `dosendsyslog`, with `syslogf` and its
//! rwlock. M11e: `log_mtx`, taken by `msgbuf_putchar` as in C. TD1: the `/dev/klog` device
//! (`logopen`, `logclose`, `logread`, `logkqfilter` with its filter, `logtick`, `logioctl`
//! with `LIOCSFD`, `msgbuf_getlen`), character major 7 on both machines, and
//! `initconsbuf`'s buffer.
//!
//! ## Deviations
//! - `msgbufp` is a [`StaticCell`] (a `&'static Msgbuf` is a fat pointer, which no atomic holds).
//! - [`init_static_msgbuf`] is ours: until `pmap` (M3) reserves physical pages that survive a
//!   warm reboot, the message buffer is a static area in `.bss`, and both architectures hand it
//!   to `initmsgbuf` from their early init.
//! - `initmsgbuf`'s check and repair of the header is [`initmsgbuf_header`], which works on a
//!   given overlay without installing it as `msgbufp`, so the host tests exercise it on a
//!   private buffer: tests run in parallel and print through the global one.
//! - `logsoftc` is five statics (`LOGSOFTC_*`); `sc_state` is an atomic changed under
//!   `log_mtx`, `sc_need_wakeup` an atomic flag set with no lock, as in C.
//! - `logread` copies each piece of the ring into a buffer on its stack under `log_mtx`
//!   and hands that to `uiomove` with the mutex released, at most 256 bytes a turn; the C
//!   gives `uiomove` the ring itself with the mutex released. What the reader gets is the
//!   same; a byte the kernel overwrites meanwhile is never read half-written.
//! - An ioctl's `int` is read from and written to the first four bytes of `data`.
//! - Until syslogd(8) sets `syslogf` (`logioctl(LIOCSFD)`), `dosendsyslog` writes a
//!   `LOG_CONS` message to the console, through `cnwrite` when `constty` or `cn_devvp` is
//!   set and through `cnputc` otherwise, and answers `ENOTCONN`; the other messages are
//!   stashed. Once a socket is
//!   set, the message goes to it through `sosend`. The stash's cursors are indices into the
//!   ring, not pointers.

use core::cell::Cell;
use core::ffi::c_void;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering, fence};

use libkern::StaticCell;

use crate::dev::cons::{cn_devvp, cnputc, cnwrite, constty};
use crate::kassert;
use crate::kern::kern_event::{
    klist_free, klist_init_mutex, klist_insert, klist_invalidate, klist_remove, knote,
};
use crate::kern::kern_lock::{mtx_enter, mtx_leave};
use crate::kern::kern_malloc::{free, malloc};
use crate::kern::kern_prot::suser;
use crate::kern::kern_rwlock::{
    rw_assert_anylock, rw_assert_wrlock, rw_enter_read, rw_enter_write, rw_exit,
};
use crate::kern::kern_sig::{pgsigio, sigio_free, sigio_getown, sigio_setown};
use crate::kern::kern_subr::uiomove;
use crate::kern::kern_synch::{sleep_finish, sleep_setup, wakeup};
use crate::kern::kern_timeout::{timeout_add_msec, timeout_del, timeout_set};
use crate::kern::subr_prf::snprintf;
use crate::kern::sys_socket::fp_socket;
use crate::kern::uipc_socket::sosend;
use crate::kern::uipc_syscalls::getsock;
use crate::machine::copy::copyin;
use crate::machine::intr::IPL_HIGH;
use crate::machine::{Machine, MachineParam};
use crate::sys::errno::Errno;
use crate::sys::event::{
    EVFILT_READ, FILTEROP_ISFD, FILTEROP_MPSAFE, Filterops, Kevent, Klist, Knote, knote_modify,
    knote_process,
};
use crate::sys::fcntl::FNONBLOCK;
use crate::sys::file::{File, fref, frele};
use crate::sys::filio::{FIOASYNC, FIOGETOWN, FIONREAD, FIOSETOWN};
use crate::sys::malloc::{M_LOG, M_TTYS, M_WAITOK, M_ZERO};
use crate::sys::msgbuf::{CONSBUFSIZE, MSG_MAGIC, Msgbuf};
use crate::sys::mutex::Mutex;
use crate::sys::param::{PCATCH, PZERO};
use crate::sys::proc::Proc;
use crate::sys::rwlock::Rwlock;
use crate::sys::sigio::{SigioRef, sigio_init};
use crate::sys::signal::SIGIO;
use crate::sys::socket::MSG_DONTWAIT;
use crate::sys::syscallargs::SysSendsyslogArgs;
use crate::sys::syslog::{LIOCSFD, LOG_CONS, LOG_KERN, LOG_MAXLINE, LOG_WARNING};
use crate::sys::systm::{INFSLP, SysArgs, kernel_lock, kernel_unlock, sysargs};
use crate::sys::timeout::Timeout;
use crate::sys::ttycom::{TIOCGPGRP, TIOCSPGRP};
use crate::sys::types::{Dev, Pid, Register};
use crate::sys::uio::{Iovec, Uio, UioRw, UioSeg};
use crate::sys::vnode::IO_NDELAY;

/// `MSGBUFSIZE` of the selected machine: the static buffer's size.
const MSGBUFSIZE: usize = <Machine as MachineParam>::MSGBUFSIZE;

/// The static message buffer area (see the module's deviations).
#[repr(C, align(8))]
struct MsgbufArea([u8; MSGBUFSIZE]);

/// `log_open`: is `/dev/klog` open? Also used in `log()`.
pub static LOG_OPEN: AtomicBool = AtomicBool::new(false);
/// `msgbufmapped`: is the message buffer mapped?
static MSGBUFMAPPED: AtomicBool = AtomicBool::new(false);
/// `msgbufp`: the mapped buffer, itself.
static MSGBUFP: StaticCell<Option<&'static Msgbuf>> = StaticCell::new(None);
/// `consbufp`: console message buffer.
static CONSBUFP: StaticCell<Option<&'static Msgbuf>> = StaticCell::new(None);
/// `LOG_RDPRI`: the priority a reader of the log device sleeps at.
const LOG_RDPRI: i32 = PZERO + 1;
/// `LOG_TICK`: log tick interval in msec.
const LOG_TICK: u64 = 50;
/// `LOG_ASYNC`: `sc_state`: the owner wants `SIGIO`.
const LOG_ASYNC: i32 = 0x04;
/// `LOG_RDWAIT`: `sc_state`: a reader sleeps for data.
const LOG_RDWAIT: i32 = 0x08;

/// `logread_filtops`: `EVFILT_READ` on the log device.
pub static LOGREAD_FILTOPS: Filterops = Filterops {
    f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
    f_attach: None,
    f_detach: Some(filt_logrdetach),
    f_event: Some(filt_logread),
    f_modify: Some(filt_logmodify),
    f_process: Some(filt_logprocess),
};

/// `logsoftc.sc_state`: \[L\] `LOG_ASYNC` and `LOG_RDWAIT`, changed under `log_mtx`.
static LOGSOFTC_STATE: AtomicI32 = AtomicI32::new(0);
/// `logsoftc.sc_klist`: \[Q\] process waiting on kevent call.
static LOGSOFTC_KLIST: Klist = Klist::new();
/// `logsoftc.sc_sigio`: async I/O registration.
static LOGSOFTC_SIGIO: SigioRef = SigioRef::new();
/// `logsoftc.sc_tick`: wakeup poll timeout.
static LOGSOFTC_TICK: Timeout = Timeout::new(logtick, ptr::null_mut());
/// `log_kq_mtx`: the lock of `sc_klist` (`MTX_NOWITNESS` in C).
static LOG_KQ_MTX: Mutex = Mutex::new(IPL_HIGH);
/// `logsoftc.sc_need_wakeup`: if set, wake up waiters.
static LOGSOFTC_NEED_WAKEUP: AtomicBool = AtomicBool::new(false);
/// `log_mtx`: serializes access to the log message buffers. This should be kept as a leaf
/// lock in order not to constrain where printf(9) can be used (`MTX_NOWITNESS` in C).
pub static LOG_MTX: Mutex = Mutex::new(IPL_HIGH);
/// The buffer `init_static_msgbuf` overlays.
static MSGBUF_AREA: StaticCell<MsgbufArea> = StaticCell::new(MsgbufArea([0; MSGBUFSIZE]));

/// `msgbufmapped`: whether `initmsgbuf` has run.
pub fn msgbufmapped() -> bool {
    MSGBUFMAPPED.load(Ordering::Acquire)
}

/// `msgbufp`: the message buffer, once mapped.
pub fn msgbufp() -> Option<&'static Msgbuf> {
    // SAFETY: written once by `initmsgbuf`, on the boot CPU before anything prints; only read
    // afterwards.
    unsafe { MSGBUFP.read() }
}

/// `consbufp`: the console buffer, once `initconsbuf` has run.
pub fn consbufp() -> Option<&'static Msgbuf> {
    // SAFETY: as for `msgbufp`.
    unsafe { CONSBUFP.read() }
}

/// `initmsgbuf`: lays the message buffer over `bufsize` bytes at `buf`. A header left by a
/// previous boot (right magic, same size, consistent pointers) is kept, so `dmesg(8)` can show
/// what happened before a reboot; otherwise the area is cleared and initialised. New output
/// always starts on a fresh line.
///
/// # Safety
///
/// As for [`Msgbuf::from_raw`]: `buf` is 8-byte aligned, valid for `bufsize` bytes for the rest
/// of the kernel's life, and used through nothing but the message buffer.
pub unsafe fn initmsgbuf(buf: *mut u8, bufsize: usize) {
    // Sanity-check the given size.
    if bufsize < Msgbuf::MIN_SIZE {
        return;
    }

    // SAFETY: forwarded from the caller.
    let mbp = unsafe { Msgbuf::from_raw(buf, bufsize) };
    // SAFETY: single writer, on the boot CPU, before any reader (see `msgbufp`).
    unsafe { MSGBUFP.write(Some(mbp)) };

    initmsgbuf_header(mbp, bufsize);

    // mark it as ready for use.
    MSGBUFMAPPED.store(true, Ordering::Release);
}

/// The header half of [`initmsgbuf`]: keeps a sane header left by a previous boot, clears and
/// initialises any other, and starts new output on a fresh line (see the module's
/// deviations).
fn initmsgbuf_header(mbp: &Msgbuf, bufsize: usize) {
    let new_bufs = (bufsize - Msgbuf::HEADER_SIZE) as i64;
    if mbp.magic() != MSG_MAGIC
        || mbp.bufs() != new_bufs
        || mbp.bufr() < 0
        || mbp.bufr() >= mbp.bufs()
        || mbp.bufx() < 0
        || mbp.bufx() >= mbp.bufs()
    {
        // If the buffer magic number is wrong, has changed size (which shouldn't happen
        // often), or is internally inconsistent, initialize it.
        mbp.clear();
        mbp.set_magic(MSG_MAGIC);
        mbp.set_bufs(new_bufs);
    }

    // Always start new buffer data on a new line. Avoid using log_mtx because mutexes do not
    // work during early boot on some architectures.
    if mbp.bufx() > 0 && mbp.bufc()[(mbp.bufx() - 1) as usize].get() != b'\n' {
        msgbuf_putchar_locked(mbp, b'\n');
    }
}

/// Hands the static buffer area to [`initmsgbuf`] (see the module's deviations).
pub fn init_static_msgbuf() {
    // SAFETY: the area is 8-byte aligned, `MSGBUFSIZE` bytes long, lives forever and is only
    // ever reached through the overlay `initmsgbuf` installs.
    unsafe { initmsgbuf(MSGBUF_AREA.as_ptr().cast::<u8>(), MSGBUFSIZE) }
}

/// `initconsbuf`: sets up a buffer to collect `/dev/console` output.
pub fn initconsbuf() {
    // Set up a buffer to collect /dev/console output
    let Some(mem) = malloc(CONSBUFSIZE, M_TTYS, M_WAITOK | M_ZERO) else {
        // malloc(M_WAITOK) does not fail in C.
        return;
    };
    // SAFETY: a fresh malloc(9) block of `CONSBUFSIZE` zeroed bytes, aligned for the header,
    // never freed and reached only through this overlay.
    let cbp = unsafe { Msgbuf::from_raw(mem.as_ptr(), CONSBUFSIZE) };
    cbp.set_magic(MSG_MAGIC);
    cbp.set_bufs((CONSBUFSIZE - Msgbuf::HEADER_SIZE) as i64);
    // SAFETY: single writer, on the boot CPU (see `consbufp`).
    unsafe { CONSBUFP.write(Some(cbp)) };
}

/// `msgbuf_putchar`: appends `c` to `mbp` under `log_mtx`; nothing happens if the buffer was
/// never initialised.
pub fn msgbuf_putchar(mbp: &Msgbuf, c: u8) {
    if mbp.magic() != MSG_MAGIC {
        // Nothing we can do
        return;
    }
    mtx_enter(&LOG_MTX);
    msgbuf_putchar_locked(mbp, c);
    mtx_leave(&LOG_MTX);
}

/// `msgbuf_putchar_locked`: appends `c` to the ring; when it is full the oldest byte is
/// dropped and counted in `msg_bufd`.
pub fn msgbuf_putchar_locked(mbp: &Msgbuf, c: u8) {
    let bufc = mbp.bufc();
    let mut x = mbp.bufx();
    bufc[x as usize].set(c);
    x += 1;
    if x < 0 || x >= mbp.bufs() {
        x = 0;
    }
    mbp.set_bufx(x);
    // If the buffer is full, keep the most recent data.
    if mbp.bufr() == x {
        let mut r = mbp.bufr() + 1;
        if r >= mbp.bufs() {
            r = 0;
        }
        mbp.set_bufr(r);
        mbp.set_bufd(mbp.bufd() + 1);
    }
}

/// `msgbuf_getlen`: how many bytes of the buffer have not been read yet.
pub fn msgbuf_getlen(mbp: &Msgbuf) -> usize {
    let mut len = mbp.bufx() - mbp.bufr();
    if len < 0 {
        len += mbp.bufs();
    }
    len as usize
}

/// `logopen`: `/dev/klog` is opened by one reader at a time, syslogd(8); the opening starts
/// the tick that turns `logwakeup`'s requests into wakeups.
pub fn logopen(_dev: Dev, _flags: i32, _mode: i32, _p: &Proc) -> Result<(), Errno> {
    if LOG_OPEN.load(Ordering::Relaxed) {
        return Err(Errno::EBUSY);
    }
    LOG_OPEN.store(true, Ordering::Relaxed);
    // SAFETY: `log_kq_mtx` is a static.
    unsafe { klist_init_mutex(&LOGSOFTC_KLIST, &LOG_KQ_MTX) };
    sigio_init(&LOGSOFTC_SIGIO);
    timeout_set(&LOGSOFTC_TICK, logtick, ptr::null_mut());
    timeout_add_msec(&LOGSOFTC_TICK, LOG_TICK);
    Ok(())
}

/// `logclose`: gives the syslog socket back, stops the tick and frees the device's state.
pub fn logclose(_dev: Dev, _flag: i32, _mode: i32, p: Option<&Proc>) -> Result<(), Errno> {
    rw_enter_write(&SYSLOGF_RWLOCK);
    let fp = SYSLOGF.swap(ptr::null_mut(), Ordering::AcqRel);
    rw_exit(&SYSLOGF_RWLOCK);

    // SAFETY: a non-null `syslogf` is a file the log device held a reference on.
    if let Some(fp) = unsafe { fp.as_ref() } {
        let _ = frele(fp, p);
    }
    LOG_OPEN.store(false, Ordering::Relaxed);
    timeout_del(&LOGSOFTC_TICK);

    klist_invalidate(&LOGSOFTC_KLIST);
    klist_free(&LOGSOFTC_KLIST);

    LOGSOFTC_STATE.store(0, Ordering::Relaxed);
    sigio_free(&LOGSOFTC_SIGIO);
    Ok(())
}

/// `logread`: hands the reader the unread bytes of the message buffer, after a line that
/// says how many were dropped when the buffer overflowed; with nothing to read it sleeps,
/// or fails with `EWOULDBLOCK` under `IO_NDELAY`.
pub fn logread(_dev: Dev, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    let Some(mbp) = msgbufp() else {
        // The C dereferences msgbufp, which initmsgbuf set long before the device exists.
        return Err(Errno::ENXIO);
    };
    logread_msgbuf(mbp, uio, flag)
}

/// `logread`'s body over a given buffer (`msgbufp` in the kernel, a private one in the host
/// tests, which run in parallel and print through the global one).
fn logread_msgbuf(mbp: &Msgbuf, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    let mut error = Ok(());

    mtx_enter(&LOG_MTX);
    'out: {
        while mbp.bufr() == mbp.bufx() {
            if flag & IO_NDELAY != 0 {
                error = Err(Errno::EWOULDBLOCK);
                break 'out;
            }
            LOGSOFTC_STATE.fetch_or(LOG_RDWAIT, Ordering::Relaxed);
            mtx_leave(&LOG_MTX);
            // Set up and enter sleep manually instead of using msleep() to keep log_mtx as
            // a leaf lock.
            sleep_setup(ptr::from_ref(mbp).cast(), LOG_RDPRI | PCATCH, "klog");
            error = sleep_finish(
                INFSLP,
                LOGSOFTC_STATE.load(Ordering::Relaxed) & LOG_RDWAIT != 0,
            );
            mtx_enter(&LOG_MTX);
            if error.is_err() {
                break 'out;
            }
        }

        if mbp.bufd() > 0 {
            let mut buf = [0u8; 64];

            let ndropped = mbp.bufd();
            mtx_leave(&LOG_MTX);
            let l = snprintf(
                &mut buf,
                format_args!(
                    "<{}>klog: dropped {} byte{}, message buffer full\n",
                    LOG_KERN | LOG_WARNING,
                    ndropped,
                    if ndropped == 1 { "" } else { "s" }
                ),
            );
            let l = l.min(buf.len() - 1);
            error = uiomove(&mut buf[..l], uio);
            mtx_enter(&LOG_MTX);
            if error.is_err() {
                break 'out;
            }
            mbp.set_bufd(mbp.bufd() - ndropped);
        }

        while uio.uio_resid > 0 {
            let mut chunk = [0u8; 256];

            let l = if mbp.bufx() >= mbp.bufr() {
                mbp.bufx() - mbp.bufr()
            } else {
                mbp.bufs() - mbp.bufr()
            } as usize;
            let l = l.min(uio.uio_resid).min(chunk.len());
            if l == 0 {
                break;
            }
            let rpos = mbp.bufr() as usize;
            // The bytes are copied out of the ring under log_mtx (see the module's
            // deviations).
            for (d, s) in chunk[..l].iter_mut().zip(&mbp.bufc()[rpos..rpos + l]) {
                *d = s.get();
            }
            mtx_leave(&LOG_MTX);
            // Ignore that concurrent readers may consume the same data.
            error = uiomove(&mut chunk[..l], uio);
            mtx_enter(&LOG_MTX);
            if error.is_err() {
                break;
            }
            let mut r = mbp.bufr() + l as i64;
            if r < 0 || r >= mbp.bufs() {
                r = 0;
            }
            mbp.set_bufr(r);
        }
    }
    mtx_leave(&LOG_MTX);
    error
}

/// `logkqfilter`: `EVFILT_READ` on the log device.
pub fn logkqfilter(_dev: Dev, kn: &Knote) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&LOGREAD_FILTOPS)),
        _ => return Err(Errno::EINVAL),
    }

    kn.kn_hook
        .set(msgbufp().map_or(ptr::null_mut(), |mbp| ptr::from_ref(mbp).cast_mut().cast()));
    klist_insert(&LOGSOFTC_KLIST, kn);

    Ok(())
}

/// `filt_logrdetach`.
pub fn filt_logrdetach(kn: &Knote) {
    klist_remove(&LOGSOFTC_KLIST, kn);
}

/// `filt_logread`: readable when the buffer holds unread bytes; their number is the data.
pub fn filt_logread(kn: &Knote, _hint: i64) -> bool {
    // kn->kn_hook is msgbufp.
    let Some(mbp) = msgbufp() else {
        return false;
    };

    mtx_enter(&LOG_MTX);
    kn.kn_data().set(msgbuf_getlen(mbp) as i64);
    mtx_leave(&LOG_MTX);
    kn.kn_data().get() != 0
}

/// `filt_logmodify`.
pub fn filt_logmodify(kev: &mut Kevent, kn: &Knote) -> bool {
    mtx_enter(&LOG_KQ_MTX);
    let active = knote_modify(kev, kn);
    mtx_leave(&LOG_KQ_MTX);

    active
}

/// `filt_logprocess`.
pub fn filt_logprocess(kn: &Knote, kev: Option<&mut Kevent>) -> bool {
    mtx_enter(&LOG_KQ_MTX);
    let active = knote_process(kn, kev);
    mtx_leave(&LOG_KQ_MTX);

    active
}

/// `logwakeup`: asks for the `/dev/klog` readers to be woken. The actual wakeup has to be
/// deferred because `logwakeup()` can be called in very varied contexts; keeping the print
/// routines usable in as many situations as possible means no locking here.
pub fn logwakeup() {
    // Ensure that preceding stores become visible to other CPUs before the flag
    // (membar_producer).
    fence(Ordering::Release);
    LOGSOFTC_NEED_WAKEUP.store(true, Ordering::Relaxed);
}

/// `logtick`: every `LOG_TICK` milliseconds while the device is open, makes the wakeups
/// `logwakeup` asked for: the knotes, `SIGIO` in async mode, the sleeping reader.
pub fn logtick(_arg: *mut c_void) {
    if !LOG_OPEN.load(Ordering::Relaxed) {
        return;
    }

    if LOGSOFTC_NEED_WAKEUP.load(Ordering::Relaxed) {
        LOGSOFTC_NEED_WAKEUP.store(false, Ordering::Relaxed);

        // sc_need_wakeup has to be cleared before handling the wakeup. Visiting log_mtx
        // ensures the proper order.

        mtx_enter(&LOG_MTX);
        let state = LOGSOFTC_STATE.load(Ordering::Relaxed);
        if state & LOG_RDWAIT != 0 {
            LOGSOFTC_STATE.store(state & !LOG_RDWAIT, Ordering::Relaxed);
        }
        mtx_leave(&LOG_MTX);

        knote(&LOGSOFTC_KLIST, 0);

        if state & LOG_ASYNC != 0 {
            pgsigio(&LOGSOFTC_SIGIO, SIGIO, false);
        }
        if state & LOG_RDWAIT != 0
            && let Some(mbp) = msgbufp()
        {
            wakeup(ptr::from_ref(mbp));
        }
    }

    timeout_add_msec(&LOGSOFTC_TICK, LOG_TICK);
}

/// The `int` an ioctl carries in `data`.
fn ioctl_int(data: &[u8]) -> i32 {
    data.first_chunk::<4>()
        .map_or(0, |b| i32::from_ne_bytes(*b))
}

/// `logioctl`: `FIONREAD` (the unread bytes), `FIOASYNC`, the owner of `SIGIO`
/// (`FIOSETOWN`/`TIOCSPGRP`, `FIOGETOWN`/`TIOCGPGRP`) and `LIOCSFD`, by which syslogd(8)
/// hands the kernel the socket `sendsyslog(2)` writes to.
pub fn logioctl(_dev: Dev, com: u64, data: &mut [u8], _flag: i32, p: &Proc) -> Result<(), Errno> {
    match com {
        // return number of characters immediately available
        FIONREAD => {
            mtx_enter(&LOG_MTX);
            let len = msgbufp().map_or(0, msgbuf_getlen) as i32;
            mtx_leave(&LOG_MTX);
            if let Some(b) = data.first_chunk_mut::<4>() {
                *b = len.to_ne_bytes();
            }
        }

        FIOASYNC => {
            mtx_enter(&LOG_MTX);
            if ioctl_int(data) != 0 {
                LOGSOFTC_STATE.fetch_or(LOG_ASYNC, Ordering::Relaxed);
            } else {
                LOGSOFTC_STATE.fetch_and(!LOG_ASYNC, Ordering::Relaxed);
            }
            mtx_leave(&LOG_MTX);
        }

        FIOSETOWN | TIOCSPGRP => {
            return sigio_setown(&LOGSOFTC_SIGIO, com, &ioctl_int(data));
        }

        FIOGETOWN | TIOCGPGRP => {
            let mut owner = 0;
            sigio_getown(&LOGSOFTC_SIGIO, com, &mut owner);
            if let Some(b) = data.first_chunk_mut::<4>() {
                *b = owner.to_ne_bytes();
            }
        }

        LIOCSFD => {
            suser(p)?;
            let newfp = getsock(p, ioctl_int(data))?;

            rw_enter_write(&SYSLOGF_RWLOCK);
            let fp = SYSLOGF.swap(ptr::from_ref(newfp).cast_mut(), Ordering::AcqRel);
            rw_exit(&SYSLOGF_RWLOCK);

            // SAFETY: a non-null `syslogf` is a file the log device held a reference on.
            if let Some(fp) = unsafe { fp.as_ref() } {
                let _ = frele(fp, p);
            }
        }

        _ => return Err(Errno::ENOTTY),
    }
    Ok(())
}

/// `LOGSTASH_SIZE`: how many messages the log stash keeps while `syslogd(8)` is away.
const LOGSTASH_SIZE: usize = 100;

/// `struct logstash_message`: one stashed message (`malloc(M_LOG)` bytes, or none).
struct LogstashMessage {
    /// `lgs_buffer`.
    lgs_buffer: Cell<Option<NonNull<u8>>>,
    /// `lgs_size`.
    lgs_size: Cell<usize>,
}

/// The log stash: `logstash_messages[]`, the `logstash_in`/`logstash_out` cursors (indices
/// into the ring) and the drop bookkeeping (`logstash_dropped`, `logstash_error`,
/// `logstash_pid`). Protected by: `logstash_rwlock`.
struct Logstash {
    /// `logstash_messages`.
    messages: [LogstashMessage; LOGSTASH_SIZE],
    /// `logstash_in`.
    r#in: Cell<usize>,
    /// `logstash_out`.
    out: Cell<usize>,
    /// `logstash_dropped`.
    dropped: Cell<i32>,
    /// `logstash_error`.
    error: Cell<i32>,
    /// `logstash_pid`.
    pid: Cell<Pid>,
}

// SAFETY: every member is touched only under `logstash_rwlock` (`LOGSTASH_RWLOCK`) held for
// writing, as in C.
unsafe impl Sync for Logstash {}

/// The log stash, empty.
static LOGSTASH: Logstash = Logstash {
    messages: [const {
        LogstashMessage {
            lgs_buffer: Cell::new(None),
            lgs_size: Cell::new(0),
        }
    }; LOGSTASH_SIZE],
    r#in: Cell::new(0),
    out: Cell::new(0),
    dropped: Cell::new(0),
    error: Cell::new(0),
    pid: Cell::new(0),
};

/// `logstash_rwlock`.
static LOGSTASH_RWLOCK: Rwlock = Rwlock::new("logstash");

/// `syslogf`: the socket `syslogd(8)` hands the log device (`LIOCSFD`); `None` while there
/// is none, which is always until the log device and sockets are ported.
static SYSLOGF: AtomicPtr<File> = AtomicPtr::new(ptr::null_mut());
/// `syslogf_rwlock`.
static SYSLOGF_RWLOCK: Rwlock = Rwlock::new("syslogf");

/// `logstash_full`.
fn logstash_full() -> bool {
    rw_assert_anylock(&LOGSTASH_RWLOCK);

    LOGSTASH.messages[LOGSTASH.out.get()]
        .lgs_buffer
        .get()
        .is_some()
        && LOGSTASH.r#in.get() == LOGSTASH.out.get()
}

/// `logstash_increment`: advances a cursor around the ring.
fn logstash_increment(cursor: &Cell<usize>) {
    rw_assert_wrlock(&LOGSTASH_RWLOCK);

    kassert!(cursor.get() < LOGSTASH_SIZE);
    if cursor.get() == LOGSTASH_SIZE - 1 {
        cursor.set(0);
    } else {
        cursor.set(cursor.get() + 1);
    }
}

/// `logstash_insert`: keeps a copy of the user message `buf` for later, or counts it as
/// dropped when the stash is full.
pub fn logstash_insert(buf: usize, nbyte: usize, logerror: Errno, pid: Pid) -> Result<(), Errno> {
    rw_enter_write(&LOGSTASH_RWLOCK);

    if logstash_full() {
        if LOGSTASH.dropped.get() == 0 {
            LOGSTASH.error.set(logerror as i32);
            LOGSTASH.pid.set(pid);
        }
        LOGSTASH.dropped.set(LOGSTASH.dropped.get() + 1);

        rw_exit(&LOGSTASH_RWLOCK);
        return Ok(());
    }

    let slot = &LOGSTASH.messages[LOGSTASH.r#in.get()];
    let Some(mem) = malloc(nbyte.max(1), M_LOG, M_WAITOK) else {
        rw_exit(&LOGSTASH_RWLOCK);
        return Err(Errno::ENOMEM);
    };
    // SAFETY: a fresh `nbyte`-byte allocation, owned by the stash from here.
    let kbuf = unsafe { core::slice::from_raw_parts_mut(mem.as_ptr(), nbyte) };
    if let Err(error) = copyin(buf, kbuf) {
        free(mem, M_LOG, nbyte.max(1));
        slot.lgs_buffer.set(None);

        rw_exit(&LOGSTASH_RWLOCK);
        return Err(error);
    }
    slot.lgs_buffer.set(Some(mem));
    slot.lgs_size.set(nbyte);
    logstash_increment(&LOGSTASH.r#in);

    rw_exit(&LOGSTASH_RWLOCK);
    Ok(())
}

/// `logstash_remove`: frees the oldest stashed message and, if messages were dropped,
/// stashes a note saying how many in their place in the sequence.
pub fn logstash_remove() {
    rw_assert_wrlock(&LOGSTASH_RWLOCK);

    let out = &LOGSTASH.messages[LOGSTASH.out.get()];
    kassert!(out.lgs_buffer.get().is_some());
    if let Some(buf) = out.lgs_buffer.take() {
        free(buf, M_LOG, out.lgs_size.get().max(1));
    }
    logstash_increment(&LOGSTASH.out);

    // Insert dropped message in sequence where messages were dropped.
    let dropped = LOGSTASH.dropped.get();
    if dropped != 0 {
        let mut buf = [0u8; 80];
        let l = snprintf(
            &mut buf,
            format_args!(
                "<{}>sendsyslog: dropped {} message{}, error {}, pid {}",
                LOG_KERN | LOG_WARNING,
                dropped,
                if dropped == 1 { "" } else { "s" },
                LOGSTASH.error.get(),
                LOGSTASH.pid.get()
            ),
        );
        LOGSTASH.dropped.set(0);
        LOGSTASH.error.set(0);
        LOGSTASH.pid.set(0);

        // Cannot fail, we have just freed a slot.
        kassert!(!logstash_full());
        let nbyte = l.min(buf.len() - 1);
        let slot = &LOGSTASH.messages[LOGSTASH.r#in.get()];
        if let Some(mem) = malloc(nbyte.max(1), M_LOG, M_WAITOK) {
            // SAFETY: a fresh `nbyte`-byte allocation, owned by the stash from here.
            unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), mem.as_ptr(), nbyte) };
            slot.lgs_buffer.set(Some(mem));
            slot.lgs_size.set(nbyte);
            logstash_increment(&LOGSTASH.r#in);
        }
    }
}

/// `logstash_sendsyslog`: sends the stashed messages, oldest first, until one fails.
pub fn logstash_sendsyslog(p: &Proc) -> Result<(), Errno> {
    rw_enter_write(&LOGSTASH_RWLOCK);

    loop {
        let out = &LOGSTASH.messages[LOGSTASH.out.get()];
        let Some(buf) = out.lgs_buffer.get() else {
            break;
        };
        let error = dosendsyslog(p, SyslogBuf::Sys(buf.as_ptr()), out.lgs_size.get(), 0);
        if let Err(error) = error {
            rw_exit(&LOGSTASH_RWLOCK);
            return Err(error);
        }
        logstash_remove();
    }

    rw_exit(&LOGSTASH_RWLOCK);
    Ok(())
}

/// Send syslog(3) message from userland to socketpair(2) created by syslogd(8). Store
/// message in kernel log stash for later if syslogd(8) is not available or sending fails.
/// Send to console if `LOG_CONS` is set and syslogd(8) socket does not exist.
pub fn sys_sendsyslog(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSendsyslogArgs = sysargs(v);

    let nbyte = uap.nbyte.get().min(LOG_MAXLINE);

    let _ = logstash_sendsyslog(p);
    let buf = uap.buf.get() as usize;
    let error = dosendsyslog(p, SyslogBuf::User(buf), nbyte, uap.flags.get());
    if let Err(e) = error
        && e != Errno::EFAULT
    {
        let _ = logstash_insert(buf, nbyte, e, p.process().ps_pid.get());
    }
    error
}

/// The message `dosendsyslog` sends: a user address (`UIO_USERSPACE`) or a kernel buffer
/// (`UIO_SYSSPACE`, a stashed message).
#[derive(Clone, Copy)]
pub enum SyslogBuf {
    /// A user address.
    User(usize),
    /// A kernel buffer of at least `nbyte` bytes.
    Sys(*const u8),
}

impl SyslogBuf {
    /// Copies `out.len()` bytes from `off` on into `out`.
    fn read(self, off: usize, out: &mut [u8]) -> Result<(), Errno> {
        match self {
            SyslogBuf::User(addr) => copyin(addr + off, out),
            SyslogBuf::Sys(ptr) => {
                // SAFETY: a stashed message holds `lgs_size` bytes, and the callers read
                // within `nbyte` of it.
                unsafe { ptr::copy_nonoverlapping(ptr.add(off), out.as_mut_ptr(), out.len()) };
                Ok(())
            }
        }
    }
}

/// `dosendsyslog`: sends `nbyte` bytes of `buf` to `syslogd(8)`'s socket, or, when there is
/// none and `LOG_CONS` is set, to the console without the `<pri>` prefix. `ENOTCONN` when
/// nothing took the message.
pub fn dosendsyslog(p: &Proc, buf: SyslogBuf, nbyte: usize, flags: i32) -> Result<(), Errno> {
    let mut nbyte = nbyte;
    let mut start = 0usize;

    // Global variable syslogf may change during sleep, use local copy.
    rw_enter_read(&SYSLOGF_RWLOCK);
    // SAFETY: a non-null `syslogf` is a file the log device holds a reference on; it is
    // read under `syslogf_rwlock`.
    let fp: Option<&'static File> = unsafe { SYSLOGF.load(Ordering::Acquire).as_ref() };
    if let Some(fp) = fp {
        fref(fp);
    }
    rw_exit(&SYSLOGF_RWLOCK);

    if fp.is_none() {
        if flags & LOG_CONS == 0 {
            return Err(Errno::ENOTCONN);
        }
        // Strip off syslog priority when logging to console. LOG_PRIMASK | LOG_FACMASK is
        // 0x03ff, so at most 4 decimal digits may appear in priority as <1023>.
        let mut pri = [0u8; 6];
        let len = nbyte.min(pri.len());
        buf.read(0, &mut pri[..len])?;
        if 0 < len && pri[0] == b'<' {
            let mut i = 1;
            while i < len {
                if !pri[i].is_ascii_digit() {
                    break;
                }
                i += 1;
            }
            if i < len && pri[i] == b'>' {
                i += 1;
                // There must be at least one digit <0>.
                if i >= 3 {
                    start = i;
                    nbyte -= i;
                }
            }
        }
    }

    // KTRACE: configured in GENERIC.MP, not ported yet (TD3).
    let mut len = nbyte;
    let (base, segflg) = match buf {
        SyslogBuf::User(addr) => (addr + start, UioSeg::UIO_USERSPACE),
        SyslogBuf::Sys(ptr) => (ptr as usize + start, UioSeg::UIO_SYSSPACE),
    };
    let mut aiov = [Iovec {
        iov_base: ptr::without_provenance_mut(base),
        iov_len: nbyte,
    }];
    let mut auio = Uio {
        uio_iov: &mut aiov,
        uio_offset: 0,
        uio_resid: nbyte,
        uio_segflg: segflg,
        uio_rw: UioRw::UIO_WRITE,
        uio_procp: Some(p),
    };
    let error = match fp {
        Some(fp) => {
            let flags = if fp.flag() & FNONBLOCK != 0 {
                MSG_DONTWAIT
            } else {
                0
            };
            let error = sosend(fp_socket(fp), None, Some(&mut auio), None, None, flags);
            if error.is_ok() {
                len -= auio.uio_resid;
            }
            error
        }
        None if constty().is_some() || cn_devvp().is_some() => {
            kernel_lock(); // KERNEL_LOCK()
            let error = cnwrite(0, &mut auio, 0);
            if error.is_ok() {
                len -= auio.uio_resid;
            }
            let mut crlf = *b"\r\n";
            let mut aiov = [Iovec {
                iov_base: crlf.as_mut_ptr().cast(),
                iov_len: crlf.len(),
            }];
            let mut auio = Uio {
                uio_iov: &mut aiov,
                uio_offset: 0,
                uio_resid: 2,
                uio_segflg: UioSeg::UIO_SYSSPACE,
                uio_rw: UioRw::UIO_WRITE,
                uio_procp: Some(p),
            };
            let _ = cnwrite(0, &mut auio, 0);
            kernel_unlock(); // KERNEL_UNLOCK()
            error
        }
        None => {
            kernel_lock(); // KERNEL_LOCK()
            // XXX console redirection breaks down...
            let mut resid = len;
            let mut error = Ok(());
            let mut kbuf = [0u8; 256];
            let mut off = 0;
            'out: while off < len {
                let chunk = (len - off).min(kbuf.len());
                if let Err(e) = buf.read(start + off, &mut kbuf[..chunk]) {
                    error = Err(e);
                    break;
                }
                for &c in &kbuf[..chunk] {
                    if c == 0 {
                        break 'out;
                    }
                    cnputc(i32::from(c));
                    resid -= 1;
                }
                off += chunk;
            }
            if error.is_ok() {
                len -= resid;
            }
            cnputc(i32::from(b'\n'));
            kernel_unlock(); // KERNEL_UNLOCK()
            error
        }
    };
    let _ = len;

    match fp {
        Some(fp) => {
            let _ = frele(fp, p);
            error
        }
        None => match error {
            Err(Errno::EFAULT) => Err(Errno::EFAULT),
            _ => Err(Errno::ENOTCONN),
        },
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C, align(8))]
    struct Area([u8; 64]);

    #[test]
    fn ring_wraps_and_drops_the_oldest() {
        let mut area = Area([0; 64]);
        // SAFETY: the area outlives the test and is used only through the overlay.
        let mbp = unsafe { Msgbuf::from_raw(area.0.as_mut_ptr(), 64) };
        mbp.clear();
        mbp.set_magic(MSG_MAGIC);
        mbp.set_bufs(24);
        for b in b"0123456789abcdefghijklm" {
            msgbuf_putchar(mbp, *b);
        }
        assert_eq!(mbp.bufx(), 23);
        assert_eq!(mbp.bufr(), 0);
        assert_eq!(mbp.bufd(), 0);
        msgbuf_putchar(mbp, b'n'); // fills the ring: bufx wraps to 0 == bufr
        assert_eq!(mbp.bufx(), 0);
        assert_eq!(mbp.bufr(), 1);
        assert_eq!(mbp.bufd(), 1);
        msgbuf_putchar(mbp, b'o'); // still full: the read pointer keeps running ahead
        assert_eq!(mbp.bufc()[0].get(), b'o');
        assert_eq!(mbp.bufr(), 2);
        assert_eq!(mbp.bufd(), 2);
        // The wrong magic makes the buffer inert.
        mbp.set_magic(0);
        msgbuf_putchar(mbp, b'p');
        assert_eq!(mbp.bufx(), 1);
    }

    /// Reads up to `len` bytes of `mbp` as the log device would, without sleeping.
    fn read(mbp: &Msgbuf, len: usize) -> Result<std::vec::Vec<u8>, Errno> {
        let mut buf = std::vec![0u8; len];
        let mut iov = [Iovec {
            iov_base: buf.as_mut_ptr().cast(),
            iov_len: len,
        }];
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: len,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: None,
        };
        logread_msgbuf(mbp, &mut uio, IO_NDELAY)?;
        let done = len - uio.uio_resid;
        buf.truncate(done);
        Ok(buf)
    }

    #[test]
    fn the_log_device_reads_what_was_not_read_yet() {
        let mut area = Area([0; 64]);
        // SAFETY: the area outlives the test and is used only through the overlay.
        let mbp = unsafe { Msgbuf::from_raw(area.0.as_mut_ptr(), 64) };
        mbp.clear();
        mbp.set_magic(MSG_MAGIC);
        mbp.set_bufs(24);

        // Nothing to read: the device would sleep, or with IO_NDELAY says so.
        assert_eq!(msgbuf_getlen(mbp), 0);
        assert_eq!(read(mbp, 8), Err(Errno::EWOULDBLOCK));

        for b in b"hello, log\n" {
            msgbuf_putchar(mbp, *b);
        }
        assert_eq!(msgbuf_getlen(mbp), 11);
        // A short read takes the oldest bytes; the next one goes on from there.
        assert_eq!(read(mbp, 4).expect("read"), b"hell");
        assert_eq!(msgbuf_getlen(mbp), 7);
        assert_eq!(read(mbp, 100).expect("read"), b"o, log\n");
        assert_eq!(msgbuf_getlen(mbp), 0);
        assert_eq!(read(mbp, 8), Err(Errno::EWOULDBLOCK));

        // Across the end of the ring: the length counts both pieces, and one read returns
        // them in order.
        for b in b"0123456789abcdefgh" {
            msgbuf_putchar(mbp, *b);
        }
        assert_eq!(mbp.bufx(), 5);
        assert_eq!(msgbuf_getlen(mbp), 18);
        assert_eq!(read(mbp, 100).expect("read"), b"0123456789abcdefgh");
        assert_eq!(mbp.bufr(), 5);
    }

    #[test]
    fn the_log_device_says_what_an_overflow_dropped() {
        let mut area = Area([0; 64]);
        // SAFETY: the area outlives the test and is used only through the overlay.
        let mbp = unsafe { Msgbuf::from_raw(area.0.as_mut_ptr(), 64) };
        mbp.clear();
        mbp.set_magic(MSG_MAGIC);
        mbp.set_bufs(24);

        // 26 bytes into a ring of 24: three of them are dropped (a full ring holds 23).
        for b in b"abcdefghijklmnopqrstuvwxyz" {
            msgbuf_putchar(mbp, *b);
        }
        assert_eq!(mbp.bufd(), 3);
        let out = read(mbp, 200).expect("read");
        let line = b"<4>klog: dropped 3 bytes, message buffer full\n";
        assert!(out.starts_with(line));
        assert_eq!(&out[line.len()..], b"defghijklmnopqrstuvwxyz");
        assert_eq!(mbp.bufd(), 0);
    }

    #[test]
    fn initmsgbuf_keeps_a_sane_header_and_resets_a_bad_one() {
        // A private buffer: the global one is shared with every other test, which may print
        // while this one runs.
        let mut area = Area([0xff; 64]);
        let p = area.0.as_mut_ptr();
        // SAFETY: the area outlives the test and is used only through the overlay.
        let mbp = unsafe { Msgbuf::from_raw(p, 64) };
        initmsgbuf_header(mbp, 64);
        assert_eq!(mbp.magic(), MSG_MAGIC);
        assert_eq!(mbp.bufs(), 24);
        assert_eq!(mbp.bufx(), 0);
        msgbuf_putchar(mbp, b'a');
        // A second init over the same bytes sees a valid header.
        initmsgbuf_header(mbp, 64);
        assert_eq!(mbp.bufx(), 2, "a newline was appended after the kept 'a'");
        assert_eq!(mbp.bufc()[1].get(), b'\n');
        // A header that disagrees with the size is reset.
        mbp.set_bufs(23);
        initmsgbuf_header(mbp, 64);
        assert_eq!((mbp.bufs(), mbp.bufx()), (24, 0));
    }

    #[test]
    fn initmsgbuf_installs_the_static_buffer() {
        // The global buffer: only what concurrent printing cannot change is checked (its
        // size and that it is mapped), and the small area below is refused before anything
        // is installed.
        let mut area = Area([0; 64]);
        // SAFETY: too small: initmsgbuf returns before touching or installing the area.
        unsafe { initmsgbuf(area.0.as_mut_ptr(), 8) };
        logwakeup();
        init_static_msgbuf();
        assert!(msgbufmapped());
        assert_eq!(
            msgbufp().unwrap().bufs(),
            (MSGBUFSIZE - Msgbuf::HEADER_SIZE) as i64
        );
    }
}
/* </TESTS> */
