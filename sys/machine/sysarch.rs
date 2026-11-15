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
//! `sysarch(2)` as a trait: the machine-dependent system call each architecture's
//! `sys_machdep.c` implements (TD3).
//!
//! The system call table (`sys/kern/init_sysent.rs`) is generic; it reaches the selected
//! architecture's `sys_sysarch` through [`sys_sysarch`] here, as `kern_sig.rs` reaches
//! `sys_sigreturn` through `MachineSignal`.

use crate::machine::Machine;
use crate::sys::errno::Errno;
use crate::sys::proc::Proc;
use crate::sys::systm::SysArgs;
use crate::sys::types::Register;

/// The machine-dependent `sysarch(2)`.
pub trait MachineSysarch {
    /// `sys_sysarch(p, v, retval)`: the operations of `<machine/sysarch.h>`; EINVAL for an
    /// unknown `op`, `pledge_fail` under pledge.
    fn sys_sysarch(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno>;
}

/// `sys_sysarch`: the system call table's entry, the selected architecture's `sysarch(2)`.
pub fn sys_sysarch(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    <Machine as MachineSysarch>::sys_sysarch(p, v, retval)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The generic entry reaches the machine's `sys_sysarch` (the host double's is arm64's):
    // every op is EINVAL for an unpledged process, where the row answered SIGSYS before TD3.

    use std::boxed::Box;
    use std::{assert_eq, vec};

    use super::*;
    use crate::kern::kern_proc::procinit;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::proc::Process;

    #[test]
    fn every_op_is_einval_without_pledge() {
        let _g = setup_real_memory();
        crate::machine::cons::consinit();
        procinit();
        let pr: &'static Process = Box::leak(Box::new(Process::new()));
        let p: &'static Proc = Box::leak(Box::new(Proc::new()));
        p.p_p.set(pr);
        pr.ps_mainproc.set(p);
        let mut rv = [0; 2];
        for op in vec![0 as Register, 1, 2, 3, 0xffff_ffff] {
            let v: SysArgs = [op, 0, 0, 0, 0, 0];
            assert_eq!(sys_sysarch(p, &v, &mut rv), Err(Errno::EINVAL), "op {op}");
        }
    }
}
/* </TESTS> */
