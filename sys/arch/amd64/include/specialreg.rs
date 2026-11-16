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
 * Copyright (c) 1991 The Regents of the University of California.
 * All rights reserved.
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
 *	@(#)specialreg.h	7.1 (Berkeley) 5/9/91
 */
/* </LICENSES> */

/* <CODE> */
//! amd64 `<machine/specialreg.h>`: control registers, MSRs and CPUID bits.
//!
//! Upstream: sys/arch/amd64/include/specialreg.h @ 3ce1f3f79392
//!
//! Status: `ported`, the whole header: the control register bits, the extended state
//! components, the CPUID feature words with their `%b` descriptions, the model-specific
//! registers and their bits, the MTRR and PAT constants, the Cyrix, P5, P6, K7 and VIA
//! families, and the VMX and SVM definitions `vmm(4)` reads.
//!
//! A constant has the type of what it is compared with or written to: a control register's
//! or an MSR's bit is a `u64`, a CPUID register's a `u32`, an MSR's number the `u32`
//! `rdmsr` takes, a VMCS field the `u64` `vmread` takes, a VMCB member's value that
//! member's width. The VMX and SVM families have no reader yet (`vmm(4)` is not ported).
//!
//! ## Deviations
//! - `CPUID(code, eax, ebx, ecx, edx)` and `CPUID_LEAF(code, leaf, ...)` are the functions
//!   [`cpuid`] and [`cpuid_leaf`] returning `(eax, ebx, ecx, edx)`: Rust has no output macro
//!   arguments, and LLVM reserves `rbx`, which `core::arch::x86_64::__cpuid_count` handles.
//! - The `*_BITS` descriptions `%b` prints with are byte strings (`subr_prf.rs`'s
//!   `Bitmask`), with the C's octal escapes written in hexadecimal.
//! - `SVM_INTERCEPT_DR3_READ` is bit 3, as the debug registers around it run; the header
//!   says `1UL << 2`, which is DR2's bit (`docs/EXTERNAL_BUGS.md`, EXT-231). Nothing reads
//!   it.

use core::arch::x86_64::{__cpuid_count, CpuidResult};

// Bits in 386 special registers:
/// `CR0_PE`: Protected mode Enable.
pub const CR0_PE: u64 = 0x0000_0001;
/// `CR0_MP`: "Math" Present (NPX or NPX emulator).
pub const CR0_MP: u64 = 0x0000_0002;
/// `CR0_EM`: EMulate non-NPX coproc. (trap ESC only).
pub const CR0_EM: u64 = 0x0000_0004;
/// `CR0_TS`: Task Switched (if MP, trap ESC and WAIT).
pub const CR0_TS: u64 = 0x0000_0008;
/// `CR0_ET`: Extension Type (387 (if set) vs 287).
pub const CR0_ET: u64 = 0x0000_0010;
/// `CR0_PG`: PaGing enable.
pub const CR0_PG: u64 = 0x8000_0000;

// Bits in 486 special registers:
/// `CR0_NE`: Numeric Error enable (EX16 vs IRQ13).
pub const CR0_NE: u64 = 0x0000_0020;
/// `CR0_WP`: Write Protect (honor PG_RW in all modes).
pub const CR0_WP: u64 = 0x0001_0000;
/// `CR0_AM`: Alignment Mask (set to enable AC flag).
pub const CR0_AM: u64 = 0x0004_0000;
/// `CR0_NW`: Not Write-through.
pub const CR0_NW: u64 = 0x2000_0000;
/// `CR0_CD`: Cache Disable.
pub const CR0_CD: u64 = 0x4000_0000;

// bits in CR3
/// `CR3_PCID`.
pub const CR3_PCID: u64 = 0xfff;
/// `CR3_PWT`.
pub const CR3_PWT: u64 = 1 << 3;
/// `CR3_PCD`.
pub const CR3_PCD: u64 = 1 << 4;
/// `CR3_REUSE_PCID`: do not flush the PCID's TLB entries on load.
pub const CR3_REUSE_PCID: u64 = 1 << 63;
/// `CR3_PADDR`: the page-table address bits of `CR3`.
pub const CR3_PADDR: u64 = 0x7fff_ffff_ffff_f000;

// bits in the pentiums %cr4 register:

/// `CR4_VME`: virtual 8086 mode extension enable.
pub const CR4_VME: u64 = 0x0000_0001;
/// `CR4_PVI`: protected mode virtual interrupt enable.
pub const CR4_PVI: u64 = 0x0000_0002;
/// `CR4_TSD`: restrict RDTSC instruction to cpl 0 only.
pub const CR4_TSD: u64 = 0x0000_0004;
/// `CR4_DE`: debugging extension.
pub const CR4_DE: u64 = 0x0000_0008;
/// `CR4_PSE`: large (4MB) page size enable.
pub const CR4_PSE: u64 = 0x0000_0010;
/// `CR4_PAE`: physical address extension enable.
pub const CR4_PAE: u64 = 0x0000_0020;
/// `CR4_MCE`: machine check enable.
pub const CR4_MCE: u64 = 0x0000_0040;
/// `CR4_PGE`: page global enable.
pub const CR4_PGE: u64 = 0x0000_0080;
/// `CR4_PCE`: enable RDPMC instruction for all cpls.
pub const CR4_PCE: u64 = 0x0000_0100;
/// `CR4_OSFXSR`: enable fxsave/fxrestor and SSE.
pub const CR4_OSFXSR: u64 = 0x0000_0200;
/// `CR4_OSXMMEXCPT`: enable unmasked SSE exceptions.
pub const CR4_OSXMMEXCPT: u64 = 0x0000_0400;
/// `CR4_UMIP`: user mode instruction prevention.
pub const CR4_UMIP: u64 = 0x0000_0800;
/// `CR4_VMXE`: enable virtual machine operation.
pub const CR4_VMXE: u64 = 0x0000_2000;
/// `CR4_SMXE`: enable safe mode operation.
pub const CR4_SMXE: u64 = 0x0000_4000;
/// `CR4_FSGSBASE`: enable {RD,WR}{FS,GS}BASE ops.
pub const CR4_FSGSBASE: u64 = 0x0001_0000;
/// `CR4_PCIDE`: enable process-context IDs.
pub const CR4_PCIDE: u64 = 0x0002_0000;
/// `CR4_OSXSAVE`: enable XSAVE and extended states.
pub const CR4_OSXSAVE: u64 = 0x0004_0000;
/// `CR4_KL`: enable AES Key Locker.
pub const CR4_KL: u64 = 0x0008_0000;
/// `CR4_SMEP`: supervisor mode exec protection.
pub const CR4_SMEP: u64 = 0x0010_0000;
/// `CR4_SMAP`: supervisor mode access prevention.
pub const CR4_SMAP: u64 = 0x0020_0000;
/// `CR4_PKE`: user-mode protection keys.
pub const CR4_PKE: u64 = 0x0040_0000;
/// `CR4_CET`: control-flow enforcement tech.
pub const CR4_CET: u64 = 0x0080_0000;
/// `CR4_PKS`: supervisor-mode protection keys.
pub const CR4_PKS: u64 = 0x0100_0000;
/// `CR4_UINTR`: user interrupts enable bit.
pub const CR4_UINTR: u64 = 0x0200_0000;

// Extended state components, for xsave/xrstor family of instructions.
/// `XFEATURE_X87`: x87 FPU/MMX state.
pub const XFEATURE_X87: u64 = 0x0000_0001;
/// `XFEATURE_SSE`: SSE state.
pub const XFEATURE_SSE: u64 = 0x0000_0002;
/// `XFEATURE_AVX`: AVX state.
pub const XFEATURE_AVX: u64 = 0x0000_0004;
/// `XFEATURE_BNDREG`: MPX state.
pub const XFEATURE_BNDREG: u64 = 0x0000_0008;
/// `XFEATURE_BNDCSR`: MPX state.
pub const XFEATURE_BNDCSR: u64 = 0x0000_0010;
/// `XFEATURE_MPX`.
pub const XFEATURE_MPX: u64 = XFEATURE_BNDREG | XFEATURE_BNDCSR;
/// `XFEATURE_OPMASK`: AVX-512 opmask.
pub const XFEATURE_OPMASK: u64 = 0x0000_0020;
/// `XFEATURE_ZMM_HI256`: AVX-512 ZMM0-7.
pub const XFEATURE_ZMM_HI256: u64 = 0x0000_0040;
/// `XFEATURE_HI16_ZMM`: AVX-512 ZMM16-31.
pub const XFEATURE_HI16_ZMM: u64 = 0x0000_0080;
/// `XFEATURE_AVX512`.
pub const XFEATURE_AVX512: u64 = XFEATURE_OPMASK | XFEATURE_ZMM_HI256 | XFEATURE_HI16_ZMM;
/// `XFEATURE_PT`: processor trace.
pub const XFEATURE_PT: u64 = 0x0000_0100;
/// `XFEATURE_PKRU`: user page key.
pub const XFEATURE_PKRU: u64 = 0x0000_0200;
/// `XFEATURE_PASID`: Process ASIDs.
pub const XFEATURE_PASID: u64 = 0x0000_0400;
/// `XFEATURE_CET_U`: ctrl-flow enforce user.
pub const XFEATURE_CET_U: u64 = 0x0000_0800;
/// `XFEATURE_CET_S`: ctrl-flow enforce system.
pub const XFEATURE_CET_S: u64 = 0x0000_1000;
/// `XFEATURE_CET`.
pub const XFEATURE_CET: u64 = XFEATURE_CET_U | XFEATURE_CET_S;
/// `XFEATURE_HDC`: HW duty cycling.
pub const XFEATURE_HDC: u64 = 0x0000_2000;
/// `XFEATURE_UINTR`: user interrupts.
pub const XFEATURE_UINTR: u64 = 0x0000_4000;
/// `XFEATURE_LBR`: last-branch record.
pub const XFEATURE_LBR: u64 = 0x0000_8000;
/// `XFEATURE_HWP`: HW P-states.
pub const XFEATURE_HWP: u64 = 0x0001_0000;
/// `XFEATURE_TILECFG`: AMX state.
pub const XFEATURE_TILECFG: u64 = 0x0002_0000;
/// `XFEATURE_TILEDATA`: AMX state.
pub const XFEATURE_TILEDATA: u64 = 0x0004_0000;
/// `XFEATURE_AMX`.
pub const XFEATURE_AMX: u64 = XFEATURE_TILECFG | XFEATURE_TILEDATA;

// valid only in xcomp_bv field:
/// `XFEATURE_COMPRESSED`: compressed format.
pub const XFEATURE_COMPRESSED: u64 = 1 << 63;

// which bits are for XCR0 and which for the XSS MSR?
/// `XFEATURE_XCR0_MASK`.
pub const XFEATURE_XCR0_MASK: u64 = XFEATURE_X87
    | XFEATURE_SSE
    | XFEATURE_AVX
    | XFEATURE_MPX
    | XFEATURE_AVX512
    | XFEATURE_PKRU
    | XFEATURE_AMX;
/// `XFEATURE_XSS_MASK`.
pub const XFEATURE_XSS_MASK: u64 = XFEATURE_PT
    | XFEATURE_PASID
    | XFEATURE_CET
    | XFEATURE_HDC
    | XFEATURE_UINTR
    | XFEATURE_LBR
    | XFEATURE_HWP;

// CPUID "features" bits (CPUID function 0x1):
// EDX bits, then ECX bits

/// `CPUID_FPU`: processor has an FPU?
pub const CPUID_FPU: u32 = 0x0000_0001;
/// `CPUID_VME`: has virtual mode (%cr4's VME/PVI).
pub const CPUID_VME: u32 = 0x0000_0002;
/// `CPUID_DE`: has debugging extension.
pub const CPUID_DE: u32 = 0x0000_0004;
/// `CPUID_PSE`: has 4MB page size extension.
pub const CPUID_PSE: u32 = 0x0000_0008;
/// `CPUID_TSC`: has time stamp counter.
pub const CPUID_TSC: u32 = 0x0000_0010;
/// `CPUID_MSR`: has model specific registers.
pub const CPUID_MSR: u32 = 0x0000_0020;
/// `CPUID_PAE`: has phys address extension.
pub const CPUID_PAE: u32 = 0x0000_0040;
/// `CPUID_MCE`: has machine check exception.
pub const CPUID_MCE: u32 = 0x0000_0080;
/// `CPUID_CX8`: has CMPXCHG8B instruction.
pub const CPUID_CX8: u32 = 0x0000_0100;
/// `CPUID_APIC`: has enabled APIC.
pub const CPUID_APIC: u32 = 0x0000_0200;
/// `CPUID_SYS1`: has SYSCALL/SYSRET inst. (Cyrix).
pub const CPUID_SYS1: u32 = 0x0000_0400;
/// `CPUID_SEP`: has SYSCALL/SYSRET inst. (AMD/Intel).
pub const CPUID_SEP: u32 = 0x0000_0800;
/// `CPUID_MTRR`: has memory type range register.
pub const CPUID_MTRR: u32 = 0x0000_1000;
/// `CPUID_PGE`: has page global extension.
pub const CPUID_PGE: u32 = 0x0000_2000;
/// `CPUID_MCA`: has machine check architecture.
pub const CPUID_MCA: u32 = 0x0000_4000;
/// `CPUID_CMOV`: has CMOVcc instruction.
pub const CPUID_CMOV: u32 = 0x0000_8000;
/// `CPUID_PAT`: has page attribute table.
pub const CPUID_PAT: u32 = 0x0001_0000;
/// `CPUID_PSE36`: has 36bit page size extension.
pub const CPUID_PSE36: u32 = 0x0002_0000;
/// `CPUID_PSN`: has processor serial number.
pub const CPUID_PSN: u32 = 0x0004_0000;
/// `CPUID_CFLUSH`: CFLUSH insn supported.
pub const CPUID_CFLUSH: u32 = 0x0008_0000;
/// `CPUID_B20`: reserved.
pub const CPUID_B20: u32 = 0x0010_0000;
/// `CPUID_DS`: Debug Store.
pub const CPUID_DS: u32 = 0x0020_0000;
/// `CPUID_ACPI`: ACPI performance modulation regs.
pub const CPUID_ACPI: u32 = 0x0040_0000;
/// `CPUID_MMX`: has MMX instructions.
pub const CPUID_MMX: u32 = 0x0080_0000;
/// `CPUID_FXSR`: has FXRSTOR instruction.
pub const CPUID_FXSR: u32 = 0x0100_0000;
/// `CPUID_SSE`: has streaming SIMD extensions.
pub const CPUID_SSE: u32 = 0x0200_0000;
/// `CPUID_SSE2`: has streaming SIMD extensions #2.
pub const CPUID_SSE2: u32 = 0x0400_0000;
/// `CPUID_SS`: self-snoop.
pub const CPUID_SS: u32 = 0x0800_0000;
/// `CPUID_HTT`: Hyper-Threading Technology.
pub const CPUID_HTT: u32 = 0x1000_0000;
/// `CPUID_TM`: thermal monitor (TCC).
pub const CPUID_TM: u32 = 0x2000_0000;
/// `CPUID_B30`: reserved.
pub const CPUID_B30: u32 = 0x4000_0000;
/// `CPUID_PBE`: Pending Break Enabled restarts clock.
pub const CPUID_PBE: u32 = 0x8000_0000;
/// `CPUID_EDX_BITS`: the names of these bits, for `%b`.
pub const CPUID_EDX_BITS: &[u8] = b"\x10\x01FPU\x02VME\x03DE\x04PSE\x05TSC\x06MSR\x07PAE\x08MCE\
    \x09CX8\x0aAPIC\x0cSEP\x0dMTRR\x0ePGE\x0fMCA\x10CMOV\x11PAT\x12PSE36\x13PSN\x14CFLUSH\x16DS\
    \x17ACPI\x18MMX\x19FXSR\x1aSSE\x1bSSE2\x1cSS\x1dHTT\x1eTM\x20PBE";

/// `CPUIDECX_SSE3`: streaming SIMD extensions #3.
pub const CPUIDECX_SSE3: u32 = 0x0000_0001;
/// `CPUIDECX_PCLMUL`: Carryless Multiplication.
pub const CPUIDECX_PCLMUL: u32 = 0x0000_0002;
/// `CPUIDECX_DTES64`: 64bit debug store.
pub const CPUIDECX_DTES64: u32 = 0x0000_0004;
/// `CPUIDECX_MWAIT`: Monitor/Mwait.
pub const CPUIDECX_MWAIT: u32 = 0x0000_0008;
/// `CPUIDECX_DSCPL`: CPL Qualified Debug Store.
pub const CPUIDECX_DSCPL: u32 = 0x0000_0010;
/// `CPUIDECX_VMX`: Virtual Machine Extensions.
pub const CPUIDECX_VMX: u32 = 0x0000_0020;
/// `CPUIDECX_SMX`: Safer Mode Extensions.
pub const CPUIDECX_SMX: u32 = 0x0000_0040;
/// `CPUIDECX_EST`: enhanced SpeedStep.
pub const CPUIDECX_EST: u32 = 0x0000_0080;
/// `CPUIDECX_TM2`: thermal monitor 2.
pub const CPUIDECX_TM2: u32 = 0x0000_0100;
/// `CPUIDECX_SSSE3`: Supplemental Streaming SIMD Ext. 3.
pub const CPUIDECX_SSSE3: u32 = 0x0000_0200;
/// `CPUIDECX_CNXTID`: Context ID.
pub const CPUIDECX_CNXTID: u32 = 0x0000_0400;
/// `CPUIDECX_SDBG`: Silicon debug capability.
pub const CPUIDECX_SDBG: u32 = 0x0000_0800;
/// `CPUIDECX_FMA3`: Fused Multiply Add.
pub const CPUIDECX_FMA3: u32 = 0x0000_1000;
/// `CPUIDECX_CX16`: has CMPXCHG16B instruction.
pub const CPUIDECX_CX16: u32 = 0x0000_2000;
/// `CPUIDECX_XTPR`: xTPR Update Control.
pub const CPUIDECX_XTPR: u32 = 0x0000_4000;
/// `CPUIDECX_PDCM`: Perfmon and Debug Capability.
pub const CPUIDECX_PDCM: u32 = 0x0000_8000;
/// `CPUIDECX_PCID`: Process-context ID Capability.
pub const CPUIDECX_PCID: u32 = 0x0002_0000;
/// `CPUIDECX_DCA`: Direct Cache Access.
pub const CPUIDECX_DCA: u32 = 0x0004_0000;
/// `CPUIDECX_SSE41`: Streaming SIMD Extensions 4.1.
pub const CPUIDECX_SSE41: u32 = 0x0008_0000;
/// `CPUIDECX_SSE42`: Streaming SIMD Extensions 4.2.
pub const CPUIDECX_SSE42: u32 = 0x0010_0000;
/// `CPUIDECX_X2APIC`: Extended xAPIC Support.
pub const CPUIDECX_X2APIC: u32 = 0x0020_0000;
/// `CPUIDECX_MOVBE`: MOVBE Instruction.
pub const CPUIDECX_MOVBE: u32 = 0x0040_0000;
/// `CPUIDECX_POPCNT`: POPCNT Instruction.
pub const CPUIDECX_POPCNT: u32 = 0x0080_0000;
/// `CPUIDECX_DEADLINE`: APIC one-shot via deadline.
pub const CPUIDECX_DEADLINE: u32 = 0x0100_0000;
/// `CPUIDECX_AES`: AES Instruction.
pub const CPUIDECX_AES: u32 = 0x0200_0000;
/// `CPUIDECX_XSAVE`: XSAVE/XSTOR States.
pub const CPUIDECX_XSAVE: u32 = 0x0400_0000;
/// `CPUIDECX_OSXSAVE`: OSXSAVE.
pub const CPUIDECX_OSXSAVE: u32 = 0x0800_0000;
/// `CPUIDECX_AVX`: Advanced Vector Extensions.
pub const CPUIDECX_AVX: u32 = 0x1000_0000;
/// `CPUIDECX_F16C`: 16bit fp conversion.
pub const CPUIDECX_F16C: u32 = 0x2000_0000;
/// `CPUIDECX_RDRAND`: RDRAND instruction.
pub const CPUIDECX_RDRAND: u32 = 0x4000_0000;
/// `CPUIDECX_HV`: running on hypervisor.
pub const CPUIDECX_HV: u32 = 0x8000_0000;
/// `CPUID_ECX_BITS`: the names of these bits, for `%b`.
pub const CPUID_ECX_BITS: &[u8] = b"\x10\x01SSE3\x02PCLMUL\x03DTES64\x04MWAIT\x05DS-CPL\x06VMX\
    \x07SMX\x08EST\x09TM2\x0aSSSE3\x0bCNXT-ID\x0cSDBG\x0dFMA3\x0eCX16\x0fxTPR\x10PDCM\x12PCID\
    \x13DCA\x14SSE4.1\x15SSE4.2\x16x2APIC\x17MOVBE\x18POPCNT\x19DEADLINE\x1aAES\x1bXSAVE\
    \x1cOSXSAVE\x1dAVX\x1eF16C\x1fRDRAND\x20HV";

// "Structured Extended Feature Flags Parameters" (CPUID function 0x7, leaf 0)
// EBX bits
/// `SEFF0EBX_FSGSBASE`: {RD,WR}[FG]SBASE instructions.
pub const SEFF0EBX_FSGSBASE: u32 = 0x0000_0001;
/// `SEFF0EBX_TSC_ADJUST`: has IA32_TSC_ADJUST MSR.
pub const SEFF0EBX_TSC_ADJUST: u32 = 0x0000_0002;
/// `SEFF0EBX_SGX`: Software Guard Extensions.
pub const SEFF0EBX_SGX: u32 = 0x0000_0004;
/// `SEFF0EBX_BMI1`: advanced bit manipulation.
pub const SEFF0EBX_BMI1: u32 = 0x0000_0008;
/// `SEFF0EBX_HLE`: Hardware Lock Elision.
pub const SEFF0EBX_HLE: u32 = 0x0000_0010;
/// `SEFF0EBX_AVX2`: Advanced Vector Extensions 2.
pub const SEFF0EBX_AVX2: u32 = 0x0000_0020;
/// `SEFF0EBX_SMEP`: Supervisor mode exec protection.
pub const SEFF0EBX_SMEP: u32 = 0x0000_0080;
/// `SEFF0EBX_BMI2`: advanced bit manipulation.
pub const SEFF0EBX_BMI2: u32 = 0x0000_0100;
/// `SEFF0EBX_ERMS`: Enhanced REP MOVSB/STOSB.
pub const SEFF0EBX_ERMS: u32 = 0x0000_0200;
/// `SEFF0EBX_INVPCID`: INVPCID instruction.
pub const SEFF0EBX_INVPCID: u32 = 0x0000_0400;
/// `SEFF0EBX_RTM`: Restricted Transactional Memory.
pub const SEFF0EBX_RTM: u32 = 0x0000_0800;
/// `SEFF0EBX_PQM`: Quality of Service Monitoring.
pub const SEFF0EBX_PQM: u32 = 0x0000_1000;
/// `SEFF0EBX_MPX`: Memory Protection Extensions.
pub const SEFF0EBX_MPX: u32 = 0x0000_4000;
/// `SEFF0EBX_AVX512F`: AVX-512 foundation inst.
pub const SEFF0EBX_AVX512F: u32 = 0x0001_0000;
/// `SEFF0EBX_AVX512DQ`: AVX-512 double/quadword.
pub const SEFF0EBX_AVX512DQ: u32 = 0x0002_0000;
/// `SEFF0EBX_RDSEED`: RDSEED instruction.
pub const SEFF0EBX_RDSEED: u32 = 0x0004_0000;
/// `SEFF0EBX_ADX`: ADCX/ADOX instructions.
pub const SEFF0EBX_ADX: u32 = 0x0008_0000;
/// `SEFF0EBX_SMAP`: Supervisor mode access prevent.
pub const SEFF0EBX_SMAP: u32 = 0x0010_0000;
/// `SEFF0EBX_AVX512IFMA`: AVX-512 integer mult-add.
pub const SEFF0EBX_AVX512IFMA: u32 = 0x0020_0000;
/// `SEFF0EBX_PCOMMIT`: Persistent commit inst.
pub const SEFF0EBX_PCOMMIT: u32 = 0x0040_0000;
/// `SEFF0EBX_CLFLUSHOPT`: cache line flush.
pub const SEFF0EBX_CLFLUSHOPT: u32 = 0x0080_0000;
/// `SEFF0EBX_CLWB`: cache line write back.
pub const SEFF0EBX_CLWB: u32 = 0x0100_0000;
/// `SEFF0EBX_PT`: Processor Trace.
pub const SEFF0EBX_PT: u32 = 0x0200_0000;
/// `SEFF0EBX_AVX512PF`: AVX-512 prefetch.
pub const SEFF0EBX_AVX512PF: u32 = 0x0400_0000;
/// `SEFF0EBX_AVX512ER`: AVX-512 exp/reciprocal.
pub const SEFF0EBX_AVX512ER: u32 = 0x0800_0000;
/// `SEFF0EBX_AVX512CD`: AVX-512 conflict detection.
pub const SEFF0EBX_AVX512CD: u32 = 0x1000_0000;
/// `SEFF0EBX_SHA`: SHA Extensions.
pub const SEFF0EBX_SHA: u32 = 0x2000_0000;
/// `SEFF0EBX_AVX512BW`: AVX-512 byte/word inst.
pub const SEFF0EBX_AVX512BW: u32 = 0x4000_0000;
/// `SEFF0EBX_AVX512VL`: AVX-512 vector len inst.
pub const SEFF0EBX_AVX512VL: u32 = 0x8000_0000;
/// `SEFF0_EBX_BITS`: the names of these bits, for `%b`.
pub const SEFF0_EBX_BITS: &[u8] = b"\x10\x01FSGSBASE\x02TSC_ADJUST\x03SGX\x04BMI1\x05HLE\x06AVX2\
    \x08SMEP\x09BMI2\x0aERMS\x0bINVPCID\x0cRTM\x0dPQM\x0fMPX\x11AVX512F\x12AVX512DQ\x13RDSEED\
    \x14ADX\x15SMAP\x16AVX512IFMA\x17PCOMMIT\x18CLFLUSHOPT\x19CLWB\x1aPT\x1bAVX512PF\x1cAVX512ER\
    \x1dAVX512CD\x1eSHA\x1fAVX512BW\x20AVX512VL";

// SEFF ECX bits
/// `SEFF0ECX_PREFETCHWT1`: PREFETCHWT1 instruction.
pub const SEFF0ECX_PREFETCHWT1: u32 = 0x0000_0001;
/// `SEFF0ECX_AVX512VBMI`: AVX-512 vector bit inst.
pub const SEFF0ECX_AVX512VBMI: u32 = 0x0000_0002;
/// `SEFF0ECX_UMIP`: UMIP support.
pub const SEFF0ECX_UMIP: u32 = 0x0000_0004;
/// `SEFF0ECX_PKU`: Page prot keys for user mode.
pub const SEFF0ECX_PKU: u32 = 0x0000_0008;
/// `SEFF0ECX_OSPKE`: OS enabled RD/WRPKRU.
pub const SEFF0ECX_OSPKE: u32 = 0x0000_0010;
/// `SEFF0ECX_WAITPKG`: UMONITOR/UMWAIT/TPAUSE insns.
pub const SEFF0ECX_WAITPKG: u32 = 0x0000_0020;
/// `SEFF0ECX_PKS`: Page prot keys for sup mode.
pub const SEFF0ECX_PKS: u32 = 0x8000_0000;
/// `SEFF0_ECX_BITS`: the names of these bits, for `%b`.
pub const SEFF0_ECX_BITS: &[u8] = b"\x10\x01PREFETCHWT1\x02AVX512VBMI\x03UMIP\x04PKU\x06WAITPKG\
    \x20PKS";

// SEFF EDX bits
/// `SEFF0EDX_AVX512_4FNNIW`: AVX-512 neural network insns.
pub const SEFF0EDX_AVX512_4FNNIW: u32 = 0x0000_0004;
/// `SEFF0EDX_AVX512_4FMAPS`: AVX-512 mult accum single prec.
pub const SEFF0EDX_AVX512_4FMAPS: u32 = 0x0000_0008;
/// `SEFF0EDX_SRBDS_CTRL`: MCU_OPT_CTRL MSR.
pub const SEFF0EDX_SRBDS_CTRL: u32 = 0x0000_0200;
/// `SEFF0EDX_MD_CLEAR`: Microarch Data Clear.
pub const SEFF0EDX_MD_CLEAR: u32 = 0x0000_0400;
/// `SEFF0EDX_TSXFA`: TSX Forced Abort.
pub const SEFF0EDX_TSXFA: u32 = 0x0000_2000;
/// `SEFF0EDX_IBT`: Indirect Branch Tracking.
pub const SEFF0EDX_IBT: u32 = 0x0010_0000;
/// `SEFF0EDX_IBRS`: IBRS / IBPB Speculation Control.
pub const SEFF0EDX_IBRS: u32 = 0x0400_0000;
/// `SEFF0EDX_STIBP`: STIBP Speculation Control.
pub const SEFF0EDX_STIBP: u32 = 0x0800_0000;
/// `SEFF0EDX_L1DF`: L1D_FLUSH.
pub const SEFF0EDX_L1DF: u32 = 0x1000_0000;
/// `SEFF0EDX_ARCH_CAP`: Has IA32_ARCH_CAPABILITIES MSR.
pub const SEFF0EDX_ARCH_CAP: u32 = 0x2000_0000;
/// `SEFF0EDX_SSBD`: Spec Store Bypass Disable.
pub const SEFF0EDX_SSBD: u32 = 0x8000_0000;
/// `SEFF0_EDX_BITS`: the names of these bits, for `%b`.
pub const SEFF0_EDX_BITS: &[u8] = b"\x10\x03AVX512FNNIW\x04AVX512FMAPS\x0aSRBDS_CTRL\x0bMD_CLEAR\
    \x0eTSXFA\x15IBT\x1bIBRS,IBPB\x1cSTIBP\x1dL1DF\x20SSBD";

// SEFF subleaf 2 EDX bits
/// `SEFF2EDX_BHI_CTRL`: BHI_DIS_S supported.
pub const SEFF2EDX_BHI_CTRL: u32 = 0x0000_0010;
/// `SEFF2_EDX_BITS`: the names of these bits, for `%b`.
pub const SEFF2_EDX_BITS: &[u8] = b"\x10\x05BHI_CTRL";

// Thermal and Power Management (CPUID function 0x6) EAX bits
/// `TPM_SENSOR`: digital temp sensor.
pub const TPM_SENSOR: u32 = 0x0000_0001;
/// `TPM_ARAT`: APIC Timer Always Running.
pub const TPM_ARAT: u32 = 0x0000_0004;
/// `TPM_PTS`: Intel Package Thermal Status.
pub const TPM_PTS: u32 = 0x0000_0040;
/// `TPM_EAX_BITS`: the names of these bits, for `%b`.
pub const TPM_EAX_BITS: &[u8] = b"\x10\x01SENSOR\x03ARAT\x07PTS";

// Thermal and Power Management (CPUID function 0x6) ECX bits
/// `TPM_EFFFREQ`: APERF & MPERF MSR present.
pub const TPM_EFFFREQ: u32 = 0x0000_0001;
/// `TPM_ECX_BITS`: the names of these bits, for `%b`.
pub const TPM_ECX_BITS: &[u8] = b"\x10\x01EFFFREQ";

// "Architectural Performance Monitoring" bits (CPUID function 0x0a):
// EAX bits, EBX bits, EDX bits.

/// `CPUIDEAX_VERID`: version ID.
pub const CPUIDEAX_VERID: u32 = 0x0000_00ff;

/// `CPUIDEBX_EVT_CORE`: Core cycle.
pub const CPUIDEBX_EVT_CORE: u32 = 1;
/// `CPUIDEBX_EVT_INST`: Instruction retired.
pub const CPUIDEBX_EVT_INST: u32 = 1 << 1;
/// `CPUIDEBX_EVT_REFR`: Reference cycles.
pub const CPUIDEBX_EVT_REFR: u32 = 1 << 2;
/// `CPUIDEBX_EVT_CACHE_REF`: Last-level cache ref.
pub const CPUIDEBX_EVT_CACHE_REF: u32 = 1 << 3;
/// `CPUIDEBX_EVT_CACHE_MIS`: Last-level cache miss.
pub const CPUIDEBX_EVT_CACHE_MIS: u32 = 1 << 4;
/// `CPUIDEBX_EVT_BRANCH_INST`: Branch instruction ret.
pub const CPUIDEBX_EVT_BRANCH_INST: u32 = 1 << 5;
/// `CPUIDEBX_EVT_BRANCH_MISP`: Branch mispredict ret.
pub const CPUIDEBX_EVT_BRANCH_MISP: u32 = 1 << 6;

// CPUID "extended features" bits (CPUID function 0x80000001):
// EDX bits, then ECX bits

/// `CPUID_MPC`: Multiprocessing Capable.
pub const CPUID_MPC: u32 = 0x0008_0000;
/// `CPUID_NXE`: No-Execute Extension.
pub const CPUID_NXE: u32 = 0x0010_0000;
/// `CPUID_MMXX`: AMD MMX Extensions.
pub const CPUID_MMXX: u32 = 0x0040_0000;
/// `CPUID_FFXSR`: fast FP/MMX save/restore.
pub const CPUID_FFXSR: u32 = 0x0200_0000;
/// `CPUID_PAGE1GB`: 1-GByte pages.
pub const CPUID_PAGE1GB: u32 = 0x0400_0000;
/// `CPUID_RDTSCP`: RDTSCP / IA32_TSC_AUX available.
pub const CPUID_RDTSCP: u32 = 0x0800_0000;
/// `CPUID_LONG`: long mode.
pub const CPUID_LONG: u32 = 0x2000_0000;
/// `CPUID_3DNOW2`: 3DNow! Instruction Extension.
pub const CPUID_3DNOW2: u32 = 0x4000_0000;
/// `CPUID_3DNOW`: 3DNow! Instructions.
pub const CPUID_3DNOW: u32 = 0x8000_0000;
/// `CPUIDE_EDX_BITS`: the names of these bits, for `%b`.
pub const CPUIDE_EDX_BITS: &[u8] = b"\x10\x14MPC\x15NXE\x17MMXX\x1aFFXSR\x1bPAGE1GB\x1cRDTSCP\
    \x1eLONG\x1f3DNOW2\x203DNOW";

/// `CPUIDECX_LAHF`: LAHF and SAHF instructions.
pub const CPUIDECX_LAHF: u32 = 0x0000_0001;
/// `CPUIDECX_CMPLEG`: Core MP legacy mode.
pub const CPUIDECX_CMPLEG: u32 = 0x0000_0002;
/// `CPUIDECX_SVM`: Secure Virtual Machine.
pub const CPUIDECX_SVM: u32 = 0x0000_0004;
/// `CPUIDECX_EAPICSP`: Extended APIC space.
pub const CPUIDECX_EAPICSP: u32 = 0x0000_0008;
/// `CPUIDECX_AMCR8`: LOCK MOV CR0 means MOV CR8.
pub const CPUIDECX_AMCR8: u32 = 0x0000_0010;
/// `CPUIDECX_ABM`: LZCNT instruction.
pub const CPUIDECX_ABM: u32 = 0x0000_0020;
/// `CPUIDECX_SSE4A`: SSE4-A instruction set.
pub const CPUIDECX_SSE4A: u32 = 0x0000_0040;
/// `CPUIDECX_MASSE`: Misaligned SSE mode.
pub const CPUIDECX_MASSE: u32 = 0x0000_0080;
/// `CPUIDECX_3DNOWP`: 3DNowPrefetch.
pub const CPUIDECX_3DNOWP: u32 = 0x0000_0100;
/// `CPUIDECX_OSVW`: OS visible workaround.
pub const CPUIDECX_OSVW: u32 = 0x0000_0200;
/// `CPUIDECX_IBS`: Instruction based sampling.
pub const CPUIDECX_IBS: u32 = 0x0000_0400;
/// `CPUIDECX_XOP`: Extended operating support.
pub const CPUIDECX_XOP: u32 = 0x0000_0800;
/// `CPUIDECX_SKINIT`: SKINIT and STGI are supported.
pub const CPUIDECX_SKINIT: u32 = 0x0000_1000;
/// `CPUIDECX_WDT`: Watchdog timer.
pub const CPUIDECX_WDT: u32 = 0x0000_2000;

// Reserved			0x00004000
/// `CPUIDECX_LWP`: Lightweight profiling support.
pub const CPUIDECX_LWP: u32 = 0x0000_8000;
/// `CPUIDECX_FMA4`: 4-operand FMA instructions.
pub const CPUIDECX_FMA4: u32 = 0x0001_0000;
/// `CPUIDECX_TCE`: Translation Cache Extension.
pub const CPUIDECX_TCE: u32 = 0x0002_0000;

// Reserved			0x00040000
/// `CPUIDECX_NODEID`: Support for MSRC001C.
pub const CPUIDECX_NODEID: u32 = 0x0008_0000;

// Reserved			0x00100000
/// `CPUIDECX_TBM`: Trailing bit manipulation instruction.
pub const CPUIDECX_TBM: u32 = 0x0020_0000;
/// `CPUIDECX_TOPEXT`: Topology extensions support.
pub const CPUIDECX_TOPEXT: u32 = 0x0040_0000;
/// `CPUIDECX_CPCTR`: core performance counter ext.
pub const CPUIDECX_CPCTR: u32 = 0x0080_0000;
/// `CPUIDECX_DBKP`: DataBreakpointExtension.
pub const CPUIDECX_DBKP: u32 = 0x0400_0000;
/// `CPUIDECX_PERFTSC`: performance time-stamp counter.
pub const CPUIDECX_PERFTSC: u32 = 0x0800_0000;
/// `CPUIDECX_PCTRL3`: L3 performance counter ext.
pub const CPUIDECX_PCTRL3: u32 = 0x1000_0000;
/// `CPUIDECX_MWAITX`: MWAITX/MONITORX.
pub const CPUIDECX_MWAITX: u32 = 0x2000_0000;
/// `CPUIDE_ECX_BITS`: the names of these bits, for `%b`.
pub const CPUIDE_ECX_BITS: &[u8] = b"\x10\x01LAHF\x02CMPLEG\x03SVM\x04EAPICSP\x05AMCR8\x06ABM\
    \x07SSE4A\x08MASSE\x093DNOWP\x0aOSVW\x0bIBS\x0cXOP\x0dSKINIT\x10WDT\x11FMA4\x12TCE\x14NODEID\
    \x16TBM\x17TOPEXT\x18CPCTR\x1bDBKP\x1cPERFTSC\x1dPCTRL3\x1eMWAITX";

// "Advanced Power Management Information" bits (CPUID function 0x80000007):
// EDX bits.
/// `CPUIDEDX_HWPSTATE`: Hardware P State Control.
pub const CPUIDEDX_HWPSTATE: u32 = 1 << 7;
/// `CPUIDEDX_ITSC`: Invariant TSC.
pub const CPUIDEDX_ITSC: u32 = 1 << 8;
/// `CPUID_APMI_EDX_BITS`: the names of these bits, for `%b`.
pub const CPUID_APMI_EDX_BITS: &[u8] = b"\x10\x08HWPSTATE\x09ITSC";

// AMD CPUID function 0x80000008 EBX bits
/// `CPUIDEBX_INVLPGB`: INVLPG w/broadcast.
pub const CPUIDEBX_INVLPGB: u32 = 1 << 3;
/// `CPUIDEBX_IBPB`: Speculation Control IBPB.
pub const CPUIDEBX_IBPB: u32 = 1 << 12;
/// `CPUIDEBX_IBRS`: Speculation Control IBRS.
pub const CPUIDEBX_IBRS: u32 = 1 << 14;
/// `CPUIDEBX_STIBP`: Speculation Control STIBP.
pub const CPUIDEBX_STIBP: u32 = 1 << 15;
/// `CPUIDEBX_IBRS_ALWAYSON`: IBRS always on mode.
pub const CPUIDEBX_IBRS_ALWAYSON: u32 = 1 << 16;
/// `CPUIDEBX_STIBP_ALWAYSON`: STIBP always on mode.
pub const CPUIDEBX_STIBP_ALWAYSON: u32 = 1 << 17;
/// `CPUIDEBX_IBRS_PREF`: IBRS preferred.
pub const CPUIDEBX_IBRS_PREF: u32 = 1 << 18;
/// `CPUIDEBX_IBRS_SAME_MODE`: IBRS not mode-specific.
pub const CPUIDEBX_IBRS_SAME_MODE: u32 = 1 << 19;
/// `CPUIDEBX_SSBD`: Speculation Control SSBD.
pub const CPUIDEBX_SSBD: u32 = 1 << 24;
/// `CPUIDEBX_VIRT_SSBD`: Virt Spec Control SSBD.
pub const CPUIDEBX_VIRT_SSBD: u32 = 1 << 25;
/// `CPUIDEBX_SSBD_NOTREQ`: SSBD not required.
pub const CPUIDEBX_SSBD_NOTREQ: u32 = 1 << 26;
/// `CPUID_AMDSPEC_EBX_BITS`: the names of these bits, for `%b`.
pub const CPUID_AMDSPEC_EBX_BITS: &[u8] = b"\x10\x04INVLPGB\x0dIBPB\x0fIBRS\x10STIBP\x11IBRS_ALL\
    \x12STIBP_ALL\x13IBRS_PREF\x14IBRS_SM\x19SSBD\x1aVIRTSSBD\x1bSSBDNR";

// AMD CPUID function 0x8000001F EAX bits
/// `CPUIDEAX_SME`: SME.
pub const CPUIDEAX_SME: u32 = 1;
/// `CPUIDEAX_SEV`: SEV.
pub const CPUIDEAX_SEV: u32 = 1 << 1;
/// `CPUIDEAX_PFLUSH_MSR`: Page Flush MSR.
pub const CPUIDEAX_PFLUSH_MSR: u32 = 1 << 2;
/// `CPUIDEAX_SEVES`: SEV-ES.
pub const CPUIDEAX_SEVES: u32 = 1 << 3;
/// `CPUIDEAX_SEVSNP`: SEV-SNP.
pub const CPUIDEAX_SEVSNP: u32 = 1 << 4;
/// `CPUIDEAX_VMPL`: VM Permission Levels.
pub const CPUIDEAX_VMPL: u32 = 1 << 5;
/// `CPUIDEAX_RMPQUERY`: RMPQUERY.
pub const CPUIDEAX_RMPQUERY: u32 = 1 << 6;
/// `CPUIDEAX_VMPLSSS`: VMPL Supervisor Shadow Stack.
pub const CPUIDEAX_VMPLSSS: u32 = 1 << 7;
/// `CPUIDEAX_SECTSC`: Secure TSC.
pub const CPUIDEAX_SECTSC: u32 = 1 << 8;
/// `CPUIDEAX_TSCAUXVIRT`: TSC Aux Virtualization.
pub const CPUIDEAX_TSCAUXVIRT: u32 = 1 << 9;
/// `CPUIDEAX_HWECACHECOH`: Coherency Across Enc. Domains.
pub const CPUIDEAX_HWECACHECOH: u32 = 1 << 10;
/// `CPUIDEAX_64BITHOST`: SEV guest requires 64bit host.
pub const CPUIDEAX_64BITHOST: u32 = 1 << 11;
/// `CPUIDEAX_RESTINJ`: Restricted Injection.
pub const CPUIDEAX_RESTINJ: u32 = 1 << 12;
/// `CPUIDEAX_ALTINJ`: Alternate Injection.
pub const CPUIDEAX_ALTINJ: u32 = 1 << 13;
/// `CPUIDEAX_DBGSTSW`: Full debug state swap.
pub const CPUIDEAX_DBGSTSW: u32 = 1 << 14;
/// `CPUIDEAX_IBSDISALLOW`: Disallowing IBS use by host.
pub const CPUIDEAX_IBSDISALLOW: u32 = 1 << 15;
/// `CPUIDEAX_VTE`: Virt. Transparent Encryption.
pub const CPUIDEAX_VTE: u32 = 1 << 16;
/// `CPUIDEAX_VMGEXITPARAM`: VMGEXIT Parameter.
pub const CPUIDEAX_VMGEXITPARAM: u32 = 1 << 17;
/// `CPUIDEAX_VTOMMSR`: Virtual TOM MSR.
pub const CPUIDEAX_VTOMMSR: u32 = 1 << 18;
/// `CPUIDEAX_IBSVIRT`: IBS Virtualization for SEV-ES.
pub const CPUIDEAX_IBSVIRT: u32 = 1 << 19;
/// `CPUIDEAX_VMSARPROT`: VMSA Register Protection.
pub const CPUIDEAX_VMSARPROT: u32 = 1 << 24;
/// `CPUIDEAX_SMTPROT`: SMT Protection.
pub const CPUIDEAX_SMTPROT: u32 = 1 << 25;
/// `CPUIDEAX_SVSMPAGEMSR`: SVSM Communication Page MSR.
pub const CPUIDEAX_SVSMPAGEMSR: u32 = 1 << 28;
/// `CPUIDEAX_NVSMSR`: NestedVirtSnpMsr.
pub const CPUIDEAX_NVSMSR: u32 = 1 << 29;
/// `CPUID_AMDSEV_EAX_BITS`: the names of these bits, for `%b`.
pub const CPUID_AMDSEV_EAX_BITS: &[u8] = b"\x10\x01SME\x02SEV\x03PFLUSH_MSR\x04SEVES\x05SEVSNP\
    \x06VMPL\x07RMPQUERY\x08VMPLSSS\x09SECTSC\x0aTSCAUXVIRT\x0bHWECACHECOH\x0cREQ64BITHOST\
    \x0dRESTINJ\x0eALTINJ\x0fDBGSTSW\x10IBSDISALLOW\x11VTE\x12VMGEXITPARAM\x13VTOMMSR\x14IBSVIRT\
    \x19VMSARPROT\x1aSMTPROT\x1dSVSMPAGEMSR\x1eNVSMSR";

// Number of encrypted guests
/// `CPUID_AMDSEV_ECX_BITS`: the names of these bits, for `%b`.
pub const CPUID_AMDSEV_ECX_BITS: &[u8] = b"\x10";

// Minimum ASID for SEV enabled, SEV-ES disabled guest.
/// `CPUID_AMDSEV_EDX_BITS`: the names of these bits, for `%b`.
pub const CPUID_AMDSEV_EDX_BITS: &[u8] = b"\x10";

// Model-specific registers for the i386 family
/// `MSR_P5_MC_ADDR`: P5 only.
pub const MSR_P5_MC_ADDR: u32 = 0x000;
/// `MSR_P5_MC_TYPE`: P5 only.
pub const MSR_P5_MC_TYPE: u32 = 0x001;
/// `MSR_TSC`.
pub const MSR_TSC: u32 = 0x010;
/// `MSR_CESR`: P5 only (trap on P6).
pub const MSR_CESR: u32 = 0x011;
/// `MSR_CTR0`: P5 only (trap on P6).
pub const MSR_CTR0: u32 = 0x012;
/// `MSR_CTR1`: P5 only (trap on P6).
pub const MSR_CTR1: u32 = 0x013;
/// `MSR_PLATFORM_ID`: Platform ID for microcode.
pub const MSR_PLATFORM_ID: u32 = 0x017;
/// `MSR_APICBASE`: the local APIC's base address and mode.
pub const MSR_APICBASE: u32 = 0x01b;
/// `APICBASE_BSP`.
pub const APICBASE_BSP: u64 = 0x100;
/// `APICBASE_ENABLE_X2APIC`.
pub const APICBASE_ENABLE_X2APIC: u64 = 0x400;
/// `APICBASE_GLOBAL_ENABLE`.
pub const APICBASE_GLOBAL_ENABLE: u64 = 0x800;
/// `APICBASE_ADDRESS_MASK`.
pub const APICBASE_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
/// `MSR_EBL_CR_POWERON`.
pub const MSR_EBL_CR_POWERON: u32 = 0x02a;
/// `MSR_EBC_FREQUENCY_ID`: Pentium 4 only.
pub const MSR_EBC_FREQUENCY_ID: u32 = 0x02c;
/// `MSR_TEST_CTL`.
pub const MSR_TEST_CTL: u32 = 0x033;
/// `MSR_IA32_FEATURE_CONTROL`.
pub const MSR_IA32_FEATURE_CONTROL: u32 = 0x03a;
/// `MSR_TSC_ADJUST`.
pub const MSR_TSC_ADJUST: u32 = 0x03b;
/// `MSR_SPEC_CTRL`: Speculation Control IBRS / STIBP.
pub const MSR_SPEC_CTRL: u32 = 0x048;
/// `SPEC_CTRL_IBRS`.
pub const SPEC_CTRL_IBRS: u64 = 1;
/// `SPEC_CTRL_STIBP`.
pub const SPEC_CTRL_STIBP: u64 = 1 << 1;
/// `SPEC_CTRL_SSBD`.
pub const SPEC_CTRL_SSBD: u64 = 1 << 2;
/// `SPEC_CTRL_BHI_DIS_S`.
pub const SPEC_CTRL_BHI_DIS_S: u64 = 1 << 10;
/// `MSR_PRED_CMD`: Speculation Control IBPB.
pub const MSR_PRED_CMD: u32 = 0x049;
/// `PRED_CMD_IBPB`.
pub const PRED_CMD_IBPB: u64 = 1;
/// `MSR_BIOS_UPDT_TRIG`.
pub const MSR_BIOS_UPDT_TRIG: u32 = 0x079;
/// `MSR_BBL_CR_D0`: PII+ only.
pub const MSR_BBL_CR_D0: u32 = 0x088;
/// `MSR_BBL_CR_D1`: PII+ only.
pub const MSR_BBL_CR_D1: u32 = 0x089;
/// `MSR_BBL_CR_D2`: PII+ only.
pub const MSR_BBL_CR_D2: u32 = 0x08a;
/// `MSR_BIOS_SIGN`.
pub const MSR_BIOS_SIGN: u32 = 0x08b;
/// `MSR_SMM_MONITOR_CTL`.
pub const MSR_SMM_MONITOR_CTL: u32 = 0x09b;
/// `MSR_SMBASE`.
pub const MSR_SMBASE: u32 = 0x09e;
/// `MSR_PERFCTR0`.
pub const MSR_PERFCTR0: u32 = 0x0c1;
/// `MSR_PERFCTR1`.
pub const MSR_PERFCTR1: u32 = 0x0c2;
/// `MSR_FSB_FREQ`: Core Duo/Solo only.
pub const MSR_FSB_FREQ: u32 = 0x0cd;
/// `MSR_MPERF`.
pub const MSR_MPERF: u32 = 0x0e7;
/// `MSR_APERF`.
pub const MSR_APERF: u32 = 0x0e8;
/// `MSR_MTRRcap`.
#[allow(non_upper_case_globals)] // the C name
pub const MSR_MTRRcap: u32 = 0x0fe;
/// `MTRRcap_FIXED`: bit 8 - fixed MTRRs supported.
#[allow(non_upper_case_globals)] // the C name
pub const MTRRcap_FIXED: u64 = 0x100;
/// `MTRRcap_WC`: bit 10 - WC type supported.
#[allow(non_upper_case_globals)] // the C name
pub const MTRRcap_WC: u64 = 0x400;
/// `MTRRcap_SMRR`: bit 11 - SMM range reg supported.
#[allow(non_upper_case_globals)] // the C name
pub const MTRRcap_SMRR: u64 = 0x800;
/// `MSR_ARCH_CAPABILITIES`.
pub const MSR_ARCH_CAPABILITIES: u32 = 0x10a;
/// `ARCH_CAP_RDCL_NO`: Meltdown safe.
pub const ARCH_CAP_RDCL_NO: u64 = 1;
/// `ARCH_CAP_IBRS_ALL`: enhanced IBRS.
pub const ARCH_CAP_IBRS_ALL: u64 = 1 << 1;
/// `ARCH_CAP_RSBA`: RSB Alternate.
pub const ARCH_CAP_RSBA: u64 = 1 << 2;
/// `ARCH_CAP_SKIP_L1DFL_VMENTRY`.
pub const ARCH_CAP_SKIP_L1DFL_VMENTRY: u64 = 1 << 3;
/// `ARCH_CAP_SSB_NO`: Spec St Byp safe.
pub const ARCH_CAP_SSB_NO: u64 = 1 << 4;
/// `ARCH_CAP_MDS_NO`: microarch data-sampling.
pub const ARCH_CAP_MDS_NO: u64 = 1 << 5;
/// `ARCH_CAP_IF_PSCHANGE_MC_NO`: PS MCE safe.
pub const ARCH_CAP_IF_PSCHANGE_MC_NO: u64 = 1 << 6;
/// `ARCH_CAP_TSX_CTRL`: has TSX_CTRL MSR.
pub const ARCH_CAP_TSX_CTRL: u64 = 1 << 7;
/// `ARCH_CAP_TAA_NO`: TSX AA safe.
pub const ARCH_CAP_TAA_NO: u64 = 1 << 8;
/// `ARCH_CAP_MCU_CONTROL`: has MCU_CTRL MSR.
pub const ARCH_CAP_MCU_CONTROL: u64 = 1 << 9;
/// `ARCH_CAP_MISC_PACKAGE_CTLS`: has MISC_PKG_CTLS MSR.
pub const ARCH_CAP_MISC_PACKAGE_CTLS: u64 = 1 << 10;
/// `ARCH_CAP_ENERGY_FILTERING_CTL`: r/w energy fltring bit.
pub const ARCH_CAP_ENERGY_FILTERING_CTL: u64 = 1 << 11;
/// `ARCH_CAP_DOITM`: Data oprnd indpdnt tmng.
pub const ARCH_CAP_DOITM: u64 = 1 << 12;
/// `ARCH_CAP_SBDR_SSDP_NO`: SBDR/SSDP safe.
pub const ARCH_CAP_SBDR_SSDP_NO: u64 = 1 << 13;
/// `ARCH_CAP_FBSDP_NO`: FBSDP safe.
pub const ARCH_CAP_FBSDP_NO: u64 = 1 << 14;
/// `ARCH_CAP_PSDP_NO`: PSDP safe.
pub const ARCH_CAP_PSDP_NO: u64 = 1 << 15;
/// `ARCH_CAP_FB_CLEAR`: MD_CLEAR covers FB.
pub const ARCH_CAP_FB_CLEAR: u64 = 1 << 17;
/// `ARCH_CAP_FB_CLEAR_CTRL`.
pub const ARCH_CAP_FB_CLEAR_CTRL: u64 = 1 << 18;
/// `ARCH_CAP_RRSBA`: has RRSBA if not dis.
pub const ARCH_CAP_RRSBA: u64 = 1 << 19;
/// `ARCH_CAP_BHI_NO`: BHI safe.
pub const ARCH_CAP_BHI_NO: u64 = 1 << 20;
/// `ARCH_CAP_XAPIC_DISABLE_STATUS`: can disable xAPIC.
pub const ARCH_CAP_XAPIC_DISABLE_STATUS: u64 = 1 << 21;
/// `ARCH_CAP_OVERCLOCKING_STATUS`: has OVRCLCKNG_STAT MSR.
pub const ARCH_CAP_OVERCLOCKING_STATUS: u64 = 1 << 23;
/// `ARCH_CAP_PBRSB_NO`: PBSR safe.
pub const ARCH_CAP_PBRSB_NO: u64 = 1 << 24;
/// `ARCH_CAP_GDS_CTRL`: has GDS_MITG_DIS/LOCK.
pub const ARCH_CAP_GDS_CTRL: u64 = 1 << 25;
/// `ARCH_CAP_GDS_NO`: GDS safe.
pub const ARCH_CAP_GDS_NO: u64 = 1 << 26;
/// `ARCH_CAP_RFDS_NO`: RFDS safe.
pub const ARCH_CAP_RFDS_NO: u64 = 1 << 27;
/// `ARCH_CAP_RFDS_CLEAR`: use VERW for RFDS.
pub const ARCH_CAP_RFDS_CLEAR: u64 = 1 << 28;
/// `ARCH_CAP_MSR_BITS`: the names of these bits, for `%b`.
pub const ARCH_CAP_MSR_BITS: &[u8] = b"\x10\x02IBRS_ALL\x03RSBA\x04SKIP_L1DFL\x05SSB_NO\x06MDS_NO\
    \x07IF_PSCHANGE\x08TSX_CTRL\x09TAA_NO\x0aMCU_CONTROL\x0bMISC_PKG_CT\x0cENERGY_FILT\x0dDOITM\
    \x0eSBDR_SSDP_N\x0fFBSDP_NO\x10PSDP_NO\x12FB_CLEAR\x13FB_CLEAR_CT\x14RRSBA\x15BHI_NO\
    \x16XAPIC_DIS\x18OVERCLOCK\x19PBRSB_NO\x1aGDS_CTRL\x1bGDS_NO\x1cRFDS_NO\x1dRFDS_CLEAR";

/// `MSR_FLUSH_CMD`.
pub const MSR_FLUSH_CMD: u32 = 0x10b;
/// `FLUSH_CMD_L1D_FLUSH`: (1ULL << 0).
pub const FLUSH_CMD_L1D_FLUSH: u64 = 0x1;
/// `MSR_BBL_CR_ADDR`: PII+ only.
pub const MSR_BBL_CR_ADDR: u32 = 0x116;
/// `MSR_BBL_CR_DECC`: PII+ only.
pub const MSR_BBL_CR_DECC: u32 = 0x118;
/// `MSR_BBL_CR_CTL`: PII+ only.
pub const MSR_BBL_CR_CTL: u32 = 0x119;
/// `MSR_BBL_CR_TRIG`: PII+ only.
pub const MSR_BBL_CR_TRIG: u32 = 0x11a;
/// `MSR_BBL_CR_BUSY`: PII+ only.
pub const MSR_BBL_CR_BUSY: u32 = 0x11b;
/// `MSR_BBL_CR_CTR3`: PII+ only.
pub const MSR_BBL_CR_CTR3: u32 = 0x11e;
/// `MSR_TSX_CTRL`.
pub const MSR_TSX_CTRL: u32 = 0x122;
/// `TSX_CTRL_RTM_DISABLE`.
pub const TSX_CTRL_RTM_DISABLE: u64 = 1;
/// `TSX_CTRL_TSX_CPUID_CLEAR`.
pub const TSX_CTRL_TSX_CPUID_CLEAR: u64 = 1 << 1;
/// `MSR_MCU_OPT_CTRL`.
pub const MSR_MCU_OPT_CTRL: u32 = 0x123;
/// `RNGDS_MITG_DIS`.
pub const RNGDS_MITG_DIS: u64 = 1;
/// `MSR_SYSENTER_CS`: PII+ only.
pub const MSR_SYSENTER_CS: u32 = 0x174;
/// `MSR_SYSENTER_ESP`: PII+ only.
pub const MSR_SYSENTER_ESP: u32 = 0x175;
/// `MSR_SYSENTER_EIP`: PII+ only.
pub const MSR_SYSENTER_EIP: u32 = 0x176;
/// `MSR_MCG_CAP`.
pub const MSR_MCG_CAP: u32 = 0x179;
/// `MSR_MCG_STATUS`.
pub const MSR_MCG_STATUS: u32 = 0x17a;
/// `MSR_MCG_CTL`.
pub const MSR_MCG_CTL: u32 = 0x17b;
/// `MSR_EVNTSEL0`.
pub const MSR_EVNTSEL0: u32 = 0x186;
/// `MSR_EVNTSEL1`.
pub const MSR_EVNTSEL1: u32 = 0x187;
/// `MSR_PERF_STATUS`: Pentium M.
pub const MSR_PERF_STATUS: u32 = 0x198;
/// `MSR_PERF_CTL`: Pentium M.
pub const MSR_PERF_CTL: u32 = 0x199;
/// `PERF_CTL_TURBO`: bit 32 - turbo mode.
pub const PERF_CTL_TURBO: u64 = 0x0000_0001_0000_0000;
/// `MSR_THERM_CONTROL`.
pub const MSR_THERM_CONTROL: u32 = 0x19a;
/// `MSR_THERM_INTERRUPT`.
pub const MSR_THERM_INTERRUPT: u32 = 0x19b;
/// `MSR_THERM_STATUS`.
pub const MSR_THERM_STATUS: u32 = 0x19c;
/// `MSR_THERM_STATUS_VALID_BIT`.
pub const MSR_THERM_STATUS_VALID_BIT: u64 = 0x8000_0000;
/// `MSR_THERM2_CTL`: Pentium M.
pub const MSR_THERM2_CTL: u32 = 0x19d;
/// `MSR_MISC_ENABLE`.
pub const MSR_MISC_ENABLE: u32 = 0x1a0;

// MSR_MISC_ENABLE (0x1a0)
//
// Enable Fast Strings: enables fast REP MOVS/REP STORS (R/W)
// Enable TCC: Enable automatic thermal control circuit (R/W)
// Performance monitoring available: 1 if enabled (R/O)
// Branch trace storage unavailable: 1 if unsupported (R/O)
// Processor event based sampling unavailable: 1 if unsupported (R/O)
// Enhanced Intel SpeedStep technology enable: 1 to enable (R/W)
// Enable monitor FSM: 1 to enable MONITOR/MWAIT (R/W)
// Limit CPUID maxval: 1 to limit CPUID leaf nodes to 0x2 and lower (R/W)
// Enable xTPR message disable: 1 to disable xTPR messages
// XD bit disable: 1 to disable NX capability (bit 34, or bit 2 of %edx/%rdx)
/// `MISC_ENABLE_FAST_STRINGS`.
pub const MISC_ENABLE_FAST_STRINGS: u64 = 1;
/// `MISC_ENABLE_TCC`.
pub const MISC_ENABLE_TCC: u64 = 1 << 3;
/// `MISC_ENABLE_PERF_MON_AVAILABLE`.
pub const MISC_ENABLE_PERF_MON_AVAILABLE: u64 = 1 << 7;
/// `MISC_ENABLE_BTS_UNAVAILABLE`.
pub const MISC_ENABLE_BTS_UNAVAILABLE: u64 = 1 << 11;
/// `MISC_ENABLE_PEBS_UNAVAILABLE`.
pub const MISC_ENABLE_PEBS_UNAVAILABLE: u64 = 1 << 12;
/// `MISC_ENABLE_EIST_ENABLED`.
pub const MISC_ENABLE_EIST_ENABLED: u64 = 1 << 16;
/// `MISC_ENABLE_ENABLE_MONITOR_FSM`.
pub const MISC_ENABLE_ENABLE_MONITOR_FSM: u64 = 1 << 18;
/// `MISC_ENABLE_LIMIT_CPUID_MAXVAL`.
pub const MISC_ENABLE_LIMIT_CPUID_MAXVAL: u64 = 1 << 22;
/// `MISC_ENABLE_xTPR_MESSAGE_DISABLE`.
#[allow(non_upper_case_globals)] // the C name
pub const MISC_ENABLE_xTPR_MESSAGE_DISABLE: u64 = 1 << 23;
/// `MISC_ENABLE_XD_BIT_DISABLE`.
pub const MISC_ENABLE_XD_BIT_DISABLE: u64 = 1 << 2;

// for Core i Series and newer Xeons, see
// http://www.intel.com/content/dam/www/public/us/en/
// documents/white-papers/cpu-monitoring-dts-peci-paper.pdf
/// `MSR_TEMPERATURE_TARGET`: Core i Series, Newer Xeons.
pub const MSR_TEMPERATURE_TARGET: u32 = 0x1a2;

// not documented anywhere, see intelcore_update_sensor()
// only available Core Duo and Core Solo Processors
/// `MSR_TEMPERATURE_TARGET_UNDOCUMENTED`.
pub const MSR_TEMPERATURE_TARGET_UNDOCUMENTED: u32 = 0x0ee;
/// `MSR_TEMPERATURE_TARGET_LOW_BIT_UNDOCUMENTED`.
pub const MSR_TEMPERATURE_TARGET_LOW_BIT_UNDOCUMENTED: u64 = 0x4000_0000;
/// `MSR_DEBUGCTLMSR`.
pub const MSR_DEBUGCTLMSR: u32 = 0x1d9;
/// `MSR_LASTBRANCHFROMIP`.
pub const MSR_LASTBRANCHFROMIP: u32 = 0x1db;
/// `MSR_LASTBRANCHTOIP`.
pub const MSR_LASTBRANCHTOIP: u32 = 0x1dc;
/// `MSR_LASTINTFROMIP`.
pub const MSR_LASTINTFROMIP: u32 = 0x1dd;
/// `MSR_LASTINTTOIP`.
pub const MSR_LASTINTTOIP: u32 = 0x1de;
/// `MSR_ROB_CR_BKUPTMPDR6`.
pub const MSR_ROB_CR_BKUPTMPDR6: u32 = 0x1e0;
/// `MSR_MTRRvarBase`.
#[allow(non_upper_case_globals)] // the C name
pub const MSR_MTRRvarBase: u32 = 0x200;
/// `MSR_MTRRfix64K_00000`.
#[allow(non_upper_case_globals)] // the C name
pub const MSR_MTRRfix64K_00000: u32 = 0x250;
/// `MSR_MTRRfix16K_80000`.
#[allow(non_upper_case_globals)] // the C name
pub const MSR_MTRRfix16K_80000: u32 = 0x258;
/// `MSR_MTRRfix4K_C0000`.
#[allow(non_upper_case_globals)] // the C name
pub const MSR_MTRRfix4K_C0000: u32 = 0x268;
/// `MSR_CR_PAT`.
pub const MSR_CR_PAT: u32 = 0x277;
/// `MSR_MTRRdefType`.
#[allow(non_upper_case_globals)] // the C name
pub const MSR_MTRRdefType: u32 = 0x2ff;
/// `MTRRdefType_FIXED_ENABLE`: bit 10 - fixed MTRR enabled.
#[allow(non_upper_case_globals)] // the C name
pub const MTRRdefType_FIXED_ENABLE: u64 = 0x400;
/// `MTRRdefType_ENABLE`: bit 11 - MTRRs enabled.
#[allow(non_upper_case_globals)] // the C name
pub const MTRRdefType_ENABLE: u64 = 0x800;
/// `MSR_PERF_FIXED_CTR1`: CPU_CLK_Unhalted.Core.
pub const MSR_PERF_FIXED_CTR1: u32 = 0x30a;
/// `MSR_PERF_FIXED_CTR2`: CPU_CLK.Unhalted.Ref.
pub const MSR_PERF_FIXED_CTR2: u32 = 0x30b;
/// `MSR_PERF_FIXED_CTR_CTRL`.
pub const MSR_PERF_FIXED_CTR_CTRL: u32 = 0x38d;
/// `MSR_PERF_FIXED_CTR_FC_DIS`: disable counter.
pub const MSR_PERF_FIXED_CTR_FC_DIS: u64 = 0x0;
/// `MSR_PERF_FIXED_CTR_FC_1`: count ring 1.
pub const MSR_PERF_FIXED_CTR_FC_1: u64 = 0x1;
/// `MSR_PERF_FIXED_CTR_FC_123`: count rings 1,2,3.
pub const MSR_PERF_FIXED_CTR_FC_123: u64 = 0x2;
/// `MSR_PERF_FIXED_CTR_FC_ANY`: count everything.
pub const MSR_PERF_FIXED_CTR_FC_ANY: u64 = 0x3;
/// `MSR_PERF_FIXED_CTR_FC_MASK`.
pub const MSR_PERF_FIXED_CTR_FC_MASK: u64 = 0x3;
/// `MSR_PERF_GLOBAL_CTRL`.
pub const MSR_PERF_GLOBAL_CTRL: u32 = 0x38f;
/// `MSR_PERF_GLOBAL_CTR1_EN`.
pub const MSR_PERF_GLOBAL_CTR1_EN: u64 = 1 << 33;
/// `MSR_PERF_GLOBAL_CTR2_EN`.
pub const MSR_PERF_GLOBAL_CTR2_EN: u64 = 1 << 34;
/// `MSR_PKG_C3_RESIDENCY`.
pub const MSR_PKG_C3_RESIDENCY: u32 = 0x3f8;
/// `MSR_PKG_C6_RESIDENCY`.
pub const MSR_PKG_C6_RESIDENCY: u32 = 0x3f9;
/// `MSR_PKG_C7_RESIDENCY`.
pub const MSR_PKG_C7_RESIDENCY: u32 = 0x3fa;
/// `MSR_CORE_C3_RESIDENCY`.
pub const MSR_CORE_C3_RESIDENCY: u32 = 0x3fc;
/// `MSR_CORE_C6_RESIDENCY`.
pub const MSR_CORE_C6_RESIDENCY: u32 = 0x3fd;
/// `MSR_CORE_C7_RESIDENCY`.
pub const MSR_CORE_C7_RESIDENCY: u32 = 0x3fe;
/// `MSR_MC0_CTL`.
pub const MSR_MC0_CTL: u32 = 0x400;
/// `MSR_MC0_STATUS`.
pub const MSR_MC0_STATUS: u32 = 0x401;
/// `MSR_MC0_ADDR`.
pub const MSR_MC0_ADDR: u32 = 0x402;
/// `MSR_MC0_MISC`.
pub const MSR_MC0_MISC: u32 = 0x403;
/// `MSR_MC1_CTL`.
pub const MSR_MC1_CTL: u32 = 0x404;
/// `MSR_MC1_STATUS`.
pub const MSR_MC1_STATUS: u32 = 0x405;
/// `MSR_MC1_ADDR`.
pub const MSR_MC1_ADDR: u32 = 0x406;
/// `MSR_MC1_MISC`.
pub const MSR_MC1_MISC: u32 = 0x407;
/// `MSR_MC2_CTL`.
pub const MSR_MC2_CTL: u32 = 0x408;
/// `MSR_MC2_STATUS`.
pub const MSR_MC2_STATUS: u32 = 0x409;
/// `MSR_MC2_ADDR`.
pub const MSR_MC2_ADDR: u32 = 0x40a;
/// `MSR_MC2_MISC`.
pub const MSR_MC2_MISC: u32 = 0x40b;
/// `MSR_MC4_CTL`.
pub const MSR_MC4_CTL: u32 = 0x40c;
/// `MSR_MC4_STATUS`.
pub const MSR_MC4_STATUS: u32 = 0x40d;
/// `MSR_MC4_ADDR`.
pub const MSR_MC4_ADDR: u32 = 0x40e;
/// `MSR_MC4_MISC`.
pub const MSR_MC4_MISC: u32 = 0x40f;
/// `MSR_MC3_CTL`.
pub const MSR_MC3_CTL: u32 = 0x410;
/// `MSR_MC3_STATUS`.
pub const MSR_MC3_STATUS: u32 = 0x411;
/// `MSR_MC3_ADDR`.
pub const MSR_MC3_ADDR: u32 = 0x412;
/// `MSR_MC3_MISC`.
pub const MSR_MC3_MISC: u32 = 0x413;
/// `MSR_PKG_C2_RESIDENCY`.
pub const MSR_PKG_C2_RESIDENCY: u32 = 0x60d;
/// `MSR_PKG_C8_RESIDENCY`.
pub const MSR_PKG_C8_RESIDENCY: u32 = 0x630;
/// `MSR_PKG_C9_RESIDENCY`.
pub const MSR_PKG_C9_RESIDENCY: u32 = 0x631;
/// `MSR_PKG_C10_RESIDENCY`.
pub const MSR_PKG_C10_RESIDENCY: u32 = 0x632;
/// `MSR_U_CET`.
pub const MSR_U_CET: u32 = 0x6a0;
/// `MSR_CET_ENDBR_EN`.
pub const MSR_CET_ENDBR_EN: u64 = 1 << 2;
/// `MSR_CET_NO_TRACK_EN`.
pub const MSR_CET_NO_TRACK_EN: u64 = 1 << 4;
/// `MSR_S_CET`.
pub const MSR_S_CET: u32 = 0x6a2;
/// `MSR_PKRS`.
pub const MSR_PKRS: u32 = 0x6e1;
/// `MSR_XSS`.
pub const MSR_XSS: u32 = 0xda0;

// VIA MSR
/// `MSR_CENT_TMTEMPERATURE`: Thermal monitor temperature.
pub const MSR_CENT_TMTEMPERATURE: u32 = 0x1423;

// AMD K6/K7 MSRs.
/// `MSR_K6_UWCCR`.
pub const MSR_K6_UWCCR: u32 = 0xc000_0085;
/// `MSR_K7_EVNTSEL0`.
pub const MSR_K7_EVNTSEL0: u32 = 0xc001_0000;
/// `MSR_K7_EVNTSEL1`.
pub const MSR_K7_EVNTSEL1: u32 = 0xc001_0001;
/// `MSR_K7_EVNTSEL2`.
pub const MSR_K7_EVNTSEL2: u32 = 0xc001_0002;
/// `MSR_K7_EVNTSEL3`.
pub const MSR_K7_EVNTSEL3: u32 = 0xc001_0003;
/// `MSR_K7_PERFCTR0`.
pub const MSR_K7_PERFCTR0: u32 = 0xc001_0004;
/// `MSR_K7_PERFCTR1`.
pub const MSR_K7_PERFCTR1: u32 = 0xc001_0005;
/// `MSR_K7_PERFCTR2`.
pub const MSR_K7_PERFCTR2: u32 = 0xc001_0006;
/// `MSR_K7_PERFCTR3`.
pub const MSR_K7_PERFCTR3: u32 = 0xc001_0007;

// AMD K8 (Opteron) MSRs.
/// `MSR_PATCH_LEVEL`.
pub const MSR_PATCH_LEVEL: u32 = 0x0000_008b;
/// `MSR_SYSCFG`.
pub const MSR_SYSCFG: u32 = 0xc000_0010;

/// `MSR_EFER`: Extended feature enable.
pub const MSR_EFER: u32 = 0xc000_0080;
/// `EFER_SCE`: SYSCALL extension.
pub const EFER_SCE: u64 = 0x0000_0001;
/// `EFER_LME`: Long Mode Enabled.
pub const EFER_LME: u64 = 0x0000_0100;
/// `EFER_LMA`: Long Mode Active.
pub const EFER_LMA: u64 = 0x0000_0400;
/// `EFER_NXE`: No-Execute Enabled.
pub const EFER_NXE: u64 = 0x0000_0800;
/// `EFER_SVME`: SVM Enabled.
pub const EFER_SVME: u64 = 0x0000_1000;

/// `MSR_STAR`: the `syscall`/`sysret` segment selectors.
pub const MSR_STAR: u32 = 0xc000_0081;
/// `MSR_LSTAR`: the 64-bit `syscall` entry point.
pub const MSR_LSTAR: u32 = 0xc000_0082;
/// `MSR_CSTAR`: the compatibility-mode `syscall` entry point.
pub const MSR_CSTAR: u32 = 0xc000_0083;
/// `MSR_SFMASK`: the `RFLAGS` bits `syscall` clears.
pub const MSR_SFMASK: u32 = 0xc000_0084;

/// `MSR_FSBASE`: the `FS` segment base.
pub const MSR_FSBASE: u32 = 0xc000_0100;
/// `MSR_GSBASE`: the `GS` segment base.
pub const MSR_GSBASE: u32 = 0xc000_0101;
/// `MSR_KERNELGSBASE`: the `GS` base `swapgs` swaps in.
pub const MSR_KERNELGSBASE: u32 = 0xc000_0102;
/// `MSR_SYS_CFG`: System Configuration.
pub const MSR_SYS_CFG: u32 = 0xc001_0010;
/// `MSR_PATCH_LOADER`.
pub const MSR_PATCH_LOADER: u32 = 0xc001_0020;
/// `MSR_INT_PEN_MSG`: Interrupt pending message.
pub const MSR_INT_PEN_MSG: u32 = 0xc001_0055;

/// `MSR_FP_CFG`: Floating Point Configuration.
pub const MSR_FP_CFG: u32 = 0xc001_1028;
/// `FP_CFG_9`: FP-DSS chickenbit.
pub const FP_CFG_9: u64 = 1 << 9;
/// `MSR_DE_CFG`: Decode Configuration.
pub const MSR_DE_CFG: u32 = 0xc001_1029;
/// `DE_CFG_721`: errata 721.
pub const DE_CFG_721: u64 = 0x0000_0001;
/// `DE_CFG_SERIALIZE_LFENCE`: Enable serializing lfence.
pub const DE_CFG_SERIALIZE_LFENCE: u64 = 1 << 1;
/// `DE_CFG_SERIALIZE_9`: Zenbleed chickenbit.
pub const DE_CFG_SERIALIZE_9: u64 = 1 << 9;

/// `MSR_BP_CFG`.
pub const MSR_BP_CFG: u32 = 0xc001_102e;
/// `BP_CFG_33`: op cache chickenbit, AMD-SB-7052.
pub const BP_CFG_33: u64 = 1 << 33;

/// `IPM_C1E_CMP_HLT`.
pub const IPM_C1E_CMP_HLT: u64 = 0x1000_0000;
/// `IPM_SMI_CMP_HLT`.
pub const IPM_SMI_CMP_HLT: u64 = 0x0800_0000;

// These require a 'passcode' for access.  See cpufunc.h.
/// `MSR_HWCR`.
pub const MSR_HWCR: u32 = 0xc001_0015;
/// `HWCR_FFDIS`.
pub const HWCR_FFDIS: u64 = 0x0000_0040;
/// `HWCR_TSCFREQSEL`.
pub const HWCR_TSCFREQSEL: u64 = 0x0100_0000;

/// `PSTATEDEF_EN`.
pub const PSTATEDEF_EN: u64 = 0x8000_0000_0000_0000;

/// `MSR_NB_CFG`.
pub const MSR_NB_CFG: u32 = 0xc001_001f;
/// `NB_CFG_DISIOREQLOCK`.
pub const NB_CFG_DISIOREQLOCK: u64 = 0x0000_0000_0000_0004;
/// `NB_CFG_DISDATMSK`.
pub const NB_CFG_DISDATMSK: u64 = 0x0000_0010_0000_0000;

/// `MSR_SEV_GHCB`: the guest-hypervisor communication block of an SEV-ES guest
/// (`locore0.S`'s `#VC` termination request).
pub const MSR_SEV_GHCB: u32 = 0xc001_0130;
/// `SEV_CPUID_REQ`.
pub const SEV_CPUID_REQ: u64 = 0x0000_0004;
/// `SEV_CPUID_RESP`.
pub const SEV_CPUID_RESP: u64 = 0x0000_0005;

/// `MSR_SEV_STATUS`.
pub const MSR_SEV_STATUS: u32 = 0xc001_0131;
/// `SEV_STAT_ENABLED`: SEV is active.
pub const SEV_STAT_ENABLED: i32 = 0x0000_0001;
/// `SEV_STAT_ES_ENABLED`: SEV-ES is active.
pub const SEV_STAT_ES_ENABLED: i32 = 0x0000_0002;
/// `SEV_STAT_SNP_ACTIVE`: SEV-SNP is active.
pub const SEV_STAT_SNP_ACTIVE: i32 = 0x0000_0004;

/// `MSR_LS_CFG`.
pub const MSR_LS_CFG: u32 = 0xc001_1020;
/// `LS_CFG_DIS_LS2_SQUISH`.
pub const LS_CFG_DIS_LS2_SQUISH: u64 = 0x0200_0000;

/// `MSR_IC_CFG`.
pub const MSR_IC_CFG: u32 = 0xc001_1021;
/// `IC_CFG_DIS_SEQ_PREFETCH`.
pub const IC_CFG_DIS_SEQ_PREFETCH: u64 = 0x0000_0800;

/// `MSR_DC_CFG`.
pub const MSR_DC_CFG: u32 = 0xc001_1022;
/// `DC_CFG_DIS_CNV_WC_SSO`.
pub const DC_CFG_DIS_CNV_WC_SSO: u64 = 0x0000_0004;
/// `DC_CFG_DIS_SMC_CHK_BUF`.
pub const DC_CFG_DIS_SMC_CHK_BUF: u64 = 0x0000_0400;

/// `MSR_BU_CFG`.
pub const MSR_BU_CFG: u32 = 0xc001_1023;
/// `BU_CFG_THRL2IDXCMPDIS`.
pub const BU_CFG_THRL2IDXCMPDIS: u64 = 0x0000_0800_0000_0000;
/// `BU_CFG_WBPFSMCCHKDIS`.
pub const BU_CFG_WBPFSMCCHKDIS: u64 = 0x0000_2000_0000_0000;
/// `BU_CFG_WBENHWSBDIS`.
pub const BU_CFG_WBENHWSBDIS: u64 = 0x0001_0000_0000_0000;

// Constants related to MTRRs
/// `MTRR_N64K`: numbers of fixed-size entries.
pub const MTRR_N64K: usize = 8;
/// `MTRR_N16K`.
pub const MTRR_N16K: usize = 16;
/// `MTRR_N4K`.
pub const MTRR_N4K: usize = 64;

// the following four 3-byte registers control the non-cacheable regions.
// These registers must be written as three separate bytes.
//
// NCRx+0: A31-A24 of starting address
// NCRx+1: A23-A16 of starting address
// NCRx+2: A15-A12 of starting address | NCR_SIZE_xx.
//
// The non-cacheable region's starting address must be aligned to the
// size indicated by the NCR_SIZE_xx field.
/// `NCR1`.
pub const NCR1: u8 = 0xc4;
/// `NCR2`.
pub const NCR2: u8 = 0xc7;
/// `NCR3`.
pub const NCR3: u8 = 0xca;
/// `NCR4`.
pub const NCR4: u8 = 0xcd;

/// `NCR_SIZE_0K`.
pub const NCR_SIZE_0K: u8 = 0;
/// `NCR_SIZE_4K`.
pub const NCR_SIZE_4K: u8 = 1;
/// `NCR_SIZE_8K`.
pub const NCR_SIZE_8K: u8 = 2;
/// `NCR_SIZE_16K`.
pub const NCR_SIZE_16K: u8 = 3;
/// `NCR_SIZE_32K`.
pub const NCR_SIZE_32K: u8 = 4;
/// `NCR_SIZE_64K`.
pub const NCR_SIZE_64K: u8 = 5;
/// `NCR_SIZE_128K`.
pub const NCR_SIZE_128K: u8 = 6;
/// `NCR_SIZE_256K`.
pub const NCR_SIZE_256K: u8 = 7;
/// `NCR_SIZE_512K`.
pub const NCR_SIZE_512K: u8 = 8;
/// `NCR_SIZE_1M`.
pub const NCR_SIZE_1M: u8 = 9;
/// `NCR_SIZE_2M`.
pub const NCR_SIZE_2M: u8 = 10;
/// `NCR_SIZE_4M`.
pub const NCR_SIZE_4M: u8 = 11;
/// `NCR_SIZE_8M`.
pub const NCR_SIZE_8M: u8 = 12;
/// `NCR_SIZE_16M`.
pub const NCR_SIZE_16M: u8 = 13;
/// `NCR_SIZE_32M`.
pub const NCR_SIZE_32M: u8 = 14;
/// `NCR_SIZE_4G`.
pub const NCR_SIZE_4G: u8 = 15;

// 586-class CESR MSR format.  Lower 16 bits is CTR0, upper 16 bits
// is CTR1.

/// `PMC5_CESR_EVENT`.
pub const PMC5_CESR_EVENT: u32 = 0x003f;
/// `PMC5_CESR_OS`.
pub const PMC5_CESR_OS: u32 = 0x0040;
/// `PMC5_CESR_USR`.
pub const PMC5_CESR_USR: u32 = 0x0080;
/// `PMC5_CESR_E`.
pub const PMC5_CESR_E: u32 = 0x0100;
/// `PMC5_CESR_P`.
pub const PMC5_CESR_P: u32 = 0x0200;

/// `PMC5_DATA_READ`.
pub const PMC5_DATA_READ: u32 = 0x00;
/// `PMC5_DATA_WRITE`.
pub const PMC5_DATA_WRITE: u32 = 0x01;
/// `PMC5_DATA_TLB_MISS`.
pub const PMC5_DATA_TLB_MISS: u32 = 0x02;
/// `PMC5_DATA_READ_MISS`.
pub const PMC5_DATA_READ_MISS: u32 = 0x03;
/// `PMC5_DATA_WRITE_MISS`.
pub const PMC5_DATA_WRITE_MISS: u32 = 0x04;
/// `PMC5_WRITE_M_E`.
pub const PMC5_WRITE_M_E: u32 = 0x05;
/// `PMC5_DATA_LINES_WBACK`.
pub const PMC5_DATA_LINES_WBACK: u32 = 0x06;
/// `PMC5_DATA_CACHE_SNOOP`.
pub const PMC5_DATA_CACHE_SNOOP: u32 = 0x07;
/// `PMC5_DATA_CACHE_SNOOP_HIT`.
pub const PMC5_DATA_CACHE_SNOOP_HIT: u32 = 0x08;
/// `PMC5_MEM_ACCESS_BOTH_PIPES`.
pub const PMC5_MEM_ACCESS_BOTH_PIPES: u32 = 0x09;
/// `PMC5_BANK_CONFLICTS`.
pub const PMC5_BANK_CONFLICTS: u32 = 0x0a;
/// `PMC5_MISALIGNED_DATA`.
pub const PMC5_MISALIGNED_DATA: u32 = 0x0b;
/// `PMC5_INST_READ`.
pub const PMC5_INST_READ: u32 = 0x0c;
/// `PMC5_INST_TLB_MISS`.
pub const PMC5_INST_TLB_MISS: u32 = 0x0d;
/// `PMC5_INST_CACHE_MISS`.
pub const PMC5_INST_CACHE_MISS: u32 = 0x0e;
/// `PMC5_SEGMENT_REG_LOAD`.
pub const PMC5_SEGMENT_REG_LOAD: u32 = 0x0f;
/// `PMC5_BRANCHES`.
pub const PMC5_BRANCHES: u32 = 0x12;
/// `PMC5_BTB_HITS`.
pub const PMC5_BTB_HITS: u32 = 0x13;
/// `PMC5_BRANCH_TAKEN`.
pub const PMC5_BRANCH_TAKEN: u32 = 0x14;
/// `PMC5_PIPELINE_FLUSH`.
pub const PMC5_PIPELINE_FLUSH: u32 = 0x15;
/// `PMC5_INST_EXECUTED`.
pub const PMC5_INST_EXECUTED: u32 = 0x16;
/// `PMC5_INST_EXECUTED_V_PIPE`.
pub const PMC5_INST_EXECUTED_V_PIPE: u32 = 0x17;
/// `PMC5_BUS_UTILIZATION`.
pub const PMC5_BUS_UTILIZATION: u32 = 0x18;
/// `PMC5_WRITE_BACKUP_STALL`.
pub const PMC5_WRITE_BACKUP_STALL: u32 = 0x19;
/// `PMC5_DATA_READ_STALL`.
pub const PMC5_DATA_READ_STALL: u32 = 0x1a;
/// `PMC5_WRITE_E_M_STALL`.
pub const PMC5_WRITE_E_M_STALL: u32 = 0x1b;
/// `PMC5_LOCKED_BUS`.
pub const PMC5_LOCKED_BUS: u32 = 0x1c;
/// `PMC5_IO_CYCLE`.
pub const PMC5_IO_CYCLE: u32 = 0x1d;
/// `PMC5_NONCACHE_MEM_READ`.
pub const PMC5_NONCACHE_MEM_READ: u32 = 0x1e;
/// `PMC5_AGI_STALL`.
pub const PMC5_AGI_STALL: u32 = 0x1f;
/// `PMC5_FLOPS`.
pub const PMC5_FLOPS: u32 = 0x22;
/// `PMC5_BP0_MATCH`.
pub const PMC5_BP0_MATCH: u32 = 0x23;
/// `PMC5_BP1_MATCH`.
pub const PMC5_BP1_MATCH: u32 = 0x24;
/// `PMC5_BP2_MATCH`.
pub const PMC5_BP2_MATCH: u32 = 0x25;
/// `PMC5_BP3_MATCH`.
pub const PMC5_BP3_MATCH: u32 = 0x26;
/// `PMC5_HARDWARE_INTR`.
pub const PMC5_HARDWARE_INTR: u32 = 0x27;
/// `PMC5_DATA_RW`.
pub const PMC5_DATA_RW: u32 = 0x28;
/// `PMC5_DATA_RW_MISS`.
pub const PMC5_DATA_RW_MISS: u32 = 0x29;

// 686-class Event Selector MSR format.

/// `PMC6_EVTSEL_EVENT`.
pub const PMC6_EVTSEL_EVENT: u32 = 0x0000_00ff;
/// `PMC6_EVTSEL_UNIT`.
pub const PMC6_EVTSEL_UNIT: u32 = 0x0000_ff00;
/// `PMC6_EVTSEL_UNIT_SHIFT`.
pub const PMC6_EVTSEL_UNIT_SHIFT: u32 = 8;
/// `PMC6_EVTSEL_USR`.
pub const PMC6_EVTSEL_USR: u32 = 1 << 16;
/// `PMC6_EVTSEL_OS`.
pub const PMC6_EVTSEL_OS: u32 = 1 << 17;
/// `PMC6_EVTSEL_E`.
pub const PMC6_EVTSEL_E: u32 = 1 << 18;
/// `PMC6_EVTSEL_PC`.
pub const PMC6_EVTSEL_PC: u32 = 1 << 19;
/// `PMC6_EVTSEL_INT`.
pub const PMC6_EVTSEL_INT: u32 = 1 << 20;
/// `PMC6_EVTSEL_EN`: PerfEvtSel0 only.
pub const PMC6_EVTSEL_EN: u32 = 1 << 22;
/// `PMC6_EVTSEL_INV`.
pub const PMC6_EVTSEL_INV: u32 = 1 << 23;
/// `PMC6_EVTSEL_COUNTER_MASK`.
pub const PMC6_EVTSEL_COUNTER_MASK: u32 = 0xff00_0000;
/// `PMC6_EVTSEL_COUNTER_MASK_SHIFT`.
pub const PMC6_EVTSEL_COUNTER_MASK_SHIFT: u32 = 24;

// Data Cache Unit
/// `PMC6_DATA_MEM_REFS`.
pub const PMC6_DATA_MEM_REFS: u32 = 0x43;
/// `PMC6_DCU_LINES_IN`.
pub const PMC6_DCU_LINES_IN: u32 = 0x45;
/// `PMC6_DCU_M_LINES_IN`.
pub const PMC6_DCU_M_LINES_IN: u32 = 0x46;
/// `PMC6_DCU_M_LINES_OUT`.
pub const PMC6_DCU_M_LINES_OUT: u32 = 0x47;
/// `PMC6_DCU_MISS_OUTSTANDING`.
pub const PMC6_DCU_MISS_OUTSTANDING: u32 = 0x48;

// Instruction Fetch Unit
/// `PMC6_IFU_IFETCH`.
pub const PMC6_IFU_IFETCH: u32 = 0x80;
/// `PMC6_IFU_IFETCH_MISS`.
pub const PMC6_IFU_IFETCH_MISS: u32 = 0x81;
/// `PMC6_ITLB_MISS`.
pub const PMC6_ITLB_MISS: u32 = 0x85;
/// `PMC6_IFU_MEM_STALL`.
pub const PMC6_IFU_MEM_STALL: u32 = 0x86;
/// `PMC6_ILD_STALL`.
pub const PMC6_ILD_STALL: u32 = 0x87;

// L2 Cache
/// `PMC6_L2_IFETCH`.
pub const PMC6_L2_IFETCH: u32 = 0x28;
/// `PMC6_L2_LD`.
pub const PMC6_L2_LD: u32 = 0x29;
/// `PMC6_L2_ST`.
pub const PMC6_L2_ST: u32 = 0x2a;
/// `PMC6_L2_LINES_IN`.
pub const PMC6_L2_LINES_IN: u32 = 0x24;
/// `PMC6_L2_LINES_OUT`.
pub const PMC6_L2_LINES_OUT: u32 = 0x26;
/// `PMC6_L2_M_LINES_INM`.
pub const PMC6_L2_M_LINES_INM: u32 = 0x25;
/// `PMC6_L2_M_LINES_OUTM`.
pub const PMC6_L2_M_LINES_OUTM: u32 = 0x27;
/// `PMC6_L2_RQSTS`.
pub const PMC6_L2_RQSTS: u32 = 0x2e;
/// `PMC6_L2_ADS`.
pub const PMC6_L2_ADS: u32 = 0x21;
/// `PMC6_L2_DBUS_BUSY`.
pub const PMC6_L2_DBUS_BUSY: u32 = 0x22;
/// `PMC6_L2_DBUS_BUSY_RD`.
pub const PMC6_L2_DBUS_BUSY_RD: u32 = 0x23;

// External Bus Logic
/// `PMC6_BUS_DRDY_CLOCKS`.
pub const PMC6_BUS_DRDY_CLOCKS: u32 = 0x62;
/// `PMC6_BUS_LOCK_CLOCKS`.
pub const PMC6_BUS_LOCK_CLOCKS: u32 = 0x63;
/// `PMC6_BUS_REQ_OUTSTANDING`.
pub const PMC6_BUS_REQ_OUTSTANDING: u32 = 0x60;
/// `PMC6_BUS_TRAN_BRD`.
pub const PMC6_BUS_TRAN_BRD: u32 = 0x65;
/// `PMC6_BUS_TRAN_RFO`.
pub const PMC6_BUS_TRAN_RFO: u32 = 0x66;
/// `PMC6_BUS_TRANS_WB`.
pub const PMC6_BUS_TRANS_WB: u32 = 0x67;
/// `PMC6_BUS_TRAN_IFETCH`.
pub const PMC6_BUS_TRAN_IFETCH: u32 = 0x68;
/// `PMC6_BUS_TRAN_INVAL`.
pub const PMC6_BUS_TRAN_INVAL: u32 = 0x69;
/// `PMC6_BUS_TRAN_PWR`.
pub const PMC6_BUS_TRAN_PWR: u32 = 0x6a;
/// `PMC6_BUS_TRANS_P`.
pub const PMC6_BUS_TRANS_P: u32 = 0x6b;
/// `PMC6_BUS_TRANS_IO`.
pub const PMC6_BUS_TRANS_IO: u32 = 0x6c;
/// `PMC6_BUS_TRAN_DEF`.
pub const PMC6_BUS_TRAN_DEF: u32 = 0x6d;
/// `PMC6_BUS_TRAN_BURST`.
pub const PMC6_BUS_TRAN_BURST: u32 = 0x6e;
/// `PMC6_BUS_TRAN_ANY`.
pub const PMC6_BUS_TRAN_ANY: u32 = 0x70;
/// `PMC6_BUS_TRAN_MEM`.
pub const PMC6_BUS_TRAN_MEM: u32 = 0x6f;
/// `PMC6_BUS_DATA_RCV`.
pub const PMC6_BUS_DATA_RCV: u32 = 0x64;
/// `PMC6_BUS_BNR_DRV`.
pub const PMC6_BUS_BNR_DRV: u32 = 0x61;
/// `PMC6_BUS_HIT_DRV`.
pub const PMC6_BUS_HIT_DRV: u32 = 0x7a;
/// `PMC6_BUS_HITM_DRDV`.
pub const PMC6_BUS_HITM_DRDV: u32 = 0x7b;
/// `PMC6_BUS_SNOOP_STALL`.
pub const PMC6_BUS_SNOOP_STALL: u32 = 0x7e;

// Floating Point Unit
/// `PMC6_FLOPS`.
pub const PMC6_FLOPS: u32 = 0xc1;
/// `PMC6_FP_COMP_OPS_EXE`.
pub const PMC6_FP_COMP_OPS_EXE: u32 = 0x10;
/// `PMC6_FP_ASSIST`.
pub const PMC6_FP_ASSIST: u32 = 0x11;
/// `PMC6_MUL`.
pub const PMC6_MUL: u32 = 0x12;
/// `PMC6_DIV`.
pub const PMC6_DIV: u32 = 0x12;
/// `PMC6_CYCLES_DIV_BUSY`.
pub const PMC6_CYCLES_DIV_BUSY: u32 = 0x14;

// Memory Ordering
/// `PMC6_LD_BLOCKS`.
pub const PMC6_LD_BLOCKS: u32 = 0x03;
/// `PMC6_SB_DRAINS`.
pub const PMC6_SB_DRAINS: u32 = 0x04;
/// `PMC6_MISALIGN_MEM_REF`.
pub const PMC6_MISALIGN_MEM_REF: u32 = 0x05;
/// `PMC6_EMON_KNI_PREF_DISPATCHED`: P-III only.
pub const PMC6_EMON_KNI_PREF_DISPATCHED: u32 = 0x07;
/// `PMC6_EMON_KNI_PREF_MISS`: P-III only.
pub const PMC6_EMON_KNI_PREF_MISS: u32 = 0x4b;

// Instruction Decoding and Retirement
/// `PMC6_INST_RETIRED`.
pub const PMC6_INST_RETIRED: u32 = 0xc0;
/// `PMC6_UOPS_RETIRED`.
pub const PMC6_UOPS_RETIRED: u32 = 0xc2;
/// `PMC6_INST_DECODED`.
pub const PMC6_INST_DECODED: u32 = 0xd0;
/// `PMC6_EMON_KNI_INST_RETIRED`.
pub const PMC6_EMON_KNI_INST_RETIRED: u32 = 0xd8;
/// `PMC6_EMON_KNI_COMP_INST_RET`.
pub const PMC6_EMON_KNI_COMP_INST_RET: u32 = 0xd9;

// Interrupts
/// `PMC6_HW_INT_RX`.
pub const PMC6_HW_INT_RX: u32 = 0xc8;
/// `PMC6_CYCLES_INT_MASKED`.
pub const PMC6_CYCLES_INT_MASKED: u32 = 0xc6;
/// `PMC6_CYCLES_INT_PENDING_AND_MASKED`.
pub const PMC6_CYCLES_INT_PENDING_AND_MASKED: u32 = 0xc7;

// Branches
/// `PMC6_BR_INST_RETIRED`.
pub const PMC6_BR_INST_RETIRED: u32 = 0xc4;
/// `PMC6_BR_MISS_PRED_RETIRED`.
pub const PMC6_BR_MISS_PRED_RETIRED: u32 = 0xc5;
/// `PMC6_BR_TAKEN_RETIRED`.
pub const PMC6_BR_TAKEN_RETIRED: u32 = 0xc9;
/// `PMC6_BR_MISS_PRED_TAKEN_RET`.
pub const PMC6_BR_MISS_PRED_TAKEN_RET: u32 = 0xca;
/// `PMC6_BR_INST_DECODED`.
pub const PMC6_BR_INST_DECODED: u32 = 0xe0;
/// `PMC6_BTB_MISSES`.
pub const PMC6_BTB_MISSES: u32 = 0xe2;
/// `PMC6_BR_BOGUS`.
pub const PMC6_BR_BOGUS: u32 = 0xe4;
/// `PMC6_BACLEARS`.
pub const PMC6_BACLEARS: u32 = 0xe6;

// Stalls
/// `PMC6_RESOURCE_STALLS`.
pub const PMC6_RESOURCE_STALLS: u32 = 0xa2;
/// `PMC6_PARTIAL_RAT_STALLS`.
pub const PMC6_PARTIAL_RAT_STALLS: u32 = 0xd2;

// Segment Register Loads
/// `PMC6_SEGMENT_REG_LOADS`.
pub const PMC6_SEGMENT_REG_LOADS: u32 = 0x06;

// Clocks
/// `PMC6_CPU_CLK_UNHALTED`.
pub const PMC6_CPU_CLK_UNHALTED: u32 = 0x79;

// MMX Unit
/// `PMC6_MMX_INSTR_EXEC`: Celeron, P-II, P-IIX only.
pub const PMC6_MMX_INSTR_EXEC: u32 = 0xb0;
/// `PMC6_MMX_SAT_INSTR_EXEC`: P-II and P-III only.
pub const PMC6_MMX_SAT_INSTR_EXEC: u32 = 0xb1;
/// `PMC6_MMX_UOPS_EXEC`: P-II and P-III only.
pub const PMC6_MMX_UOPS_EXEC: u32 = 0xb2;
/// `PMC6_MMX_INSTR_TYPE_EXEC`: P-II and P-III only.
pub const PMC6_MMX_INSTR_TYPE_EXEC: u32 = 0xb3;
/// `PMC6_FP_MMX_TRANS`: P-II and P-III only.
pub const PMC6_FP_MMX_TRANS: u32 = 0xcc;
/// `PMC6_MMX_ASSIST`: P-II and P-III only.
pub const PMC6_MMX_ASSIST: u32 = 0xcd;
/// `PMC6_MMX_INSTR_RET`: P-II only.
pub const PMC6_MMX_INSTR_RET: u32 = 0xc3;

// Segment Register Renaming
/// `PMC6_SEG_RENAME_STALLS`: P-II and P-III only.
pub const PMC6_SEG_RENAME_STALLS: u32 = 0xd4;
/// `PMC6_SEG_REG_RENAMES`: P-II and P-III only.
pub const PMC6_SEG_REG_RENAMES: u32 = 0xd5;
/// `PMC6_RET_SEG_RENAMES`: P-II and P-III only.
pub const PMC6_RET_SEG_RENAMES: u32 = 0xd6;

// AMD K7 Event Selector MSR format.

/// `K7_EVTSEL_EVENT`.
pub const K7_EVTSEL_EVENT: u32 = 0x0000_00ff;
/// `K7_EVTSEL_UNIT`.
pub const K7_EVTSEL_UNIT: u32 = 0x0000_ff00;
/// `K7_EVTSEL_UNIT_SHIFT`.
pub const K7_EVTSEL_UNIT_SHIFT: u32 = 8;
/// `K7_EVTSEL_USR`.
pub const K7_EVTSEL_USR: u32 = 1 << 16;
/// `K7_EVTSEL_OS`.
pub const K7_EVTSEL_OS: u32 = 1 << 17;
/// `K7_EVTSEL_E`.
pub const K7_EVTSEL_E: u32 = 1 << 18;
/// `K7_EVTSEL_PC`.
pub const K7_EVTSEL_PC: u32 = 1 << 19;
/// `K7_EVTSEL_INT`.
pub const K7_EVTSEL_INT: u32 = 1 << 20;
/// `K7_EVTSEL_EN`.
pub const K7_EVTSEL_EN: u32 = 1 << 22;
/// `K7_EVTSEL_INV`.
pub const K7_EVTSEL_INV: u32 = 1 << 23;
/// `K7_EVTSEL_COUNTER_MASK`.
pub const K7_EVTSEL_COUNTER_MASK: u32 = 0xff00_0000;
/// `K7_EVTSEL_COUNTER_MASK_SHIFT`.
pub const K7_EVTSEL_COUNTER_MASK_SHIFT: u32 = 24;

// Segment Register Loads
/// `K7_SEGMENT_REG_LOADS`.
pub const K7_SEGMENT_REG_LOADS: u32 = 0x20;

/// `K7_STORES_TO_ACTIVE_INST_STREAM`.
pub const K7_STORES_TO_ACTIVE_INST_STREAM: u32 = 0x21;

// Data Cache Unit
/// `K7_DATA_CACHE_ACCESS`.
pub const K7_DATA_CACHE_ACCESS: u32 = 0x40;
/// `K7_DATA_CACHE_MISS`.
pub const K7_DATA_CACHE_MISS: u32 = 0x41;
/// `K7_DATA_CACHE_REFILL`.
pub const K7_DATA_CACHE_REFILL: u32 = 0x42;
/// `K7_DATA_CACHE_REFILL_SYSTEM`.
pub const K7_DATA_CACHE_REFILL_SYSTEM: u32 = 0x43;
/// `K7_DATA_CACHE_WBACK`.
pub const K7_DATA_CACHE_WBACK: u32 = 0x44;
/// `K7_L2_DTLB_HIT`.
pub const K7_L2_DTLB_HIT: u32 = 0x45;
/// `K7_L2_DTLB_MISS`.
pub const K7_L2_DTLB_MISS: u32 = 0x46;
/// `K7_MISALIGNED_DATA_REF`.
pub const K7_MISALIGNED_DATA_REF: u32 = 0x47;
/// `K7_SYSTEM_REQUEST`.
pub const K7_SYSTEM_REQUEST: u32 = 0x64;
/// `K7_SYSTEM_REQUEST_TYPE`.
pub const K7_SYSTEM_REQUEST_TYPE: u32 = 0x65;

/// `K7_SNOOP_HIT`.
pub const K7_SNOOP_HIT: u32 = 0x73;
/// `K7_SINGLE_BIT_ECC_ERROR`.
pub const K7_SINGLE_BIT_ECC_ERROR: u32 = 0x74;
/// `K7_CACHE_LINE_INVAL`.
pub const K7_CACHE_LINE_INVAL: u32 = 0x75;
/// `K7_CYCLES_PROCESSOR_IS_RUNNING`.
pub const K7_CYCLES_PROCESSOR_IS_RUNNING: u32 = 0x76;
/// `K7_L2_REQUEST`.
pub const K7_L2_REQUEST: u32 = 0x79;
/// `K7_L2_REQUEST_BUSY`.
pub const K7_L2_REQUEST_BUSY: u32 = 0x7a;

// Instruction Fetch Unit
/// `K7_IFU_IFETCH`.
pub const K7_IFU_IFETCH: u32 = 0x80;
/// `K7_IFU_IFETCH_MISS`.
pub const K7_IFU_IFETCH_MISS: u32 = 0x81;
/// `K7_IFU_REFILL_FROM_L2`.
pub const K7_IFU_REFILL_FROM_L2: u32 = 0x82;
/// `K7_IFU_REFILL_FROM_SYSTEM`.
pub const K7_IFU_REFILL_FROM_SYSTEM: u32 = 0x83;
/// `K7_ITLB_L1_MISS`.
pub const K7_ITLB_L1_MISS: u32 = 0x84;
/// `K7_ITLB_L2_MISS`.
pub const K7_ITLB_L2_MISS: u32 = 0x85;
/// `K7_SNOOP_RESYNC`.
pub const K7_SNOOP_RESYNC: u32 = 0x86;
/// `K7_IFU_STALL`.
pub const K7_IFU_STALL: u32 = 0x87;

/// `K7_RETURN_STACK_HITS`.
pub const K7_RETURN_STACK_HITS: u32 = 0x88;
/// `K7_RETURN_STACK_OVERFLOW`.
pub const K7_RETURN_STACK_OVERFLOW: u32 = 0x89;

// Retired
/// `K7_RETIRED_INST`.
pub const K7_RETIRED_INST: u32 = 0xc0;
/// `K7_RETIRED_OPS`.
pub const K7_RETIRED_OPS: u32 = 0xc1;
/// `K7_RETIRED_BRANCHES`.
pub const K7_RETIRED_BRANCHES: u32 = 0xc2;
/// `K7_RETIRED_BRANCH_MISPREDICTED`.
pub const K7_RETIRED_BRANCH_MISPREDICTED: u32 = 0xc3;
/// `K7_RETIRED_TAKEN_BRANCH`.
pub const K7_RETIRED_TAKEN_BRANCH: u32 = 0xc4;
/// `K7_RETIRED_TAKEN_BRANCH_MISPREDICTED`.
pub const K7_RETIRED_TAKEN_BRANCH_MISPREDICTED: u32 = 0xc5;
/// `K7_RETIRED_FAR_CONTROL_TRANSFER`.
pub const K7_RETIRED_FAR_CONTROL_TRANSFER: u32 = 0xc6;
/// `K7_RETIRED_RESYNC_BRANCH`.
pub const K7_RETIRED_RESYNC_BRANCH: u32 = 0xc7;
/// `K7_RETIRED_NEAR_RETURNS`.
pub const K7_RETIRED_NEAR_RETURNS: u32 = 0xc8;
/// `K7_RETIRED_NEAR_RETURNS_MISPREDICTED`.
pub const K7_RETIRED_NEAR_RETURNS_MISPREDICTED: u32 = 0xc9;
/// `K7_RETIRED_INDIRECT_MISPREDICTED`.
pub const K7_RETIRED_INDIRECT_MISPREDICTED: u32 = 0xca;

// Interrupts
/// `K7_CYCLES_INT_MASKED`.
pub const K7_CYCLES_INT_MASKED: u32 = 0xcd;
/// `K7_CYCLES_INT_PENDING_AND_MASKED`.
pub const K7_CYCLES_INT_PENDING_AND_MASKED: u32 = 0xce;
/// `K7_HW_INTR_RECV`.
pub const K7_HW_INTR_RECV: u32 = 0xcf;

/// `K7_INSTRUCTION_DECODER_EMPTY`.
pub const K7_INSTRUCTION_DECODER_EMPTY: u32 = 0xd0;
/// `K7_DISPATCH_STALLS`.
pub const K7_DISPATCH_STALLS: u32 = 0xd1;
/// `K7_BRANCH_ABORTS_TO_RETIRE`.
pub const K7_BRANCH_ABORTS_TO_RETIRE: u32 = 0xd2;
/// `K7_SERIALIZE`.
pub const K7_SERIALIZE: u32 = 0xd3;
/// `K7_SEGMENT_LOAD_STALL`.
pub const K7_SEGMENT_LOAD_STALL: u32 = 0xd4;
/// `K7_ICU_FULL`.
pub const K7_ICU_FULL: u32 = 0xd5;
/// `K7_RESERVATION_STATIONS_FULL`.
pub const K7_RESERVATION_STATIONS_FULL: u32 = 0xd6;
/// `K7_FPU_FULL`.
pub const K7_FPU_FULL: u32 = 0xd7;
/// `K7_LS_FULL`.
pub const K7_LS_FULL: u32 = 0xd8;
/// `K7_ALL_QUIET_STALL`.
pub const K7_ALL_QUIET_STALL: u32 = 0xd9;
/// `K7_FAR_TRANSFER_OR_RESYNC_BRANCH_PENDING`.
pub const K7_FAR_TRANSFER_OR_RESYNC_BRANCH_PENDING: u32 = 0xda;

/// `K7_BP0_MATCH`.
pub const K7_BP0_MATCH: u32 = 0xdc;
/// `K7_BP1_MATCH`.
pub const K7_BP1_MATCH: u32 = 0xdd;
/// `K7_BP2_MATCH`.
pub const K7_BP2_MATCH: u32 = 0xde;
/// `K7_BP3_MATCH`.
pub const K7_BP3_MATCH: u32 = 0xdf;

// VIA C3 crypto featureset: for amd64_has_xcrypt
/// `C3_HAS_AES`: cpu has AES.
pub const C3_HAS_AES: i32 = 1;
/// `C3_HAS_SHA`: cpu has SHA1 & SHA256.
pub const C3_HAS_SHA: i32 = 2;
/// `C3_HAS_MM`: cpu has RSA instructions.
pub const C3_HAS_MM: i32 = 4;
/// `C3_HAS_AESCTR`: cpu has AES-CTR instructions.
pub const C3_HAS_AESCTR: i32 = 8;

// Centaur Extended Feature flags
/// `C3_CPUID_HAS_RNG`.
pub const C3_CPUID_HAS_RNG: u32 = 0x0000_0004;
/// `C3_CPUID_DO_RNG`.
pub const C3_CPUID_DO_RNG: u32 = 0x0000_0008;
/// `C3_CPUID_HAS_ACE`.
pub const C3_CPUID_HAS_ACE: u32 = 0x0000_0040;
/// `C3_CPUID_DO_ACE`.
pub const C3_CPUID_DO_ACE: u32 = 0x0000_0080;
/// `C3_CPUID_HAS_ACE2`.
pub const C3_CPUID_HAS_ACE2: u32 = 0x0000_0100;
/// `C3_CPUID_DO_ACE2`.
pub const C3_CPUID_DO_ACE2: u32 = 0x0000_0200;
/// `C3_CPUID_HAS_PHE`.
pub const C3_CPUID_HAS_PHE: u32 = 0x0000_0400;
/// `C3_CPUID_DO_PHE`.
pub const C3_CPUID_DO_PHE: u32 = 0x0000_0800;
/// `C3_CPUID_HAS_PMM`.
pub const C3_CPUID_HAS_PMM: u32 = 0x0000_1000;
/// `C3_CPUID_DO_PMM`.
pub const C3_CPUID_DO_PMM: u32 = 0x0000_2000;

// VIA C3 xcrypt-* instruction context control options
/// `C3_CRYPT_CWLO_ROUND_M`.
pub const C3_CRYPT_CWLO_ROUND_M: u32 = 0x0000_000f;
/// `C3_CRYPT_CWLO_ALG_M`.
pub const C3_CRYPT_CWLO_ALG_M: u32 = 0x0000_0070;
/// `C3_CRYPT_CWLO_ALG_AES`.
pub const C3_CRYPT_CWLO_ALG_AES: u32 = 0x0000_0000;
/// `C3_CRYPT_CWLO_KEYGEN_M`.
pub const C3_CRYPT_CWLO_KEYGEN_M: u32 = 0x0000_0080;
/// `C3_CRYPT_CWLO_KEYGEN_HW`.
pub const C3_CRYPT_CWLO_KEYGEN_HW: u32 = 0x0000_0000;
/// `C3_CRYPT_CWLO_KEYGEN_SW`.
pub const C3_CRYPT_CWLO_KEYGEN_SW: u32 = 0x0000_0080;
/// `C3_CRYPT_CWLO_NORMAL`.
pub const C3_CRYPT_CWLO_NORMAL: u32 = 0x0000_0000;
/// `C3_CRYPT_CWLO_INTERMEDIATE`.
pub const C3_CRYPT_CWLO_INTERMEDIATE: u32 = 0x0000_0100;
/// `C3_CRYPT_CWLO_ENCRYPT`.
pub const C3_CRYPT_CWLO_ENCRYPT: u32 = 0x0000_0000;
/// `C3_CRYPT_CWLO_DECRYPT`.
pub const C3_CRYPT_CWLO_DECRYPT: u32 = 0x0000_0200;
/// `C3_CRYPT_CWLO_KEY128`: 128bit, 10 rds.
pub const C3_CRYPT_CWLO_KEY128: u32 = 0x0000_000a;
/// `C3_CRYPT_CWLO_KEY192`: 192bit, 12 rds.
pub const C3_CRYPT_CWLO_KEY192: u32 = 0x0000_040c;
/// `C3_CRYPT_CWLO_KEY256`: 256bit, 15 rds.
pub const C3_CRYPT_CWLO_KEY256: u32 = 0x0000_080e;

// Intel Silicon Debug
/// `IA32_DEBUG_INTERFACE`.
pub const IA32_DEBUG_INTERFACE: u32 = 0xc80;
/// `IA32_DEBUG_INTERFACE_ENABLE`.
pub const IA32_DEBUG_INTERFACE_ENABLE: u64 = 0x0000_0001;
/// `IA32_DEBUG_INTERFACE_LOCK`.
pub const IA32_DEBUG_INTERFACE_LOCK: u64 = 0x4000_0000;
/// `IA32_DEBUG_INTERFACE_MASK`.
pub const IA32_DEBUG_INTERFACE_MASK: u64 = 0x8000_0000;

// VMX
/// `IA32_FEATURE_CONTROL_LOCK`.
pub const IA32_FEATURE_CONTROL_LOCK: u64 = 0x01;
/// `IA32_FEATURE_CONTROL_SMX_EN`.
pub const IA32_FEATURE_CONTROL_SMX_EN: u64 = 0x02;
/// `IA32_FEATURE_CONTROL_VMX_EN`.
pub const IA32_FEATURE_CONTROL_VMX_EN: u64 = 0x04;
/// `IA32_FEATURE_CONTROL_SENTER_EN`.
pub const IA32_FEATURE_CONTROL_SENTER_EN: u64 = 1 << 15;
/// `IA32_FEATURE_CONTROL_SENTER_PARAM_MASK`.
pub const IA32_FEATURE_CONTROL_SENTER_PARAM_MASK: u64 = 0x7f00;
/// `IA32_VMX_BASIC`.
pub const IA32_VMX_BASIC: u32 = 0x480;
/// `IA32_VMX_PINBASED_CTLS`.
pub const IA32_VMX_PINBASED_CTLS: u32 = 0x481;
/// `IA32_VMX_PROCBASED_CTLS`.
pub const IA32_VMX_PROCBASED_CTLS: u32 = 0x482;
/// `IA32_VMX_EXIT_CTLS`.
pub const IA32_VMX_EXIT_CTLS: u32 = 0x483;
/// `IA32_VMX_ENTRY_CTLS`.
pub const IA32_VMX_ENTRY_CTLS: u32 = 0x484;
/// `IA32_VMX_MISC`.
pub const IA32_VMX_MISC: u32 = 0x485;
/// `IA32_VMX_CR0_FIXED0`.
pub const IA32_VMX_CR0_FIXED0: u32 = 0x486;
/// `IA32_VMX_CR0_FIXED1`.
pub const IA32_VMX_CR0_FIXED1: u32 = 0x487;
/// `IA32_VMX_CR4_FIXED0`.
pub const IA32_VMX_CR4_FIXED0: u32 = 0x488;
/// `IA32_VMX_CR4_FIXED1`.
pub const IA32_VMX_CR4_FIXED1: u32 = 0x489;
/// `IA32_VMX_PROCBASED2_CTLS`.
pub const IA32_VMX_PROCBASED2_CTLS: u32 = 0x48b;
/// `IA32_VMX_EPT_VPID_CAP`.
pub const IA32_VMX_EPT_VPID_CAP: u32 = 0x48c;
/// `IA32_VMX_TRUE_PINBASED_CTLS`.
pub const IA32_VMX_TRUE_PINBASED_CTLS: u32 = 0x48d;
/// `IA32_VMX_TRUE_PROCBASED_CTLS`.
pub const IA32_VMX_TRUE_PROCBASED_CTLS: u32 = 0x48e;
/// `IA32_VMX_TRUE_EXIT_CTLS`.
pub const IA32_VMX_TRUE_EXIT_CTLS: u32 = 0x48f;
/// `IA32_VMX_TRUE_ENTRY_CTLS`.
pub const IA32_VMX_TRUE_ENTRY_CTLS: u32 = 0x490;
/// `IA32_VMX_VMFUNC`.
pub const IA32_VMX_VMFUNC: u32 = 0x491;

/// `IA32_EPT_VPID_CAP_XO_TRANSLATIONS`.
pub const IA32_EPT_VPID_CAP_XO_TRANSLATIONS: u64 = 1;
/// `IA32_EPT_VPID_CAP_PAGE_WALK_4`.
pub const IA32_EPT_VPID_CAP_PAGE_WALK_4: u64 = 1 << 6;
/// `IA32_EPT_VPID_CAP_WB`.
pub const IA32_EPT_VPID_CAP_WB: u64 = 1 << 14;
/// `IA32_EPT_VPID_CAP_AD_BITS`.
pub const IA32_EPT_VPID_CAP_AD_BITS: u64 = 1 << 21;
/// `IA32_EPT_VPID_CAP_INVEPT_CONTEXT`.
pub const IA32_EPT_VPID_CAP_INVEPT_CONTEXT: u64 = 1 << 25;
/// `IA32_EPT_VPID_CAP_INVEPT_ALL`.
pub const IA32_EPT_VPID_CAP_INVEPT_ALL: u64 = 1 << 26;

/// `IA32_EPT_PAGING_CACHE_TYPE_UC`.
pub const IA32_EPT_PAGING_CACHE_TYPE_UC: u64 = 0x0;
/// `IA32_EPT_PAGING_CACHE_TYPE_WB`.
pub const IA32_EPT_PAGING_CACHE_TYPE_WB: u64 = 0x6;
/// `IA32_EPT_AD_BITS_ENABLE`.
pub const IA32_EPT_AD_BITS_ENABLE: u64 = 1 << 6;
/// `IA32_EPT_PAGE_WALK_LENGTH`.
pub const IA32_EPT_PAGE_WALK_LENGTH: u64 = 0x4;

// VMX : IA32_VMX_BASIC bits
/// `IA32_VMX_TRUE_CTLS_AVAIL`.
pub const IA32_VMX_TRUE_CTLS_AVAIL: u64 = 1 << 55;

// VMX : IA32_VMX_PINBASED_CTLS bits
/// `IA32_VMX_EXTERNAL_INT_EXITING`.
pub const IA32_VMX_EXTERNAL_INT_EXITING: u64 = 1;
/// `IA32_VMX_NMI_EXITING`.
pub const IA32_VMX_NMI_EXITING: u64 = 1 << 3;
/// `IA32_VMX_VIRTUAL_NMIS`.
pub const IA32_VMX_VIRTUAL_NMIS: u64 = 1 << 5;
/// `IA32_VMX_ACTIVATE_VMX_PREEMPTION_TIMER`.
pub const IA32_VMX_ACTIVATE_VMX_PREEMPTION_TIMER: u64 = 1 << 6;
/// `IA32_VMX_PROCESS_POSTED_INTERRUPTS`.
pub const IA32_VMX_PROCESS_POSTED_INTERRUPTS: u64 = 1 << 7;

// VMX : IA32_VMX_PROCBASED_CTLS bits
/// `IA32_VMX_INTERRUPT_WINDOW_EXITING`.
pub const IA32_VMX_INTERRUPT_WINDOW_EXITING: u64 = 1 << 2;
/// `IA32_VMX_USE_TSC_OFFSETTING`.
pub const IA32_VMX_USE_TSC_OFFSETTING: u64 = 1 << 3;
/// `IA32_VMX_HLT_EXITING`.
pub const IA32_VMX_HLT_EXITING: u64 = 1 << 7;
/// `IA32_VMX_INVLPG_EXITING`.
pub const IA32_VMX_INVLPG_EXITING: u64 = 1 << 9;
/// `IA32_VMX_MWAIT_EXITING`.
pub const IA32_VMX_MWAIT_EXITING: u64 = 1 << 10;
/// `IA32_VMX_RDPMC_EXITING`.
pub const IA32_VMX_RDPMC_EXITING: u64 = 1 << 11;
/// `IA32_VMX_RDTSC_EXITING`.
pub const IA32_VMX_RDTSC_EXITING: u64 = 1 << 12;
/// `IA32_VMX_CR3_LOAD_EXITING`.
pub const IA32_VMX_CR3_LOAD_EXITING: u64 = 1 << 15;
/// `IA32_VMX_CR3_STORE_EXITING`.
pub const IA32_VMX_CR3_STORE_EXITING: u64 = 1 << 16;
/// `IA32_VMX_CR8_LOAD_EXITING`.
pub const IA32_VMX_CR8_LOAD_EXITING: u64 = 1 << 19;
/// `IA32_VMX_CR8_STORE_EXITING`.
pub const IA32_VMX_CR8_STORE_EXITING: u64 = 1 << 20;
/// `IA32_VMX_USE_TPR_SHADOW`.
pub const IA32_VMX_USE_TPR_SHADOW: u64 = 1 << 21;
/// `IA32_VMX_NMI_WINDOW_EXITING`.
pub const IA32_VMX_NMI_WINDOW_EXITING: u64 = 1 << 22;
/// `IA32_VMX_MOV_DR_EXITING`.
pub const IA32_VMX_MOV_DR_EXITING: u64 = 1 << 23;
/// `IA32_VMX_UNCONDITIONAL_IO_EXITING`.
pub const IA32_VMX_UNCONDITIONAL_IO_EXITING: u64 = 1 << 24;
/// `IA32_VMX_USE_IO_BITMAPS`.
pub const IA32_VMX_USE_IO_BITMAPS: u64 = 1 << 25;
/// `IA32_VMX_MONITOR_TRAP_FLAG`.
pub const IA32_VMX_MONITOR_TRAP_FLAG: u64 = 1 << 27;
/// `IA32_VMX_USE_MSR_BITMAPS`.
pub const IA32_VMX_USE_MSR_BITMAPS: u64 = 1 << 28;
/// `IA32_VMX_MONITOR_EXITING`.
pub const IA32_VMX_MONITOR_EXITING: u64 = 1 << 29;
/// `IA32_VMX_PAUSE_EXITING`.
pub const IA32_VMX_PAUSE_EXITING: u64 = 1 << 30;
/// `IA32_VMX_ACTIVATE_SECONDARY_CONTROLS`.
pub const IA32_VMX_ACTIVATE_SECONDARY_CONTROLS: u64 = 1 << 31;

// VMX : IA32_VMX_PROCBASED2_CTLS bits
/// `IA32_VMX_VIRTUALIZE_APIC`.
pub const IA32_VMX_VIRTUALIZE_APIC: u64 = 1;
/// `IA32_VMX_ENABLE_EPT`.
pub const IA32_VMX_ENABLE_EPT: u64 = 1 << 1;
/// `IA32_VMX_DESCRIPTOR_TABLE_EXITING`.
pub const IA32_VMX_DESCRIPTOR_TABLE_EXITING: u64 = 1 << 2;
/// `IA32_VMX_ENABLE_RDTSCP`.
pub const IA32_VMX_ENABLE_RDTSCP: u64 = 1 << 3;
/// `IA32_VMX_VIRTUALIZE_X2APIC_MODE`.
pub const IA32_VMX_VIRTUALIZE_X2APIC_MODE: u64 = 1 << 4;
/// `IA32_VMX_ENABLE_VPID`.
pub const IA32_VMX_ENABLE_VPID: u64 = 1 << 5;
/// `IA32_VMX_WBINVD_EXITING`.
pub const IA32_VMX_WBINVD_EXITING: u64 = 1 << 6;
/// `IA32_VMX_UNRESTRICTED_GUEST`.
pub const IA32_VMX_UNRESTRICTED_GUEST: u64 = 1 << 7;
/// `IA32_VMX_APIC_REGISTER_VIRTUALIZATION`.
pub const IA32_VMX_APIC_REGISTER_VIRTUALIZATION: u64 = 1 << 8;
/// `IA32_VMX_VIRTUAL_INTERRUPT_DELIVERY`.
pub const IA32_VMX_VIRTUAL_INTERRUPT_DELIVERY: u64 = 1 << 9;
/// `IA32_VMX_PAUSE_LOOP_EXITING`.
pub const IA32_VMX_PAUSE_LOOP_EXITING: u64 = 1 << 10;
/// `IA32_VMX_RDRAND_EXITING`.
pub const IA32_VMX_RDRAND_EXITING: u64 = 1 << 11;
/// `IA32_VMX_ENABLE_INVPCID`.
pub const IA32_VMX_ENABLE_INVPCID: u64 = 1 << 12;
/// `IA32_VMX_ENABLE_VM_FUNCTIONS`.
pub const IA32_VMX_ENABLE_VM_FUNCTIONS: u64 = 1 << 13;
/// `IA32_VMX_VMCS_SHADOWING`.
pub const IA32_VMX_VMCS_SHADOWING: u64 = 1 << 14;
/// `IA32_VMX_ENABLE_ENCLS_EXITING`.
pub const IA32_VMX_ENABLE_ENCLS_EXITING: u64 = 1 << 15;
/// `IA32_VMX_RDSEED_EXITING`.
pub const IA32_VMX_RDSEED_EXITING: u64 = 1 << 16;
/// `IA32_VMX_ENABLE_PML`.
pub const IA32_VMX_ENABLE_PML: u64 = 1 << 17;
/// `IA32_VMX_EPT_VIOLATION_VE`.
pub const IA32_VMX_EPT_VIOLATION_VE: u64 = 1 << 18;
/// `IA32_VMX_CONCEAL_VMX_FROM_PT`.
pub const IA32_VMX_CONCEAL_VMX_FROM_PT: u64 = 1 << 19;
/// `IA32_VMX_ENABLE_XSAVES_XRSTORS`.
pub const IA32_VMX_ENABLE_XSAVES_XRSTORS: u64 = 1 << 20;
/// `IA32_VMX_ENABLE_TSC_SCALING`.
pub const IA32_VMX_ENABLE_TSC_SCALING: u64 = 1 << 25;

// VMX : IA32_VMX_EXIT_CTLS bits
/// `IA32_VMX_SAVE_DEBUG_CONTROLS`.
pub const IA32_VMX_SAVE_DEBUG_CONTROLS: u64 = 1 << 2;
/// `IA32_VMX_HOST_SPACE_ADDRESS_SIZE`.
pub const IA32_VMX_HOST_SPACE_ADDRESS_SIZE: u64 = 1 << 9;
/// `IA32_VMX_LOAD_IA32_PERF_GLOBAL_CTRL_ON_EXIT`.
pub const IA32_VMX_LOAD_IA32_PERF_GLOBAL_CTRL_ON_EXIT: u64 = 1 << 12;
/// `IA32_VMX_ACKNOWLEDGE_INTERRUPT_ON_EXIT`.
pub const IA32_VMX_ACKNOWLEDGE_INTERRUPT_ON_EXIT: u64 = 1 << 15;
/// `IA32_VMX_SAVE_IA32_PAT_ON_EXIT`.
pub const IA32_VMX_SAVE_IA32_PAT_ON_EXIT: u64 = 1 << 18;
/// `IA32_VMX_LOAD_IA32_PAT_ON_EXIT`.
pub const IA32_VMX_LOAD_IA32_PAT_ON_EXIT: u64 = 1 << 19;
/// `IA32_VMX_SAVE_IA32_EFER_ON_EXIT`.
pub const IA32_VMX_SAVE_IA32_EFER_ON_EXIT: u64 = 1 << 20;
/// `IA32_VMX_LOAD_IA32_EFER_ON_EXIT`.
pub const IA32_VMX_LOAD_IA32_EFER_ON_EXIT: u64 = 1 << 21;
/// `IA32_VMX_SAVE_VMX_PREEMPTION_TIMER`.
pub const IA32_VMX_SAVE_VMX_PREEMPTION_TIMER: u64 = 1 << 22;
/// `IA32_VMX_CLEAR_IA32_BNDCFGS_ON_EXIT`.
pub const IA32_VMX_CLEAR_IA32_BNDCFGS_ON_EXIT: u64 = 1 << 23;
/// `IA32_VMX_CONCEAL_VM_EXITS_FROM_PT`.
pub const IA32_VMX_CONCEAL_VM_EXITS_FROM_PT: u64 = 1 << 24;
/// `IA32_VMX_LOAD_HOST_CET_STATE`.
pub const IA32_VMX_LOAD_HOST_CET_STATE: u64 = 1 << 28;

// VMX: IA32_VMX_ENTRY_CTLS bits
/// `IA32_VMX_LOAD_DEBUG_CONTROLS`.
pub const IA32_VMX_LOAD_DEBUG_CONTROLS: u64 = 1 << 2;
/// `IA32_VMX_IA32E_MODE_GUEST`.
pub const IA32_VMX_IA32E_MODE_GUEST: u64 = 1 << 9;
/// `IA32_VMX_ENTRY_TO_SMM`.
pub const IA32_VMX_ENTRY_TO_SMM: u64 = 1 << 10;
/// `IA32_VMX_DEACTIVATE_DUAL_MONITOR_TREATMENT`.
pub const IA32_VMX_DEACTIVATE_DUAL_MONITOR_TREATMENT: u64 = 1 << 11;
/// `IA32_VMX_LOAD_IA32_PERF_GLOBAL_CTRL_ON_ENTRY`.
pub const IA32_VMX_LOAD_IA32_PERF_GLOBAL_CTRL_ON_ENTRY: u64 = 1 << 13;
/// `IA32_VMX_LOAD_IA32_PAT_ON_ENTRY`.
pub const IA32_VMX_LOAD_IA32_PAT_ON_ENTRY: u64 = 1 << 14;
/// `IA32_VMX_LOAD_IA32_EFER_ON_ENTRY`.
pub const IA32_VMX_LOAD_IA32_EFER_ON_ENTRY: u64 = 1 << 15;
/// `IA32_VMX_LOAD_IA32_BNDCFGS_ON_ENTRY`.
pub const IA32_VMX_LOAD_IA32_BNDCFGS_ON_ENTRY: u64 = 1 << 16;
/// `IA32_VMX_CONCEAL_VM_ENTRIES_FROM_PT`.
pub const IA32_VMX_CONCEAL_VM_ENTRIES_FROM_PT: u64 = 1 << 17;
/// `IA32_VMX_LOAD_GUEST_CET_STATE`.
pub const IA32_VMX_LOAD_GUEST_CET_STATE: u64 = 1 << 20;

// 16-bit control fields
/// `VMCS_GUEST_VPID`.
pub const VMCS_GUEST_VPID: u64 = 0x0000;
/// `VMCS_POSTED_INT_NOTIF_VECTOR`.
pub const VMCS_POSTED_INT_NOTIF_VECTOR: u64 = 0x0002;
/// `VMCS_EPTP_INDEX`.
pub const VMCS_EPTP_INDEX: u64 = 0x0004;

// 16-bit guest state fields
/// `VMCS_GUEST_IA32_ES_SEL`.
pub const VMCS_GUEST_IA32_ES_SEL: u64 = 0x0800;
/// `VMCS_GUEST_IA32_CS_SEL`.
pub const VMCS_GUEST_IA32_CS_SEL: u64 = 0x0802;
/// `VMCS_GUEST_IA32_SS_SEL`.
pub const VMCS_GUEST_IA32_SS_SEL: u64 = 0x0804;
/// `VMCS_GUEST_IA32_DS_SEL`.
pub const VMCS_GUEST_IA32_DS_SEL: u64 = 0x0806;
/// `VMCS_GUEST_IA32_FS_SEL`.
pub const VMCS_GUEST_IA32_FS_SEL: u64 = 0x0808;
/// `VMCS_GUEST_IA32_GS_SEL`.
pub const VMCS_GUEST_IA32_GS_SEL: u64 = 0x080a;
/// `VMCS_GUEST_IA32_LDTR_SEL`.
pub const VMCS_GUEST_IA32_LDTR_SEL: u64 = 0x080c;
/// `VMCS_GUEST_IA32_TR_SEL`.
pub const VMCS_GUEST_IA32_TR_SEL: u64 = 0x080e;
/// `VMCS_GUEST_INTERRUPT_STATUS`.
pub const VMCS_GUEST_INTERRUPT_STATUS: u64 = 0x0810;
/// `VMCS_GUEST_PML_INDEX`.
pub const VMCS_GUEST_PML_INDEX: u64 = 0x0812;

// 16-bit host state fields
/// `VMCS_HOST_IA32_ES_SEL`.
pub const VMCS_HOST_IA32_ES_SEL: u64 = 0x0c00;
/// `VMCS_HOST_IA32_CS_SEL`.
pub const VMCS_HOST_IA32_CS_SEL: u64 = 0x0c02;
/// `VMCS_HOST_IA32_SS_SEL`.
pub const VMCS_HOST_IA32_SS_SEL: u64 = 0x0c04;
/// `VMCS_HOST_IA32_DS_SEL`.
pub const VMCS_HOST_IA32_DS_SEL: u64 = 0x0c06;
/// `VMCS_HOST_IA32_FS_SEL`.
pub const VMCS_HOST_IA32_FS_SEL: u64 = 0x0c08;
/// `VMCS_HOST_IA32_GS_SEL`.
pub const VMCS_HOST_IA32_GS_SEL: u64 = 0x0c0a;
/// `VMCS_HOST_IA32_TR_SEL`.
pub const VMCS_HOST_IA32_TR_SEL: u64 = 0x0c0c;

// 64-bit control fields
/// `VMCS_IO_BITMAP_A`.
pub const VMCS_IO_BITMAP_A: u64 = 0x2000;
/// `VMCS_IO_BITMAP_B`.
pub const VMCS_IO_BITMAP_B: u64 = 0x2002;
/// `VMCS_MSR_BITMAP_ADDRESS`.
pub const VMCS_MSR_BITMAP_ADDRESS: u64 = 0x2004;
/// `VMCS_EXIT_STORE_MSR_ADDRESS`.
pub const VMCS_EXIT_STORE_MSR_ADDRESS: u64 = 0x2006;
/// `VMCS_EXIT_LOAD_MSR_ADDRESS`.
pub const VMCS_EXIT_LOAD_MSR_ADDRESS: u64 = 0x2008;
/// `VMCS_ENTRY_LOAD_MSR_ADDRESS`.
pub const VMCS_ENTRY_LOAD_MSR_ADDRESS: u64 = 0x200a;
/// `VMCS_EXECUTIVE_VMCS_POINTER`.
pub const VMCS_EXECUTIVE_VMCS_POINTER: u64 = 0x200c;
/// `VMCS_PML_ADDRESS`.
pub const VMCS_PML_ADDRESS: u64 = 0x200e;
/// `VMCS_TSC_OFFSET`.
pub const VMCS_TSC_OFFSET: u64 = 0x2010;
/// `VMCS_VIRTUAL_APIC_ADDRESS`.
pub const VMCS_VIRTUAL_APIC_ADDRESS: u64 = 0x2012;
/// `VMCS_APIC_ACCESS_ADDRESS`.
pub const VMCS_APIC_ACCESS_ADDRESS: u64 = 0x2014;
/// `VMCS_POSTED_INTERRUPT_DESC`.
pub const VMCS_POSTED_INTERRUPT_DESC: u64 = 0x2016;
/// `VMCS_VM_FUNCTION_CONTROLS`.
pub const VMCS_VM_FUNCTION_CONTROLS: u64 = 0x2018;
/// `VMCS_GUEST_IA32_EPTP`.
pub const VMCS_GUEST_IA32_EPTP: u64 = 0x201a;
/// `VMCS_EOI_EXIT_BITMAP_0`.
pub const VMCS_EOI_EXIT_BITMAP_0: u64 = 0x201c;
/// `VMCS_EOI_EXIT_BITMAP_1`.
pub const VMCS_EOI_EXIT_BITMAP_1: u64 = 0x201e;
/// `VMCS_EOI_EXIT_BITMAP_2`.
pub const VMCS_EOI_EXIT_BITMAP_2: u64 = 0x2020;
/// `VMCS_EOI_EXIT_BITMAP_3`.
pub const VMCS_EOI_EXIT_BITMAP_3: u64 = 0x2022;
/// `VMCS_EPTP_LIST_ADDRESS`.
pub const VMCS_EPTP_LIST_ADDRESS: u64 = 0x2024;
/// `VMCS_VMREAD_BITMAP_ADDRESS`.
pub const VMCS_VMREAD_BITMAP_ADDRESS: u64 = 0x2026;
/// `VMCS_VMWRITE_BITMAP_ADDRESS`.
pub const VMCS_VMWRITE_BITMAP_ADDRESS: u64 = 0x2028;
/// `VMCS_VIRTUALIZATION_EXC_ADDRESS`.
pub const VMCS_VIRTUALIZATION_EXC_ADDRESS: u64 = 0x202a;
/// `VMCS_XSS_EXITING_BITMAP`.
pub const VMCS_XSS_EXITING_BITMAP: u64 = 0x202c;
/// `VMCS_ENCLS_EXITING_BITMAP`.
pub const VMCS_ENCLS_EXITING_BITMAP: u64 = 0x202e;
/// `VMCS_TSC_MULTIPLIER`.
pub const VMCS_TSC_MULTIPLIER: u64 = 0x2032;

// 64-bit RO data field
/// `VMCS_GUEST_PHYSICAL_ADDRESS`.
pub const VMCS_GUEST_PHYSICAL_ADDRESS: u64 = 0x2400;

// 64-bit guest state fields
/// `VMCS_LINK_POINTER`.
pub const VMCS_LINK_POINTER: u64 = 0x2800;
/// `VMCS_GUEST_IA32_DEBUGCTL`.
pub const VMCS_GUEST_IA32_DEBUGCTL: u64 = 0x2802;
/// `VMCS_GUEST_IA32_PAT`.
pub const VMCS_GUEST_IA32_PAT: u64 = 0x2804;
/// `VMCS_GUEST_IA32_EFER`.
pub const VMCS_GUEST_IA32_EFER: u64 = 0x2806;
/// `VMCS_GUEST_IA32_PERF_GBL_CTRL`.
pub const VMCS_GUEST_IA32_PERF_GBL_CTRL: u64 = 0x2808;
/// `VMCS_GUEST_PDPTE0`.
pub const VMCS_GUEST_PDPTE0: u64 = 0x280a;
/// `VMCS_GUEST_PDPTE1`.
pub const VMCS_GUEST_PDPTE1: u64 = 0x280c;
/// `VMCS_GUEST_PDPTE2`.
pub const VMCS_GUEST_PDPTE2: u64 = 0x280e;
/// `VMCS_GUEST_PDPTE3`.
pub const VMCS_GUEST_PDPTE3: u64 = 0x2810;
/// `VMCS_GUEST_IA32_BNDCFGS`.
pub const VMCS_GUEST_IA32_BNDCFGS: u64 = 0x2812;

// 64-bit host state fields
/// `VMCS_HOST_IA32_PAT`.
pub const VMCS_HOST_IA32_PAT: u64 = 0x2c00;
/// `VMCS_HOST_IA32_EFER`.
pub const VMCS_HOST_IA32_EFER: u64 = 0x2c02;
/// `VMCS_HOST_IA32_PERF_GBL_CTRL`.
pub const VMCS_HOST_IA32_PERF_GBL_CTRL: u64 = 0x2c04;

// 32-bit control fields
/// `VMCS_PINBASED_CTLS`.
pub const VMCS_PINBASED_CTLS: u64 = 0x4000;
/// `VMCS_PROCBASED_CTLS`.
pub const VMCS_PROCBASED_CTLS: u64 = 0x4002;
/// `VMCS_EXCEPTION_BITMAP`.
pub const VMCS_EXCEPTION_BITMAP: u64 = 0x4004;
/// `VMCS_PF_ERROR_CODE_MASK`.
pub const VMCS_PF_ERROR_CODE_MASK: u64 = 0x4006;
/// `VMCS_PF_ERROR_CODE_MATCH`.
pub const VMCS_PF_ERROR_CODE_MATCH: u64 = 0x4008;
/// `VMCS_CR3_TARGET_COUNT`.
pub const VMCS_CR3_TARGET_COUNT: u64 = 0x400a;
/// `VMCS_EXIT_CTLS`.
pub const VMCS_EXIT_CTLS: u64 = 0x400c;
/// `VMCS_EXIT_MSR_STORE_COUNT`.
pub const VMCS_EXIT_MSR_STORE_COUNT: u64 = 0x400e;
/// `VMCS_EXIT_MSR_LOAD_COUNT`.
pub const VMCS_EXIT_MSR_LOAD_COUNT: u64 = 0x4010;
/// `VMCS_ENTRY_CTLS`.
pub const VMCS_ENTRY_CTLS: u64 = 0x4012;
/// `VMCS_ENTRY_MSR_LOAD_COUNT`.
pub const VMCS_ENTRY_MSR_LOAD_COUNT: u64 = 0x4014;
/// `VMCS_ENTRY_INTERRUPTION_INFO`.
pub const VMCS_ENTRY_INTERRUPTION_INFO: u64 = 0x4016;
/// `VMCS_ENTRY_EXCEPTION_ERROR_CODE`.
pub const VMCS_ENTRY_EXCEPTION_ERROR_CODE: u64 = 0x4018;
/// `VMCS_ENTRY_INSTRUCTION_LENGTH`.
pub const VMCS_ENTRY_INSTRUCTION_LENGTH: u64 = 0x401a;
/// `VMCS_TPR_THRESHOLD`.
pub const VMCS_TPR_THRESHOLD: u64 = 0x401c;
/// `VMCS_PROCBASED2_CTLS`.
pub const VMCS_PROCBASED2_CTLS: u64 = 0x401e;
/// `VMCS_PLE_GAP`.
pub const VMCS_PLE_GAP: u64 = 0x4020;
/// `VMCS_PLE_WINDOW`.
pub const VMCS_PLE_WINDOW: u64 = 0x4022;

// 32-bit RO data fields
/// `VMCS_INSTRUCTION_ERROR`.
pub const VMCS_INSTRUCTION_ERROR: u64 = 0x4400;
/// `VMCS_EXIT_REASON`.
pub const VMCS_EXIT_REASON: u64 = 0x4402;
/// `VMCS_EXIT_INTERRUPTION_INFO`.
pub const VMCS_EXIT_INTERRUPTION_INFO: u64 = 0x4404;
/// `VMCS_EXIT_INTERRUPTION_ERR_CODE`.
pub const VMCS_EXIT_INTERRUPTION_ERR_CODE: u64 = 0x4406;
/// `VMCS_IDT_VECTORING_INFO`.
pub const VMCS_IDT_VECTORING_INFO: u64 = 0x4408;
/// `VMCS_IDT_VECTORING_ERROR_CODE`.
pub const VMCS_IDT_VECTORING_ERROR_CODE: u64 = 0x440a;
/// `VMCS_INSTRUCTION_LENGTH`.
pub const VMCS_INSTRUCTION_LENGTH: u64 = 0x440c;
/// `VMCS_EXIT_INSTRUCTION_INFO`.
pub const VMCS_EXIT_INSTRUCTION_INFO: u64 = 0x440e;

// 32-bit guest state fields
/// `VMCS_GUEST_IA32_ES_LIMIT`.
pub const VMCS_GUEST_IA32_ES_LIMIT: u64 = 0x4800;
/// `VMCS_GUEST_IA32_CS_LIMIT`.
pub const VMCS_GUEST_IA32_CS_LIMIT: u64 = 0x4802;
/// `VMCS_GUEST_IA32_SS_LIMIT`.
pub const VMCS_GUEST_IA32_SS_LIMIT: u64 = 0x4804;
/// `VMCS_GUEST_IA32_DS_LIMIT`.
pub const VMCS_GUEST_IA32_DS_LIMIT: u64 = 0x4806;
/// `VMCS_GUEST_IA32_FS_LIMIT`.
pub const VMCS_GUEST_IA32_FS_LIMIT: u64 = 0x4808;
/// `VMCS_GUEST_IA32_GS_LIMIT`.
pub const VMCS_GUEST_IA32_GS_LIMIT: u64 = 0x480a;
/// `VMCS_GUEST_IA32_LDTR_LIMIT`.
pub const VMCS_GUEST_IA32_LDTR_LIMIT: u64 = 0x480c;
/// `VMCS_GUEST_IA32_TR_LIMIT`.
pub const VMCS_GUEST_IA32_TR_LIMIT: u64 = 0x480e;
/// `VMCS_GUEST_IA32_GDTR_LIMIT`.
pub const VMCS_GUEST_IA32_GDTR_LIMIT: u64 = 0x4810;
/// `VMCS_GUEST_IA32_IDTR_LIMIT`.
pub const VMCS_GUEST_IA32_IDTR_LIMIT: u64 = 0x4812;
/// `VMCS_GUEST_IA32_ES_AR`.
pub const VMCS_GUEST_IA32_ES_AR: u64 = 0x4814;
/// `VMCS_GUEST_IA32_CS_AR`.
pub const VMCS_GUEST_IA32_CS_AR: u64 = 0x4816;
/// `VMCS_GUEST_IA32_SS_AR`.
pub const VMCS_GUEST_IA32_SS_AR: u64 = 0x4818;
/// `VMCS_GUEST_IA32_DS_AR`.
pub const VMCS_GUEST_IA32_DS_AR: u64 = 0x481a;
/// `VMCS_GUEST_IA32_FS_AR`.
pub const VMCS_GUEST_IA32_FS_AR: u64 = 0x481c;
/// `VMCS_GUEST_IA32_GS_AR`.
pub const VMCS_GUEST_IA32_GS_AR: u64 = 0x481e;
/// `VMCS_GUEST_IA32_LDTR_AR`.
pub const VMCS_GUEST_IA32_LDTR_AR: u64 = 0x4820;
/// `VMCS_GUEST_IA32_TR_AR`.
pub const VMCS_GUEST_IA32_TR_AR: u64 = 0x4822;
/// `VMCS_GUEST_INTERRUPTIBILITY_ST`.
pub const VMCS_GUEST_INTERRUPTIBILITY_ST: u64 = 0x4824;
/// `VMX_INT_STATE_BLOCK_STI`.
pub const VMX_INT_STATE_BLOCK_STI: u64 = 1;
/// `VMX_INT_STATE_BLOCK_MOVSS`.
pub const VMX_INT_STATE_BLOCK_MOVSS: u64 = 1 << 1;
/// `VMCS_GUEST_ACTIVITY_STATE`.
pub const VMCS_GUEST_ACTIVITY_STATE: u64 = 0x4826;
/// `VMCS_GUEST_SMBASE`.
pub const VMCS_GUEST_SMBASE: u64 = 0x4828;
/// `VMCS_GUEST_IA32_SYSENTER_CS`.
pub const VMCS_GUEST_IA32_SYSENTER_CS: u64 = 0x482a;
/// `VMCS_VMX_PREEMPTION_TIMER_VAL`.
pub const VMCS_VMX_PREEMPTION_TIMER_VAL: u64 = 0x482e;

// 32-bit host state field
/// `VMCS_HOST_IA32_SYSENTER_CS`.
pub const VMCS_HOST_IA32_SYSENTER_CS: u64 = 0x4c00;

// Natural-width control fields
/// `VMCS_CR0_MASK`.
pub const VMCS_CR0_MASK: u64 = 0x6000;
/// `VMCS_CR4_MASK`.
pub const VMCS_CR4_MASK: u64 = 0x6002;
/// `VMCS_CR0_READ_SHADOW`.
pub const VMCS_CR0_READ_SHADOW: u64 = 0x6004;
/// `VMCS_CR4_READ_SHADOW`.
pub const VMCS_CR4_READ_SHADOW: u64 = 0x6006;
/// `VMCS_CR3_TARGET_0`.
pub const VMCS_CR3_TARGET_0: u64 = 0x6008;
/// `VMCS_CR3_TARGET_1`.
pub const VMCS_CR3_TARGET_1: u64 = 0x600a;
/// `VMCS_CR3_TARGET_2`.
pub const VMCS_CR3_TARGET_2: u64 = 0x600c;
/// `VMCS_CR3_TARGET_3`.
pub const VMCS_CR3_TARGET_3: u64 = 0x600e;

// Natural-width RO fields
/// `VMCS_GUEST_EXIT_QUALIFICATION`.
pub const VMCS_GUEST_EXIT_QUALIFICATION: u64 = 0x6400;
/// `VMCS_IO_RCX`.
pub const VMCS_IO_RCX: u64 = 0x6402;
/// `VMCS_IO_RSI`.
pub const VMCS_IO_RSI: u64 = 0x6404;
/// `VMCS_IO_RDI`.
pub const VMCS_IO_RDI: u64 = 0x6406;
/// `VMCS_IO_RIP`.
pub const VMCS_IO_RIP: u64 = 0x6408;
/// `VMCS_GUEST_LINEAR_ADDRESS`.
pub const VMCS_GUEST_LINEAR_ADDRESS: u64 = 0x640a;

// Natural-width guest state fields
/// `VMCS_GUEST_IA32_CR0`.
pub const VMCS_GUEST_IA32_CR0: u64 = 0x6800;
/// `VMCS_GUEST_IA32_CR3`.
pub const VMCS_GUEST_IA32_CR3: u64 = 0x6802;
/// `VMCS_GUEST_IA32_CR4`.
pub const VMCS_GUEST_IA32_CR4: u64 = 0x6804;
/// `VMCS_GUEST_IA32_ES_BASE`.
pub const VMCS_GUEST_IA32_ES_BASE: u64 = 0x6806;
/// `VMCS_GUEST_IA32_CS_BASE`.
pub const VMCS_GUEST_IA32_CS_BASE: u64 = 0x6808;
/// `VMCS_GUEST_IA32_SS_BASE`.
pub const VMCS_GUEST_IA32_SS_BASE: u64 = 0x680a;
/// `VMCS_GUEST_IA32_DS_BASE`.
pub const VMCS_GUEST_IA32_DS_BASE: u64 = 0x680c;
/// `VMCS_GUEST_IA32_FS_BASE`.
pub const VMCS_GUEST_IA32_FS_BASE: u64 = 0x680e;
/// `VMCS_GUEST_IA32_GS_BASE`.
pub const VMCS_GUEST_IA32_GS_BASE: u64 = 0x6810;
/// `VMCS_GUEST_IA32_LDTR_BASE`.
pub const VMCS_GUEST_IA32_LDTR_BASE: u64 = 0x6812;
/// `VMCS_GUEST_IA32_TR_BASE`.
pub const VMCS_GUEST_IA32_TR_BASE: u64 = 0x6814;
/// `VMCS_GUEST_IA32_GDTR_BASE`.
pub const VMCS_GUEST_IA32_GDTR_BASE: u64 = 0x6816;
/// `VMCS_GUEST_IA32_IDTR_BASE`.
pub const VMCS_GUEST_IA32_IDTR_BASE: u64 = 0x6818;
/// `VMCS_GUEST_IA32_DR7`.
pub const VMCS_GUEST_IA32_DR7: u64 = 0x681a;
/// `VMCS_GUEST_IA32_RSP`.
pub const VMCS_GUEST_IA32_RSP: u64 = 0x681c;
/// `VMCS_GUEST_IA32_RIP`.
pub const VMCS_GUEST_IA32_RIP: u64 = 0x681e;
/// `VMCS_GUEST_IA32_RFLAGS`.
pub const VMCS_GUEST_IA32_RFLAGS: u64 = 0x6820;
/// `VMCS_GUEST_PENDING_DBG_EXC`.
pub const VMCS_GUEST_PENDING_DBG_EXC: u64 = 0x6822;
/// `VMCS_GUEST_IA32_SYSENTER_ESP`.
pub const VMCS_GUEST_IA32_SYSENTER_ESP: u64 = 0x6824;
/// `VMCS_GUEST_IA32_SYSENTER_EIP`.
pub const VMCS_GUEST_IA32_SYSENTER_EIP: u64 = 0x6826;
/// `VMCS_GUEST_IA32_S_CET`.
pub const VMCS_GUEST_IA32_S_CET: u64 = 0x6828;
/// `VMCS_GUEST_SSP`.
pub const VMCS_GUEST_SSP: u64 = 0x682a;
/// `VMCS_GUEST_IA32_INTR_SSP_TABLE`.
pub const VMCS_GUEST_IA32_INTR_SSP_TABLE: u64 = 0x682c;

// Natural-width host state fields
/// `VMCS_HOST_IA32_CR0`.
pub const VMCS_HOST_IA32_CR0: u64 = 0x6c00;
/// `VMCS_HOST_IA32_CR3`.
pub const VMCS_HOST_IA32_CR3: u64 = 0x6c02;
/// `VMCS_HOST_IA32_CR4`.
pub const VMCS_HOST_IA32_CR4: u64 = 0x6c04;
/// `VMCS_HOST_IA32_FS_BASE`.
pub const VMCS_HOST_IA32_FS_BASE: u64 = 0x6c06;
/// `VMCS_HOST_IA32_GS_BASE`.
pub const VMCS_HOST_IA32_GS_BASE: u64 = 0x6c08;
/// `VMCS_HOST_IA32_TR_BASE`.
pub const VMCS_HOST_IA32_TR_BASE: u64 = 0x6c0a;
/// `VMCS_HOST_IA32_GDTR_BASE`.
pub const VMCS_HOST_IA32_GDTR_BASE: u64 = 0x6c0c;
/// `VMCS_HOST_IA32_IDTR_BASE`.
pub const VMCS_HOST_IA32_IDTR_BASE: u64 = 0x6c0e;
/// `VMCS_HOST_IA32_SYSENTER_ESP`.
pub const VMCS_HOST_IA32_SYSENTER_ESP: u64 = 0x6c10;
/// `VMCS_HOST_IA32_SYSENTER_EIP`.
pub const VMCS_HOST_IA32_SYSENTER_EIP: u64 = 0x6c12;
/// `VMCS_HOST_IA32_RSP`.
pub const VMCS_HOST_IA32_RSP: u64 = 0x6c14;
/// `VMCS_HOST_IA32_RIP`.
pub const VMCS_HOST_IA32_RIP: u64 = 0x6c16;
/// `VMCS_HOST_IA32_S_CET`.
pub const VMCS_HOST_IA32_S_CET: u64 = 0x6c18;
/// `VMCS_HOST_SSP`.
pub const VMCS_HOST_SSP: u64 = 0x6c1a;
/// `VMCS_HOST_IA32_INTR_SSP_TABLE`.
pub const VMCS_HOST_IA32_INTR_SSP_TABLE: u64 = 0x6c1c;

/// `IA32_VMX_INVVPID_INDIV_ADDR_CTX`.
pub const IA32_VMX_INVVPID_INDIV_ADDR_CTX: u64 = 0x0;
/// `IA32_VMX_INVVPID_SINGLE_CTX`.
pub const IA32_VMX_INVVPID_SINGLE_CTX: u64 = 0x1;
/// `IA32_VMX_INVVPID_ALL_CTX`.
pub const IA32_VMX_INVVPID_ALL_CTX: u64 = 0x2;
/// `IA32_VMX_INVVPID_SINGLE_CTX_GLB`.
pub const IA32_VMX_INVVPID_SINGLE_CTX_GLB: u64 = 0x3;

/// `IA32_VMX_INVEPT_SINGLE_CTX`.
pub const IA32_VMX_INVEPT_SINGLE_CTX: u64 = 0x1;
/// `IA32_VMX_INVEPT_GLOBAL_CTX`.
pub const IA32_VMX_INVEPT_GLOBAL_CTX: u64 = 0x2;

/// `IA32_VMX_EPT_FAULT_READ`.
pub const IA32_VMX_EPT_FAULT_READ: u64 = 1;
/// `IA32_VMX_EPT_FAULT_WRITE`.
pub const IA32_VMX_EPT_FAULT_WRITE: u64 = 1 << 1;
/// `IA32_VMX_EPT_FAULT_EXEC`.
pub const IA32_VMX_EPT_FAULT_EXEC: u64 = 1 << 2;

/// `IA32_VMX_EPT_FAULT_WAS_READABLE`.
pub const IA32_VMX_EPT_FAULT_WAS_READABLE: u64 = 1 << 3;
/// `IA32_VMX_EPT_FAULT_WAS_WRITABLE`.
pub const IA32_VMX_EPT_FAULT_WAS_WRITABLE: u64 = 1 << 4;
/// `IA32_VMX_EPT_FAULT_WAS_EXECABLE`.
pub const IA32_VMX_EPT_FAULT_WAS_EXECABLE: u64 = 1 << 5;

/// `IA32_VMX_MSR_LIST_SIZE_MASK`.
pub const IA32_VMX_MSR_LIST_SIZE_MASK: u64 = 7 << 25;
/// `IA32_VMX_CR3_TGT_SIZE_MASK`.
pub const IA32_VMX_CR3_TGT_SIZE_MASK: u64 = 0x1ff << 16;

/// `VMX_SKIP_L1D_FLUSH`.
pub const VMX_SKIP_L1D_FLUSH: i32 = 2;
/// `VMX_L1D_FLUSH_SIZE`.
pub const VMX_L1D_FLUSH_SIZE: usize = 64 * 1024;

// SVM
/// `MSR_AMD_VM_CR`.
pub const MSR_AMD_VM_CR: u32 = 0xc001_0114;
/// `MSR_AMD_VM_HSAVE_PA`.
pub const MSR_AMD_VM_HSAVE_PA: u32 = 0xc001_0117;
/// `CPUID_AMD_SVM_CAP`.
pub const CPUID_AMD_SVM_CAP: u32 = 0x8000_000a;
/// `AMD_SVM_NESTED_PAGING_CAP`.
pub const AMD_SVM_NESTED_PAGING_CAP: u32 = 1;
/// `AMD_SVM_NRIP_SAVE_CAP`.
pub const AMD_SVM_NRIP_SAVE_CAP: u32 = 1 << 3;
/// `AMD_SVM_VMCB_CLEAN_CAP`.
pub const AMD_SVM_VMCB_CLEAN_CAP: u32 = 1 << 5;
/// `AMD_SVM_FLUSH_BY_ASID_CAP`.
pub const AMD_SVM_FLUSH_BY_ASID_CAP: u32 = 1 << 6;
/// `AMD_SVM_DECODE_ASSIST_CAP`.
pub const AMD_SVM_DECODE_ASSIST_CAP: u32 = 1 << 7;
/// `AMD_SVMDIS`.
pub const AMD_SVMDIS: u64 = 0x10;

/// `SVM_TLB_CONTROL_FLUSH_NONE`.
pub const SVM_TLB_CONTROL_FLUSH_NONE: u8 = 0;
/// `SVM_TLB_CONTROL_FLUSH_ALL`.
pub const SVM_TLB_CONTROL_FLUSH_ALL: u8 = 1;
/// `SVM_TLB_CONTROL_FLUSH_ASID`.
pub const SVM_TLB_CONTROL_FLUSH_ASID: u8 = 3;
/// `SVM_TLB_CONTROL_FLUSH_ASID_GLB`.
pub const SVM_TLB_CONTROL_FLUSH_ASID_GLB: u8 = 7;

/// `SVM_CLEANBITS_I`.
pub const SVM_CLEANBITS_I: u32 = 1;
/// `SVM_CLEANBITS_IOPM`.
pub const SVM_CLEANBITS_IOPM: u32 = 1 << 1;
/// `SVM_CLEANBITS_ASID`.
pub const SVM_CLEANBITS_ASID: u32 = 1 << 2;
/// `SVM_CLEANBITS_TPR`.
pub const SVM_CLEANBITS_TPR: u32 = 1 << 3;
/// `SVM_CLEANBITS_NP`.
pub const SVM_CLEANBITS_NP: u32 = 1 << 4;
/// `SVM_CLEANBITS_CR`.
pub const SVM_CLEANBITS_CR: u32 = 1 << 5;
/// `SVM_CLEANBITS_DR`.
pub const SVM_CLEANBITS_DR: u32 = 1 << 6;
/// `SVM_CLEANBITS_DT`.
pub const SVM_CLEANBITS_DT: u32 = 1 << 7;
/// `SVM_CLEANBITS_SEG`.
pub const SVM_CLEANBITS_SEG: u32 = 1 << 8;
/// `SVM_CLEANBITS_CR2`.
pub const SVM_CLEANBITS_CR2: u32 = 1 << 9;
/// `SVM_CLEANBITS_LBR`.
pub const SVM_CLEANBITS_LBR: u32 = 1 << 10;
/// `SVM_CLEANBITS_AVIC`.
pub const SVM_CLEANBITS_AVIC: u32 = 1 << 11;

/// `SVM_CLEANBITS_ALL`.
pub const SVM_CLEANBITS_ALL: u32 = SVM_CLEANBITS_I
    | SVM_CLEANBITS_IOPM
    | SVM_CLEANBITS_ASID
    | SVM_CLEANBITS_TPR
    | SVM_CLEANBITS_NP
    | SVM_CLEANBITS_CR
    | SVM_CLEANBITS_DR
    | SVM_CLEANBITS_DT
    | SVM_CLEANBITS_SEG
    | SVM_CLEANBITS_CR2
    | SVM_CLEANBITS_LBR
    | SVM_CLEANBITS_AVIC;

/// `SVM_INTR_MISC_V_IGN_TPR`.
pub const SVM_INTR_MISC_V_IGN_TPR: u8 = 0x10;

// SVM : VMCB intercepts
/// `SVM_INTERCEPT_CR0_READ`.
pub const SVM_INTERCEPT_CR0_READ: u32 = 1;
/// `SVM_INTERCEPT_CR1_READ`.
pub const SVM_INTERCEPT_CR1_READ: u32 = 1 << 1;
/// `SVM_INTERCEPT_CR2_READ`.
pub const SVM_INTERCEPT_CR2_READ: u32 = 1 << 2;
/// `SVM_INTERCEPT_CR3_READ`.
pub const SVM_INTERCEPT_CR3_READ: u32 = 1 << 2;
/// `SVM_INTERCEPT_CR4_READ`.
pub const SVM_INTERCEPT_CR4_READ: u32 = 1 << 4;
/// `SVM_INTERCEPT_CR5_READ`.
pub const SVM_INTERCEPT_CR5_READ: u32 = 1 << 5;
/// `SVM_INTERCEPT_CR6_READ`.
pub const SVM_INTERCEPT_CR6_READ: u32 = 1 << 6;
/// `SVM_INTERCEPT_CR7_READ`.
pub const SVM_INTERCEPT_CR7_READ: u32 = 1 << 7;
/// `SVM_INTERCEPT_CR8_READ`.
pub const SVM_INTERCEPT_CR8_READ: u32 = 1 << 8;
/// `SVM_INTERCEPT_CR9_READ`.
pub const SVM_INTERCEPT_CR9_READ: u32 = 1 << 9;
/// `SVM_INTERCEPT_CR10_READ`.
pub const SVM_INTERCEPT_CR10_READ: u32 = 1 << 10;
/// `SVM_INTERCEPT_CR11_READ`.
pub const SVM_INTERCEPT_CR11_READ: u32 = 1 << 11;
/// `SVM_INTERCEPT_CR12_READ`.
pub const SVM_INTERCEPT_CR12_READ: u32 = 1 << 12;
/// `SVM_INTERCEPT_CR13_READ`.
pub const SVM_INTERCEPT_CR13_READ: u32 = 1 << 13;
/// `SVM_INTERCEPT_CR14_READ`.
pub const SVM_INTERCEPT_CR14_READ: u32 = 1 << 14;
/// `SVM_INTERCEPT_CR15_READ`.
pub const SVM_INTERCEPT_CR15_READ: u32 = 1 << 15;
/// `SVM_INTERCEPT_CR0_WRITE`.
pub const SVM_INTERCEPT_CR0_WRITE: u32 = 1 << 16;
/// `SVM_INTERCEPT_CR1_WRITE`.
pub const SVM_INTERCEPT_CR1_WRITE: u32 = 1 << 17;
/// `SVM_INTERCEPT_CR2_WRITE`.
pub const SVM_INTERCEPT_CR2_WRITE: u32 = 1 << 18;
/// `SVM_INTERCEPT_CR3_WRITE`.
pub const SVM_INTERCEPT_CR3_WRITE: u32 = 1 << 19;
/// `SVM_INTERCEPT_CR4_WRITE`.
pub const SVM_INTERCEPT_CR4_WRITE: u32 = 1 << 20;
/// `SVM_INTERCEPT_CR5_WRITE`.
pub const SVM_INTERCEPT_CR5_WRITE: u32 = 1 << 21;
/// `SVM_INTERCEPT_CR6_WRITE`.
pub const SVM_INTERCEPT_CR6_WRITE: u32 = 1 << 22;
/// `SVM_INTERCEPT_CR7_WRITE`.
pub const SVM_INTERCEPT_CR7_WRITE: u32 = 1 << 23;
/// `SVM_INTERCEPT_CR8_WRITE`.
pub const SVM_INTERCEPT_CR8_WRITE: u32 = 1 << 24;
/// `SVM_INTERCEPT_CR9_WRITE`.
pub const SVM_INTERCEPT_CR9_WRITE: u32 = 1 << 25;
/// `SVM_INTERCEPT_CR10_WRITE`.
pub const SVM_INTERCEPT_CR10_WRITE: u32 = 1 << 26;
/// `SVM_INTERCEPT_CR11_WRITE`.
pub const SVM_INTERCEPT_CR11_WRITE: u32 = 1 << 27;
/// `SVM_INTERCEPT_CR12_WRITE`.
pub const SVM_INTERCEPT_CR12_WRITE: u32 = 1 << 28;
/// `SVM_INTERCEPT_CR13_WRITE`.
pub const SVM_INTERCEPT_CR13_WRITE: u32 = 1 << 29;
/// `SVM_INTERCEPT_CR14_WRITE`.
pub const SVM_INTERCEPT_CR14_WRITE: u32 = 1 << 30;
/// `SVM_INTERCEPT_CR15_WRITE`.
pub const SVM_INTERCEPT_CR15_WRITE: u32 = 1 << 31;
/// `SVM_INTERCEPT_DR0_READ`.
pub const SVM_INTERCEPT_DR0_READ: u32 = 1;
/// `SVM_INTERCEPT_DR1_READ`.
pub const SVM_INTERCEPT_DR1_READ: u32 = 1 << 1;
/// `SVM_INTERCEPT_DR2_READ`.
pub const SVM_INTERCEPT_DR2_READ: u32 = 1 << 2;
/// `SVM_INTERCEPT_DR3_READ`: the header says `1UL << 2`, which is DR2's bit (EXT-231).
pub const SVM_INTERCEPT_DR3_READ: u32 = 1 << 3;
/// `SVM_INTERCEPT_DR4_READ`.
pub const SVM_INTERCEPT_DR4_READ: u32 = 1 << 4;
/// `SVM_INTERCEPT_DR5_READ`.
pub const SVM_INTERCEPT_DR5_READ: u32 = 1 << 5;
/// `SVM_INTERCEPT_DR6_READ`.
pub const SVM_INTERCEPT_DR6_READ: u32 = 1 << 6;
/// `SVM_INTERCEPT_DR7_READ`.
pub const SVM_INTERCEPT_DR7_READ: u32 = 1 << 7;
/// `SVM_INTERCEPT_DR8_READ`.
pub const SVM_INTERCEPT_DR8_READ: u32 = 1 << 8;
/// `SVM_INTERCEPT_DR9_READ`.
pub const SVM_INTERCEPT_DR9_READ: u32 = 1 << 9;
/// `SVM_INTERCEPT_DR10_READ`.
pub const SVM_INTERCEPT_DR10_READ: u32 = 1 << 10;
/// `SVM_INTERCEPT_DR11_READ`.
pub const SVM_INTERCEPT_DR11_READ: u32 = 1 << 11;
/// `SVM_INTERCEPT_DR12_READ`.
pub const SVM_INTERCEPT_DR12_READ: u32 = 1 << 12;
/// `SVM_INTERCEPT_DR13_READ`.
pub const SVM_INTERCEPT_DR13_READ: u32 = 1 << 13;
/// `SVM_INTERCEPT_DR14_READ`.
pub const SVM_INTERCEPT_DR14_READ: u32 = 1 << 14;
/// `SVM_INTERCEPT_DR15_READ`.
pub const SVM_INTERCEPT_DR15_READ: u32 = 1 << 15;
/// `SVM_INTERCEPT_DR0_WRITE`.
pub const SVM_INTERCEPT_DR0_WRITE: u32 = 1 << 16;
/// `SVM_INTERCEPT_DR1_WRITE`.
pub const SVM_INTERCEPT_DR1_WRITE: u32 = 1 << 17;
/// `SVM_INTERCEPT_DR2_WRITE`.
pub const SVM_INTERCEPT_DR2_WRITE: u32 = 1 << 18;
/// `SVM_INTERCEPT_DR3_WRITE`.
pub const SVM_INTERCEPT_DR3_WRITE: u32 = 1 << 19;
/// `SVM_INTERCEPT_DR4_WRITE`.
pub const SVM_INTERCEPT_DR4_WRITE: u32 = 1 << 20;
/// `SVM_INTERCEPT_DR5_WRITE`.
pub const SVM_INTERCEPT_DR5_WRITE: u32 = 1 << 21;
/// `SVM_INTERCEPT_DR6_WRITE`.
pub const SVM_INTERCEPT_DR6_WRITE: u32 = 1 << 22;
/// `SVM_INTERCEPT_DR7_WRITE`.
pub const SVM_INTERCEPT_DR7_WRITE: u32 = 1 << 23;
/// `SVM_INTERCEPT_DR8_WRITE`.
pub const SVM_INTERCEPT_DR8_WRITE: u32 = 1 << 24;
/// `SVM_INTERCEPT_DR9_WRITE`.
pub const SVM_INTERCEPT_DR9_WRITE: u32 = 1 << 25;
/// `SVM_INTERCEPT_DR10_WRITE`.
pub const SVM_INTERCEPT_DR10_WRITE: u32 = 1 << 26;
/// `SVM_INTERCEPT_DR11_WRITE`.
pub const SVM_INTERCEPT_DR11_WRITE: u32 = 1 << 27;
/// `SVM_INTERCEPT_DR12_WRITE`.
pub const SVM_INTERCEPT_DR12_WRITE: u32 = 1 << 28;
/// `SVM_INTERCEPT_DR13_WRITE`.
pub const SVM_INTERCEPT_DR13_WRITE: u32 = 1 << 29;
/// `SVM_INTERCEPT_DR14_WRITE`.
pub const SVM_INTERCEPT_DR14_WRITE: u32 = 1 << 30;
/// `SVM_INTERCEPT_DR15_WRITE`.
pub const SVM_INTERCEPT_DR15_WRITE: u32 = 1 << 31;
/// `SVM_INTERCEPT_INTR`.
pub const SVM_INTERCEPT_INTR: u32 = 1;
/// `SVM_INTERCEPT_NMI`.
pub const SVM_INTERCEPT_NMI: u32 = 1 << 1;
/// `SVM_INTERCEPT_SMI`.
pub const SVM_INTERCEPT_SMI: u32 = 1 << 2;
/// `SVM_INTERCEPT_INIT`.
pub const SVM_INTERCEPT_INIT: u32 = 1 << 3;
/// `SVM_INTERCEPT_VINTR`.
pub const SVM_INTERCEPT_VINTR: u32 = 1 << 4;
/// `SVM_INTERCEPT_CR0_SEL_WRITE`.
pub const SVM_INTERCEPT_CR0_SEL_WRITE: u32 = 1 << 5;
/// `SVM_INTERCEPT_IDTR_READ`.
pub const SVM_INTERCEPT_IDTR_READ: u32 = 1 << 6;
/// `SVM_INTERCEPT_GDTR_READ`.
pub const SVM_INTERCEPT_GDTR_READ: u32 = 1 << 7;
/// `SVM_INTERCEPT_LDTR_READ`.
pub const SVM_INTERCEPT_LDTR_READ: u32 = 1 << 8;
/// `SVM_INTERCEPT_TR_READ`.
pub const SVM_INTERCEPT_TR_READ: u32 = 1 << 9;
/// `SVM_INTERCEPT_IDTR_WRITE`.
pub const SVM_INTERCEPT_IDTR_WRITE: u32 = 1 << 10;
/// `SVM_INTERCEPT_GDTR_WRITE`.
pub const SVM_INTERCEPT_GDTR_WRITE: u32 = 1 << 11;
/// `SVM_INTERCEPT_LDTR_WRITE`.
pub const SVM_INTERCEPT_LDTR_WRITE: u32 = 1 << 12;
/// `SVM_INTERCEPT_TR_WRITE`.
pub const SVM_INTERCEPT_TR_WRITE: u32 = 1 << 13;
/// `SVM_INTERCEPT_RDTSC`.
pub const SVM_INTERCEPT_RDTSC: u32 = 1 << 14;
/// `SVM_INTERCEPT_RDPMC`.
pub const SVM_INTERCEPT_RDPMC: u32 = 1 << 15;
/// `SVM_INTERCEPT_PUSHF`.
pub const SVM_INTERCEPT_PUSHF: u32 = 1 << 16;
/// `SVM_INTERCEPT_POPF`.
pub const SVM_INTERCEPT_POPF: u32 = 1 << 17;
/// `SVM_INTERCEPT_CPUID`.
pub const SVM_INTERCEPT_CPUID: u32 = 1 << 18;
/// `SVM_INTERCEPT_RSM`.
pub const SVM_INTERCEPT_RSM: u32 = 1 << 19;
/// `SVM_INTERCEPT_IRET`.
pub const SVM_INTERCEPT_IRET: u32 = 1 << 20;
/// `SVM_INTERCEPT_INTN`.
pub const SVM_INTERCEPT_INTN: u32 = 1 << 21;
/// `SVM_INTERCEPT_INVD`.
pub const SVM_INTERCEPT_INVD: u32 = 1 << 22;
/// `SVM_INTERCEPT_PAUSE`.
pub const SVM_INTERCEPT_PAUSE: u32 = 1 << 23;
/// `SVM_INTERCEPT_HLT`.
pub const SVM_INTERCEPT_HLT: u32 = 1 << 24;
/// `SVM_INTERCEPT_INVLPG`.
pub const SVM_INTERCEPT_INVLPG: u32 = 1 << 25;
/// `SVM_INTERCEPT_INVLPGA`.
pub const SVM_INTERCEPT_INVLPGA: u32 = 1 << 26;
/// `SVM_INTERCEPT_INOUT`.
pub const SVM_INTERCEPT_INOUT: u32 = 1 << 27;
/// `SVM_INTERCEPT_MSR`.
pub const SVM_INTERCEPT_MSR: u32 = 1 << 28;
/// `SVM_INTERCEPT_TASK_SWITCH`.
pub const SVM_INTERCEPT_TASK_SWITCH: u32 = 1 << 29;
/// `SVM_INTERCEPT_FERR_FREEZE`.
pub const SVM_INTERCEPT_FERR_FREEZE: u32 = 1 << 30;
/// `SVM_INTERCEPT_SHUTDOWN`.
pub const SVM_INTERCEPT_SHUTDOWN: u32 = 1 << 31;
/// `SVM_INTERCEPT_VMRUN`.
pub const SVM_INTERCEPT_VMRUN: u32 = 1;
/// `SVM_INTERCEPT_VMMCALL`.
pub const SVM_INTERCEPT_VMMCALL: u32 = 1 << 1;
/// `SVM_INTERCEPT_VMLOAD`.
pub const SVM_INTERCEPT_VMLOAD: u32 = 1 << 2;
/// `SVM_INTERCEPT_VMSAVE`.
pub const SVM_INTERCEPT_VMSAVE: u32 = 1 << 3;
/// `SVM_INTERCEPT_STGI`.
pub const SVM_INTERCEPT_STGI: u32 = 1 << 4;
/// `SVM_INTERCEPT_CLGI`.
pub const SVM_INTERCEPT_CLGI: u32 = 1 << 5;
/// `SVM_INTERCEPT_SKINIT`.
pub const SVM_INTERCEPT_SKINIT: u32 = 1 << 6;
/// `SVM_INTERCEPT_RDTSCP`.
pub const SVM_INTERCEPT_RDTSCP: u32 = 1 << 7;
/// `SVM_INTERCEPT_ICEBP`.
pub const SVM_INTERCEPT_ICEBP: u32 = 1 << 8;
/// `SVM_INTERCEPT_WBINVD`.
pub const SVM_INTERCEPT_WBINVD: u32 = 1 << 9;
/// `SVM_INTERCEPT_MONITOR`.
pub const SVM_INTERCEPT_MONITOR: u32 = 1 << 10;
/// `SVM_INTERCEPT_MWAIT_UNCOND`.
pub const SVM_INTERCEPT_MWAIT_UNCOND: u32 = 1 << 11;
/// `SVM_INTERCEPT_MWAIT_COND`.
pub const SVM_INTERCEPT_MWAIT_COND: u32 = 1 << 12;
/// `SVM_INTERCEPT_XSETBV`.
pub const SVM_INTERCEPT_XSETBV: u32 = 1 << 13;
/// `SVM_INTERCEPT_EFER_WRITE`.
pub const SVM_INTERCEPT_EFER_WRITE: u32 = 1 << 15;
/// `SVM_INTERCEPT_CR0_WRITE_POST`.
pub const SVM_INTERCEPT_CR0_WRITE_POST: u32 = 1 << 16;
/// `SVM_INTERCEPT_CR1_WRITE_POST`.
pub const SVM_INTERCEPT_CR1_WRITE_POST: u32 = 1 << 17;
/// `SVM_INTERCEPT_CR2_WRITE_POST`.
pub const SVM_INTERCEPT_CR2_WRITE_POST: u32 = 1 << 18;
/// `SVM_INTERCEPT_CR3_WRITE_POST`.
pub const SVM_INTERCEPT_CR3_WRITE_POST: u32 = 1 << 19;
/// `SVM_INTERCEPT_CR4_WRITE_POST`.
pub const SVM_INTERCEPT_CR4_WRITE_POST: u32 = 1 << 20;
/// `SVM_INTERCEPT_CR5_WRITE_POST`.
pub const SVM_INTERCEPT_CR5_WRITE_POST: u32 = 1 << 21;
/// `SVM_INTERCEPT_CR6_WRITE_POST`.
pub const SVM_INTERCEPT_CR6_WRITE_POST: u32 = 1 << 22;
/// `SVM_INTERCEPT_CR7_WRITE_POST`.
pub const SVM_INTERCEPT_CR7_WRITE_POST: u32 = 1 << 23;
/// `SVM_INTERCEPT_CR8_WRITE_POST`.
pub const SVM_INTERCEPT_CR8_WRITE_POST: u32 = 1 << 24;
/// `SVM_INTERCEPT_CR9_WRITE_POST`.
pub const SVM_INTERCEPT_CR9_WRITE_POST: u32 = 1 << 25;
/// `SVM_INTERCEPT_CR10_WRITE_POST`.
pub const SVM_INTERCEPT_CR10_WRITE_POST: u32 = 1 << 26;
/// `SVM_INTERCEPT_CR11_WRITE_POST`.
pub const SVM_INTERCEPT_CR11_WRITE_POST: u32 = 1 << 27;
/// `SVM_INTERCEPT_CR12_WRITE_POST`.
pub const SVM_INTERCEPT_CR12_WRITE_POST: u32 = 1 << 28;
/// `SVM_INTERCEPT_CR13_WRITE_POST`.
pub const SVM_INTERCEPT_CR13_WRITE_POST: u32 = 1 << 29;
/// `SVM_INTERCEPT_CR14_WRITE_POST`.
pub const SVM_INTERCEPT_CR14_WRITE_POST: u32 = 1 << 30;
/// `SVM_INTERCEPT_CR15_WRITE_POST`.
pub const SVM_INTERCEPT_CR15_WRITE_POST: u32 = 1 << 31;

// SME and SEV
/// `CPUID_AMD_SEV_CAP`.
pub const CPUID_AMD_SEV_CAP: u32 = 0x8000_001f;
/// `AMD_SME_CAP`.
pub const AMD_SME_CAP: u32 = 1;
/// `AMD_SEV_CAP`.
pub const AMD_SEV_CAP: u32 = 1 << 1;

// PAT
/// `PAT_UC`.
pub const PAT_UC: u64 = 0x0;
/// `PAT_WC`.
pub const PAT_WC: u64 = 0x1;
/// `PAT_WT`.
pub const PAT_WT: u64 = 0x4;
/// `PAT_WP`.
pub const PAT_WP: u64 = 0x5;
/// `PAT_WB`.
pub const PAT_WB: u64 = 0x6;
/// `PAT_UCMINUS`.
pub const PAT_UCMINUS: u64 = 0x7;

// XSAVE subfeatures (cpuid 0xd, leaf 1)
/// `XSAVE_XSAVEOPT`.
pub const XSAVE_XSAVEOPT: u32 = 0x01;
/// `XSAVE_XSAVEC`.
pub const XSAVE_XSAVEC: u32 = 0x02;
/// `XSAVE_XGETBV1`.
pub const XSAVE_XGETBV1: u32 = 0x04;
/// `XSAVE_XSAVES`.
pub const XSAVE_XSAVES: u32 = 0x08;
/// `XSAVE_XFD`.
pub const XSAVE_XFD: u32 = 0x10;
/// `XSAVE_BITS`: the names of these bits, for `%b`.
pub const XSAVE_BITS: &[u8] = b"\x10\x01XSAVEOPT\x02XSAVEC\x03XGETBV1\x04XSAVES\x05XFD";

// Default cr0 and cr4 flags.
/// `CR0_DEFAULT`: what `mptramp.S` loads into an application processor's `CR0`.
pub const CR0_DEFAULT: u64 = CR0_PE | CR0_PG | CR0_NE | CR0_WP;
/// `CR4_DEFAULT`: the `CR4` bits every CPU runs with.
pub const CR4_DEFAULT: u64 = CR4_PAE | CR4_PGE | CR4_PSE | CR4_OSFXSR | CR4_OSXMMEXCPT;

/// `CPUIDEAX_NUM_GC(cpuid)`.
pub const fn cpuideax_num_gc(cpuid: u32) -> u32 {
    (cpuid >> 8) & 0x0000_00ff
}

/// `CPUIDEAX_BIT_GC(cpuid)`.
pub const fn cpuideax_bit_gc(cpuid: u32) -> u32 {
    (cpuid >> 16) & 0x0000_00ff
}

/// `CPUIDEAX_LEN_EBX(cpuid)`.
pub const fn cpuideax_len_ebx(cpuid: u32) -> u32 {
    (cpuid >> 24) & 0x0000_00ff
}

/// `CPUIDEDX_NUM_FC(cpuid)`: the number of fixed-function counters.
pub const fn cpuidedx_num_fc(cpuid: u32) -> u32 {
    cpuid & 0x0000_001f
}

/// `CPUIDEDX_BIT_FC(cpuid)`.
pub const fn cpuidedx_bit_fc(cpuid: u32) -> u32 {
    (cpuid >> 5) & 0x0000_00ff
}

/// `CPUID2FAMILY(cpuid)`.
pub const fn cpuid2family(cpuid: u32) -> u32 {
    (cpuid >> 8) & 15
}

/// `CPUID2MODEL(cpuid)`.
pub const fn cpuid2model(cpuid: u32) -> u32 {
    (cpuid >> 4) & 15
}

/// `CPUID2STEPPING(cpuid)`.
pub const fn cpuid2stepping(cpuid: u32) -> u32 {
    cpuid & 15
}

/// `CPUID(code, eax, ebx, ecx, edx)`: executes `cpuid` for function `code`.
#[inline]
pub fn cpuid(code: u32) -> (u32, u32, u32, u32) {
    cpuid_leaf(code, 0)
}

/// `CPUID_LEAF(code, leaf, eax, ebx, ecx, edx)`: executes `cpuid` for function `code`, sub-leaf
/// `leaf`.
#[inline]
pub fn cpuid_leaf(code: u32, leaf: u32) -> (u32, u32, u32, u32) {
    // `__cpuid_count` is safe to call on every x86-64 CPU (the instruction always exists in
    // long mode); it saves and restores `rbx` around `cpuid` for LLVM.
    #[allow(unused_unsafe)]
    // SAFETY: `cpuid` only reads identification registers; it has no side effects.
    let CpuidResult { eax, ebx, ecx, edx } = unsafe { __cpuid_count(code, leaf) };
    (eax, ebx, ecx, edx)
}

/// `MSR_THERM_STATUS_TEMP(msr)`.
pub const fn msr_therm_status_temp(msr: u64) -> u64 {
    (msr >> 16) & 0x7f
}

/// `MSR_TEMPERATURE_TARGET_TJMAX(msr)`.
pub const fn msr_temperature_target_tjmax(msr: u64) -> u64 {
    (msr >> 16) & 0xff
}

/// `MSR_PERF_FIXED_CTR_FC(_i, _v)`.
pub const fn msr_perf_fixed_ctr_fc(i: u64, v: u64) -> u64 {
    v << (4 * i)
}

/// `MSR_PERF_FIXED_CTR_ANYTHR(_i)`.
pub const fn msr_perf_fixed_ctr_anythr(i: u64) -> u64 {
    0x4 << (4 * i)
}

/// `MSR_PERF_FIXED_CTR_INT(_i)`.
pub const fn msr_perf_fixed_ctr_int(i: u64) -> u64 {
    0x8 << (4 * i)
}

/// `MSR_PSTATEDEF(_n)`.
pub const fn msr_pstatedef(n: u32) -> u32 {
    0xc001_0064 + n
}

/// `PATENTRY(n, type)`.
pub const fn patentry(n: u64, r#type: u64) -> u64 {
    r#type << (n * 8)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::reftest::{assert_complete, assert_defines};

    /// The bytes of a `%b` description as the C spells it: string literals side by side,
    /// each an octal escape and a name.
    fn c_bits(text: &str) -> std::vec::Vec<u8> {
        let mut out = std::vec::Vec::new();
        for (i, piece) in text.split('"').enumerate() {
            if i % 2 == 0 {
                assert!(
                    piece.trim_matches([' ', '\t', '(', ')']).is_empty(),
                    "{text}"
                );
                continue;
            }
            let rest = piece.strip_prefix('\\').expect("an escape first");
            // At most three octal digits, as in C: a name may start with a digit.
            let octal = |b: &u8| (b'0'..=b'7').contains(b);
            let digits = rest.bytes().take(3).take_while(octal).count();
            out.push(u8::from_str_radix(&rest[..digits], 8).expect("an octal escape"));
            out.extend_from_slice(rest[digits..].as_bytes());
        }
        out
    }

    #[test]
    fn the_fields_of_a_cpuid_word() {
        // Family 6, model 0xe, stepping 3 in function 1's EAX.
        assert_eq!(cpuid2family(0x06e3), 6);
        assert_eq!(cpuid2model(0x06e3), 0xe);
        assert_eq!(cpuid2stepping(0x06e3), 3);
        // Function 0xa: version 2, 4 general counters of 48 bits, 7 event bits.
        assert_eq!(0x0730_0402 & CPUIDEAX_VERID, 2);
        assert_eq!(cpuideax_num_gc(0x0730_0402), 4);
        assert_eq!(cpuideax_bit_gc(0x0730_0402), 48);
        assert_eq!(cpuideax_len_ebx(0x0730_0402), 7);
        assert_eq!(cpuidedx_num_fc(0x0603), 3);
        assert_eq!(cpuidedx_bit_fc(0x0603), 48);
    }

    #[test]
    fn the_fields_of_an_msr() {
        assert_eq!(msr_therm_status_temp(0x8825_0000), 0x25);
        assert_eq!(msr_temperature_target_tjmax(0x0064_0000), 100);
        assert_eq!(msr_perf_fixed_ctr_fc(1, MSR_PERF_FIXED_CTR_FC_ANY), 0x30);
        assert_eq!(msr_perf_fixed_ctr_anythr(2), 0x400);
        assert_eq!(msr_perf_fixed_ctr_int(0), 0x8);
        assert_eq!(msr_pstatedef(3), 0xc001_0067);
        // Entry 1 of the PAT as write-combining.
        assert_eq!(patentry(1, PAT_WC), 0x100);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/arch/amd64/include/specialreg.h");
        let ints = assert_defines!(defs;
            CR0_PE, CR0_MP, CR0_EM, CR0_TS, CR0_ET, CR0_PG, CR0_NE, CR0_WP, CR0_AM, CR0_NW,
            CR0_CD, CR3_PCID, CR3_PWT, CR3_PCD, CR3_REUSE_PCID, CR3_PADDR, CR4_VME, CR4_PVI,
            CR4_TSD, CR4_DE, CR4_PSE, CR4_PAE, CR4_MCE, CR4_PGE, CR4_PCE, CR4_OSFXSR,
            CR4_OSXMMEXCPT, CR4_UMIP, CR4_VMXE, CR4_SMXE, CR4_FSGSBASE, CR4_PCIDE, CR4_OSXSAVE,
            CR4_KL, CR4_SMEP, CR4_SMAP, CR4_PKE, CR4_CET, CR4_PKS, CR4_UINTR, XFEATURE_X87,
            XFEATURE_SSE, XFEATURE_AVX, XFEATURE_BNDREG, XFEATURE_BNDCSR, XFEATURE_MPX,
            XFEATURE_OPMASK, XFEATURE_ZMM_HI256, XFEATURE_HI16_ZMM, XFEATURE_AVX512,
            XFEATURE_PT, XFEATURE_PKRU, XFEATURE_PASID, XFEATURE_CET_U, XFEATURE_CET_S,
            XFEATURE_CET, XFEATURE_HDC, XFEATURE_UINTR, XFEATURE_LBR, XFEATURE_HWP,
            XFEATURE_TILECFG, XFEATURE_TILEDATA, XFEATURE_AMX, XFEATURE_COMPRESSED,
            XFEATURE_XCR0_MASK, XFEATURE_XSS_MASK, CPUID_FPU, CPUID_VME, CPUID_DE, CPUID_PSE,
            CPUID_TSC, CPUID_MSR, CPUID_PAE, CPUID_MCE, CPUID_CX8, CPUID_APIC, CPUID_SYS1,
            CPUID_SEP, CPUID_MTRR, CPUID_PGE, CPUID_MCA, CPUID_CMOV, CPUID_PAT, CPUID_PSE36,
            CPUID_PSN, CPUID_CFLUSH, CPUID_B20, CPUID_DS, CPUID_ACPI, CPUID_MMX, CPUID_FXSR,
            CPUID_SSE, CPUID_SSE2, CPUID_SS, CPUID_HTT, CPUID_TM, CPUID_B30, CPUID_PBE,
            CPUIDECX_SSE3, CPUIDECX_PCLMUL, CPUIDECX_DTES64, CPUIDECX_MWAIT, CPUIDECX_DSCPL,
            CPUIDECX_VMX, CPUIDECX_SMX, CPUIDECX_EST, CPUIDECX_TM2, CPUIDECX_SSSE3,
            CPUIDECX_CNXTID, CPUIDECX_SDBG, CPUIDECX_FMA3, CPUIDECX_CX16, CPUIDECX_XTPR,
            CPUIDECX_PDCM, CPUIDECX_PCID, CPUIDECX_DCA, CPUIDECX_SSE41, CPUIDECX_SSE42,
            CPUIDECX_X2APIC, CPUIDECX_MOVBE, CPUIDECX_POPCNT, CPUIDECX_DEADLINE, CPUIDECX_AES,
            CPUIDECX_XSAVE, CPUIDECX_OSXSAVE, CPUIDECX_AVX, CPUIDECX_F16C, CPUIDECX_RDRAND,
            CPUIDECX_HV, SEFF0EBX_FSGSBASE, SEFF0EBX_TSC_ADJUST, SEFF0EBX_SGX, SEFF0EBX_BMI1,
            SEFF0EBX_HLE, SEFF0EBX_AVX2, SEFF0EBX_SMEP, SEFF0EBX_BMI2, SEFF0EBX_ERMS,
            SEFF0EBX_INVPCID, SEFF0EBX_RTM, SEFF0EBX_PQM, SEFF0EBX_MPX, SEFF0EBX_AVX512F,
            SEFF0EBX_AVX512DQ, SEFF0EBX_RDSEED, SEFF0EBX_ADX, SEFF0EBX_SMAP,
            SEFF0EBX_AVX512IFMA, SEFF0EBX_PCOMMIT, SEFF0EBX_CLFLUSHOPT, SEFF0EBX_CLWB,
            SEFF0EBX_PT, SEFF0EBX_AVX512PF, SEFF0EBX_AVX512ER, SEFF0EBX_AVX512CD, SEFF0EBX_SHA,
            SEFF0EBX_AVX512BW, SEFF0EBX_AVX512VL, SEFF0ECX_PREFETCHWT1, SEFF0ECX_AVX512VBMI,
            SEFF0ECX_UMIP, SEFF0ECX_PKU, SEFF0ECX_OSPKE, SEFF0ECX_WAITPKG, SEFF0ECX_PKS,
            SEFF0EDX_AVX512_4FNNIW, SEFF0EDX_AVX512_4FMAPS, SEFF0EDX_SRBDS_CTRL,
            SEFF0EDX_MD_CLEAR, SEFF0EDX_TSXFA, SEFF0EDX_IBT, SEFF0EDX_IBRS, SEFF0EDX_STIBP,
            SEFF0EDX_L1DF, SEFF0EDX_ARCH_CAP, SEFF0EDX_SSBD, SEFF2EDX_BHI_CTRL, TPM_SENSOR,
            TPM_ARAT, TPM_PTS, TPM_EFFFREQ, CPUIDEAX_VERID, CPUIDEBX_EVT_CORE,
            CPUIDEBX_EVT_INST, CPUIDEBX_EVT_REFR, CPUIDEBX_EVT_CACHE_REF,
            CPUIDEBX_EVT_CACHE_MIS, CPUIDEBX_EVT_BRANCH_INST, CPUIDEBX_EVT_BRANCH_MISP,
            CPUID_MPC, CPUID_NXE, CPUID_MMXX, CPUID_FFXSR, CPUID_PAGE1GB, CPUID_RDTSCP,
            CPUID_LONG, CPUID_3DNOW2, CPUID_3DNOW, CPUIDECX_LAHF, CPUIDECX_CMPLEG, CPUIDECX_SVM,
            CPUIDECX_EAPICSP, CPUIDECX_AMCR8, CPUIDECX_ABM, CPUIDECX_SSE4A, CPUIDECX_MASSE,
            CPUIDECX_3DNOWP, CPUIDECX_OSVW, CPUIDECX_IBS, CPUIDECX_XOP, CPUIDECX_SKINIT,
            CPUIDECX_WDT, CPUIDECX_LWP, CPUIDECX_FMA4, CPUIDECX_TCE, CPUIDECX_NODEID,
            CPUIDECX_TBM, CPUIDECX_TOPEXT, CPUIDECX_CPCTR, CPUIDECX_DBKP, CPUIDECX_PERFTSC,
            CPUIDECX_PCTRL3, CPUIDECX_MWAITX, CPUIDEDX_HWPSTATE, CPUIDEDX_ITSC,
            CPUIDEBX_INVLPGB, CPUIDEBX_IBPB, CPUIDEBX_IBRS, CPUIDEBX_STIBP,
            CPUIDEBX_IBRS_ALWAYSON, CPUIDEBX_STIBP_ALWAYSON, CPUIDEBX_IBRS_PREF,
            CPUIDEBX_IBRS_SAME_MODE, CPUIDEBX_SSBD, CPUIDEBX_VIRT_SSBD, CPUIDEBX_SSBD_NOTREQ,
            CPUIDEAX_SME, CPUIDEAX_SEV, CPUIDEAX_PFLUSH_MSR, CPUIDEAX_SEVES, CPUIDEAX_SEVSNP,
            CPUIDEAX_VMPL, CPUIDEAX_RMPQUERY, CPUIDEAX_VMPLSSS, CPUIDEAX_SECTSC,
            CPUIDEAX_TSCAUXVIRT, CPUIDEAX_HWECACHECOH, CPUIDEAX_64BITHOST, CPUIDEAX_RESTINJ,
            CPUIDEAX_ALTINJ, CPUIDEAX_DBGSTSW, CPUIDEAX_IBSDISALLOW, CPUIDEAX_VTE,
            CPUIDEAX_VMGEXITPARAM, CPUIDEAX_VTOMMSR, CPUIDEAX_IBSVIRT, CPUIDEAX_VMSARPROT,
            CPUIDEAX_SMTPROT, CPUIDEAX_SVSMPAGEMSR, CPUIDEAX_NVSMSR, MSR_P5_MC_ADDR,
            MSR_P5_MC_TYPE, MSR_TSC, MSR_CESR, MSR_CTR0, MSR_CTR1, MSR_PLATFORM_ID,
            MSR_APICBASE, APICBASE_BSP, APICBASE_ENABLE_X2APIC, APICBASE_GLOBAL_ENABLE,
            APICBASE_ADDRESS_MASK, MSR_EBL_CR_POWERON, MSR_EBC_FREQUENCY_ID, MSR_TEST_CTL,
            MSR_IA32_FEATURE_CONTROL, MSR_TSC_ADJUST, MSR_SPEC_CTRL, SPEC_CTRL_IBRS,
            SPEC_CTRL_STIBP, SPEC_CTRL_SSBD, SPEC_CTRL_BHI_DIS_S, MSR_PRED_CMD, PRED_CMD_IBPB,
            MSR_BIOS_UPDT_TRIG, MSR_BBL_CR_D0, MSR_BBL_CR_D1, MSR_BBL_CR_D2, MSR_BIOS_SIGN,
            MSR_SMM_MONITOR_CTL, MSR_SMBASE, MSR_PERFCTR0, MSR_PERFCTR1, MSR_FSB_FREQ,
            MSR_MPERF, MSR_APERF, MSR_MTRRcap, MTRRcap_FIXED, MTRRcap_WC, MTRRcap_SMRR,
            MSR_ARCH_CAPABILITIES, ARCH_CAP_RDCL_NO, ARCH_CAP_IBRS_ALL, ARCH_CAP_RSBA,
            ARCH_CAP_SKIP_L1DFL_VMENTRY, ARCH_CAP_SSB_NO, ARCH_CAP_MDS_NO,
            ARCH_CAP_IF_PSCHANGE_MC_NO, ARCH_CAP_TSX_CTRL, ARCH_CAP_TAA_NO,
            ARCH_CAP_MCU_CONTROL, ARCH_CAP_MISC_PACKAGE_CTLS, ARCH_CAP_ENERGY_FILTERING_CTL,
            ARCH_CAP_DOITM, ARCH_CAP_SBDR_SSDP_NO, ARCH_CAP_FBSDP_NO, ARCH_CAP_PSDP_NO,
            ARCH_CAP_FB_CLEAR, ARCH_CAP_FB_CLEAR_CTRL, ARCH_CAP_RRSBA, ARCH_CAP_BHI_NO,
            ARCH_CAP_XAPIC_DISABLE_STATUS, ARCH_CAP_OVERCLOCKING_STATUS, ARCH_CAP_PBRSB_NO,
            ARCH_CAP_GDS_CTRL, ARCH_CAP_GDS_NO, ARCH_CAP_RFDS_NO, ARCH_CAP_RFDS_CLEAR,
            MSR_FLUSH_CMD, FLUSH_CMD_L1D_FLUSH, MSR_BBL_CR_ADDR, MSR_BBL_CR_DECC,
            MSR_BBL_CR_CTL, MSR_BBL_CR_TRIG, MSR_BBL_CR_BUSY, MSR_BBL_CR_CTR3, MSR_TSX_CTRL,
            TSX_CTRL_RTM_DISABLE, TSX_CTRL_TSX_CPUID_CLEAR, MSR_MCU_OPT_CTRL, RNGDS_MITG_DIS,
            MSR_SYSENTER_CS, MSR_SYSENTER_ESP, MSR_SYSENTER_EIP, MSR_MCG_CAP, MSR_MCG_STATUS,
            MSR_MCG_CTL, MSR_EVNTSEL0, MSR_EVNTSEL1, MSR_PERF_STATUS, MSR_PERF_CTL,
            PERF_CTL_TURBO, MSR_THERM_CONTROL, MSR_THERM_INTERRUPT, MSR_THERM_STATUS,
            MSR_THERM_STATUS_VALID_BIT, MSR_THERM2_CTL, MSR_MISC_ENABLE,
            MISC_ENABLE_FAST_STRINGS, MISC_ENABLE_TCC, MISC_ENABLE_PERF_MON_AVAILABLE,
            MISC_ENABLE_BTS_UNAVAILABLE, MISC_ENABLE_PEBS_UNAVAILABLE, MISC_ENABLE_EIST_ENABLED,
            MISC_ENABLE_ENABLE_MONITOR_FSM, MISC_ENABLE_LIMIT_CPUID_MAXVAL,
            MISC_ENABLE_xTPR_MESSAGE_DISABLE, MISC_ENABLE_XD_BIT_DISABLE,
            MSR_TEMPERATURE_TARGET, MSR_TEMPERATURE_TARGET_UNDOCUMENTED,
            MSR_TEMPERATURE_TARGET_LOW_BIT_UNDOCUMENTED, MSR_DEBUGCTLMSR, MSR_LASTBRANCHFROMIP,
            MSR_LASTBRANCHTOIP, MSR_LASTINTFROMIP, MSR_LASTINTTOIP, MSR_ROB_CR_BKUPTMPDR6,
            MSR_MTRRvarBase, MSR_MTRRfix64K_00000, MSR_MTRRfix16K_80000, MSR_MTRRfix4K_C0000,
            MSR_CR_PAT, MSR_MTRRdefType, MTRRdefType_FIXED_ENABLE, MTRRdefType_ENABLE,
            MSR_PERF_FIXED_CTR1, MSR_PERF_FIXED_CTR2, MSR_PERF_FIXED_CTR_CTRL,
            MSR_PERF_FIXED_CTR_FC_DIS, MSR_PERF_FIXED_CTR_FC_1, MSR_PERF_FIXED_CTR_FC_123,
            MSR_PERF_FIXED_CTR_FC_ANY, MSR_PERF_FIXED_CTR_FC_MASK, MSR_PERF_GLOBAL_CTRL,
            MSR_PERF_GLOBAL_CTR1_EN, MSR_PERF_GLOBAL_CTR2_EN, MSR_PKG_C3_RESIDENCY,
            MSR_PKG_C6_RESIDENCY, MSR_PKG_C7_RESIDENCY, MSR_CORE_C3_RESIDENCY,
            MSR_CORE_C6_RESIDENCY, MSR_CORE_C7_RESIDENCY, MSR_MC0_CTL, MSR_MC0_STATUS,
            MSR_MC0_ADDR, MSR_MC0_MISC, MSR_MC1_CTL, MSR_MC1_STATUS, MSR_MC1_ADDR, MSR_MC1_MISC,
            MSR_MC2_CTL, MSR_MC2_STATUS, MSR_MC2_ADDR, MSR_MC2_MISC, MSR_MC4_CTL,
            MSR_MC4_STATUS, MSR_MC4_ADDR, MSR_MC4_MISC, MSR_MC3_CTL, MSR_MC3_STATUS,
            MSR_MC3_ADDR, MSR_MC3_MISC, MSR_PKG_C2_RESIDENCY, MSR_PKG_C8_RESIDENCY,
            MSR_PKG_C9_RESIDENCY, MSR_PKG_C10_RESIDENCY, MSR_U_CET, MSR_CET_ENDBR_EN,
            MSR_CET_NO_TRACK_EN, MSR_S_CET, MSR_PKRS, MSR_XSS, MSR_CENT_TMTEMPERATURE,
            MSR_K6_UWCCR, MSR_K7_EVNTSEL0, MSR_K7_EVNTSEL1, MSR_K7_EVNTSEL2, MSR_K7_EVNTSEL3,
            MSR_K7_PERFCTR0, MSR_K7_PERFCTR1, MSR_K7_PERFCTR2, MSR_K7_PERFCTR3, MSR_PATCH_LEVEL,
            MSR_SYSCFG, MSR_EFER, EFER_SCE, EFER_LME, EFER_LMA, EFER_NXE, EFER_SVME, MSR_STAR,
            MSR_LSTAR, MSR_CSTAR, MSR_SFMASK, MSR_FSBASE, MSR_GSBASE, MSR_KERNELGSBASE,
            MSR_SYS_CFG, MSR_PATCH_LOADER, MSR_INT_PEN_MSG, MSR_FP_CFG, FP_CFG_9, MSR_DE_CFG,
            DE_CFG_721, DE_CFG_SERIALIZE_LFENCE, DE_CFG_SERIALIZE_9, MSR_BP_CFG, BP_CFG_33,
            IPM_C1E_CMP_HLT, IPM_SMI_CMP_HLT, MSR_HWCR, HWCR_FFDIS, HWCR_TSCFREQSEL,
            PSTATEDEF_EN, MSR_NB_CFG, NB_CFG_DISIOREQLOCK, NB_CFG_DISDATMSK, MSR_SEV_GHCB,
            SEV_CPUID_REQ, SEV_CPUID_RESP, MSR_SEV_STATUS, SEV_STAT_ENABLED,
            SEV_STAT_ES_ENABLED, SEV_STAT_SNP_ACTIVE, MSR_LS_CFG, LS_CFG_DIS_LS2_SQUISH,
            MSR_IC_CFG, IC_CFG_DIS_SEQ_PREFETCH, MSR_DC_CFG, DC_CFG_DIS_CNV_WC_SSO,
            DC_CFG_DIS_SMC_CHK_BUF, MSR_BU_CFG, BU_CFG_THRL2IDXCMPDIS, BU_CFG_WBPFSMCCHKDIS,
            BU_CFG_WBENHWSBDIS, MTRR_N64K, MTRR_N16K, MTRR_N4K, NCR1, NCR2, NCR3, NCR4,
            NCR_SIZE_0K, NCR_SIZE_4K, NCR_SIZE_8K, NCR_SIZE_16K, NCR_SIZE_32K, NCR_SIZE_64K,
            NCR_SIZE_128K, NCR_SIZE_256K, NCR_SIZE_512K, NCR_SIZE_1M, NCR_SIZE_2M, NCR_SIZE_4M,
            NCR_SIZE_8M, NCR_SIZE_16M, NCR_SIZE_32M, NCR_SIZE_4G, PMC5_CESR_EVENT, PMC5_CESR_OS,
            PMC5_CESR_USR, PMC5_CESR_E, PMC5_CESR_P, PMC5_DATA_READ, PMC5_DATA_WRITE,
            PMC5_DATA_TLB_MISS, PMC5_DATA_READ_MISS, PMC5_DATA_WRITE_MISS, PMC5_WRITE_M_E,
            PMC5_DATA_LINES_WBACK, PMC5_DATA_CACHE_SNOOP, PMC5_DATA_CACHE_SNOOP_HIT,
            PMC5_MEM_ACCESS_BOTH_PIPES, PMC5_BANK_CONFLICTS, PMC5_MISALIGNED_DATA,
            PMC5_INST_READ, PMC5_INST_TLB_MISS, PMC5_INST_CACHE_MISS, PMC5_SEGMENT_REG_LOAD,
            PMC5_BRANCHES, PMC5_BTB_HITS, PMC5_BRANCH_TAKEN, PMC5_PIPELINE_FLUSH,
            PMC5_INST_EXECUTED, PMC5_INST_EXECUTED_V_PIPE, PMC5_BUS_UTILIZATION,
            PMC5_WRITE_BACKUP_STALL, PMC5_DATA_READ_STALL, PMC5_WRITE_E_M_STALL,
            PMC5_LOCKED_BUS, PMC5_IO_CYCLE, PMC5_NONCACHE_MEM_READ, PMC5_AGI_STALL, PMC5_FLOPS,
            PMC5_BP0_MATCH, PMC5_BP1_MATCH, PMC5_BP2_MATCH, PMC5_BP3_MATCH, PMC5_HARDWARE_INTR,
            PMC5_DATA_RW, PMC5_DATA_RW_MISS, PMC6_EVTSEL_EVENT, PMC6_EVTSEL_UNIT,
            PMC6_EVTSEL_UNIT_SHIFT, PMC6_EVTSEL_USR, PMC6_EVTSEL_OS, PMC6_EVTSEL_E,
            PMC6_EVTSEL_PC, PMC6_EVTSEL_INT, PMC6_EVTSEL_EN, PMC6_EVTSEL_INV,
            PMC6_EVTSEL_COUNTER_MASK, PMC6_EVTSEL_COUNTER_MASK_SHIFT, PMC6_DATA_MEM_REFS,
            PMC6_DCU_LINES_IN, PMC6_DCU_M_LINES_IN, PMC6_DCU_M_LINES_OUT,
            PMC6_DCU_MISS_OUTSTANDING, PMC6_IFU_IFETCH, PMC6_IFU_IFETCH_MISS, PMC6_ITLB_MISS,
            PMC6_IFU_MEM_STALL, PMC6_ILD_STALL, PMC6_L2_IFETCH, PMC6_L2_LD, PMC6_L2_ST,
            PMC6_L2_LINES_IN, PMC6_L2_LINES_OUT, PMC6_L2_M_LINES_INM, PMC6_L2_M_LINES_OUTM,
            PMC6_L2_RQSTS, PMC6_L2_ADS, PMC6_L2_DBUS_BUSY, PMC6_L2_DBUS_BUSY_RD,
            PMC6_BUS_DRDY_CLOCKS, PMC6_BUS_LOCK_CLOCKS, PMC6_BUS_REQ_OUTSTANDING,
            PMC6_BUS_TRAN_BRD, PMC6_BUS_TRAN_RFO, PMC6_BUS_TRANS_WB, PMC6_BUS_TRAN_IFETCH,
            PMC6_BUS_TRAN_INVAL, PMC6_BUS_TRAN_PWR, PMC6_BUS_TRANS_P, PMC6_BUS_TRANS_IO,
            PMC6_BUS_TRAN_DEF, PMC6_BUS_TRAN_BURST, PMC6_BUS_TRAN_ANY, PMC6_BUS_TRAN_MEM,
            PMC6_BUS_DATA_RCV, PMC6_BUS_BNR_DRV, PMC6_BUS_HIT_DRV, PMC6_BUS_HITM_DRDV,
            PMC6_BUS_SNOOP_STALL, PMC6_FLOPS, PMC6_FP_COMP_OPS_EXE, PMC6_FP_ASSIST, PMC6_MUL,
            PMC6_DIV, PMC6_CYCLES_DIV_BUSY, PMC6_LD_BLOCKS, PMC6_SB_DRAINS,
            PMC6_MISALIGN_MEM_REF, PMC6_EMON_KNI_PREF_DISPATCHED, PMC6_EMON_KNI_PREF_MISS,
            PMC6_INST_RETIRED, PMC6_UOPS_RETIRED, PMC6_INST_DECODED, PMC6_EMON_KNI_INST_RETIRED,
            PMC6_EMON_KNI_COMP_INST_RET, PMC6_HW_INT_RX, PMC6_CYCLES_INT_MASKED,
            PMC6_CYCLES_INT_PENDING_AND_MASKED, PMC6_BR_INST_RETIRED, PMC6_BR_MISS_PRED_RETIRED,
            PMC6_BR_TAKEN_RETIRED, PMC6_BR_MISS_PRED_TAKEN_RET, PMC6_BR_INST_DECODED,
            PMC6_BTB_MISSES, PMC6_BR_BOGUS, PMC6_BACLEARS, PMC6_RESOURCE_STALLS,
            PMC6_PARTIAL_RAT_STALLS, PMC6_SEGMENT_REG_LOADS, PMC6_CPU_CLK_UNHALTED,
            PMC6_MMX_INSTR_EXEC, PMC6_MMX_SAT_INSTR_EXEC, PMC6_MMX_UOPS_EXEC,
            PMC6_MMX_INSTR_TYPE_EXEC, PMC6_FP_MMX_TRANS, PMC6_MMX_ASSIST, PMC6_MMX_INSTR_RET,
            PMC6_SEG_RENAME_STALLS, PMC6_SEG_REG_RENAMES, PMC6_RET_SEG_RENAMES, K7_EVTSEL_EVENT,
            K7_EVTSEL_UNIT, K7_EVTSEL_UNIT_SHIFT, K7_EVTSEL_USR, K7_EVTSEL_OS, K7_EVTSEL_E,
            K7_EVTSEL_PC, K7_EVTSEL_INT, K7_EVTSEL_EN, K7_EVTSEL_INV, K7_EVTSEL_COUNTER_MASK,
            K7_EVTSEL_COUNTER_MASK_SHIFT, K7_SEGMENT_REG_LOADS, K7_STORES_TO_ACTIVE_INST_STREAM,
            K7_DATA_CACHE_ACCESS, K7_DATA_CACHE_MISS, K7_DATA_CACHE_REFILL,
            K7_DATA_CACHE_REFILL_SYSTEM, K7_DATA_CACHE_WBACK, K7_L2_DTLB_HIT, K7_L2_DTLB_MISS,
            K7_MISALIGNED_DATA_REF, K7_SYSTEM_REQUEST, K7_SYSTEM_REQUEST_TYPE, K7_SNOOP_HIT,
            K7_SINGLE_BIT_ECC_ERROR, K7_CACHE_LINE_INVAL, K7_CYCLES_PROCESSOR_IS_RUNNING,
            K7_L2_REQUEST, K7_L2_REQUEST_BUSY, K7_IFU_IFETCH, K7_IFU_IFETCH_MISS,
            K7_IFU_REFILL_FROM_L2, K7_IFU_REFILL_FROM_SYSTEM, K7_ITLB_L1_MISS, K7_ITLB_L2_MISS,
            K7_SNOOP_RESYNC, K7_IFU_STALL, K7_RETURN_STACK_HITS, K7_RETURN_STACK_OVERFLOW,
            K7_RETIRED_INST, K7_RETIRED_OPS, K7_RETIRED_BRANCHES,
            K7_RETIRED_BRANCH_MISPREDICTED, K7_RETIRED_TAKEN_BRANCH,
            K7_RETIRED_TAKEN_BRANCH_MISPREDICTED, K7_RETIRED_FAR_CONTROL_TRANSFER,
            K7_RETIRED_RESYNC_BRANCH, K7_RETIRED_NEAR_RETURNS,
            K7_RETIRED_NEAR_RETURNS_MISPREDICTED, K7_RETIRED_INDIRECT_MISPREDICTED,
            K7_CYCLES_INT_MASKED, K7_CYCLES_INT_PENDING_AND_MASKED, K7_HW_INTR_RECV,
            K7_INSTRUCTION_DECODER_EMPTY, K7_DISPATCH_STALLS, K7_BRANCH_ABORTS_TO_RETIRE,
            K7_SERIALIZE, K7_SEGMENT_LOAD_STALL, K7_ICU_FULL, K7_RESERVATION_STATIONS_FULL,
            K7_FPU_FULL, K7_LS_FULL, K7_ALL_QUIET_STALL,
            K7_FAR_TRANSFER_OR_RESYNC_BRANCH_PENDING, K7_BP0_MATCH, K7_BP1_MATCH, K7_BP2_MATCH,
            K7_BP3_MATCH, C3_HAS_AES, C3_HAS_SHA, C3_HAS_MM, C3_HAS_AESCTR, C3_CPUID_HAS_RNG,
            C3_CPUID_DO_RNG, C3_CPUID_HAS_ACE, C3_CPUID_DO_ACE, C3_CPUID_HAS_ACE2,
            C3_CPUID_DO_ACE2, C3_CPUID_HAS_PHE, C3_CPUID_DO_PHE, C3_CPUID_HAS_PMM,
            C3_CPUID_DO_PMM, C3_CRYPT_CWLO_ROUND_M, C3_CRYPT_CWLO_ALG_M, C3_CRYPT_CWLO_ALG_AES,
            C3_CRYPT_CWLO_KEYGEN_M, C3_CRYPT_CWLO_KEYGEN_HW, C3_CRYPT_CWLO_KEYGEN_SW,
            C3_CRYPT_CWLO_NORMAL, C3_CRYPT_CWLO_INTERMEDIATE, C3_CRYPT_CWLO_ENCRYPT,
            C3_CRYPT_CWLO_DECRYPT, C3_CRYPT_CWLO_KEY128, C3_CRYPT_CWLO_KEY192,
            C3_CRYPT_CWLO_KEY256, IA32_DEBUG_INTERFACE, IA32_DEBUG_INTERFACE_ENABLE,
            IA32_DEBUG_INTERFACE_LOCK, IA32_DEBUG_INTERFACE_MASK, IA32_FEATURE_CONTROL_LOCK,
            IA32_FEATURE_CONTROL_SMX_EN, IA32_FEATURE_CONTROL_VMX_EN,
            IA32_FEATURE_CONTROL_SENTER_EN, IA32_FEATURE_CONTROL_SENTER_PARAM_MASK,
            IA32_VMX_BASIC, IA32_VMX_PINBASED_CTLS, IA32_VMX_PROCBASED_CTLS, IA32_VMX_EXIT_CTLS,
            IA32_VMX_ENTRY_CTLS, IA32_VMX_MISC, IA32_VMX_CR0_FIXED0, IA32_VMX_CR0_FIXED1,
            IA32_VMX_CR4_FIXED0, IA32_VMX_CR4_FIXED1, IA32_VMX_PROCBASED2_CTLS,
            IA32_VMX_EPT_VPID_CAP, IA32_VMX_TRUE_PINBASED_CTLS, IA32_VMX_TRUE_PROCBASED_CTLS,
            IA32_VMX_TRUE_EXIT_CTLS, IA32_VMX_TRUE_ENTRY_CTLS, IA32_VMX_VMFUNC,
            IA32_EPT_VPID_CAP_XO_TRANSLATIONS, IA32_EPT_VPID_CAP_PAGE_WALK_4,
            IA32_EPT_VPID_CAP_WB, IA32_EPT_VPID_CAP_AD_BITS, IA32_EPT_VPID_CAP_INVEPT_CONTEXT,
            IA32_EPT_VPID_CAP_INVEPT_ALL, IA32_EPT_PAGING_CACHE_TYPE_UC,
            IA32_EPT_PAGING_CACHE_TYPE_WB, IA32_EPT_AD_BITS_ENABLE, IA32_EPT_PAGE_WALK_LENGTH,
            IA32_VMX_TRUE_CTLS_AVAIL, IA32_VMX_EXTERNAL_INT_EXITING, IA32_VMX_NMI_EXITING,
            IA32_VMX_VIRTUAL_NMIS, IA32_VMX_ACTIVATE_VMX_PREEMPTION_TIMER,
            IA32_VMX_PROCESS_POSTED_INTERRUPTS, IA32_VMX_INTERRUPT_WINDOW_EXITING,
            IA32_VMX_USE_TSC_OFFSETTING, IA32_VMX_HLT_EXITING, IA32_VMX_INVLPG_EXITING,
            IA32_VMX_MWAIT_EXITING, IA32_VMX_RDPMC_EXITING, IA32_VMX_RDTSC_EXITING,
            IA32_VMX_CR3_LOAD_EXITING, IA32_VMX_CR3_STORE_EXITING, IA32_VMX_CR8_LOAD_EXITING,
            IA32_VMX_CR8_STORE_EXITING, IA32_VMX_USE_TPR_SHADOW, IA32_VMX_NMI_WINDOW_EXITING,
            IA32_VMX_MOV_DR_EXITING, IA32_VMX_UNCONDITIONAL_IO_EXITING, IA32_VMX_USE_IO_BITMAPS,
            IA32_VMX_MONITOR_TRAP_FLAG, IA32_VMX_USE_MSR_BITMAPS, IA32_VMX_MONITOR_EXITING,
            IA32_VMX_PAUSE_EXITING, IA32_VMX_ACTIVATE_SECONDARY_CONTROLS,
            IA32_VMX_VIRTUALIZE_APIC, IA32_VMX_ENABLE_EPT, IA32_VMX_DESCRIPTOR_TABLE_EXITING,
            IA32_VMX_ENABLE_RDTSCP, IA32_VMX_VIRTUALIZE_X2APIC_MODE, IA32_VMX_ENABLE_VPID,
            IA32_VMX_WBINVD_EXITING, IA32_VMX_UNRESTRICTED_GUEST,
            IA32_VMX_APIC_REGISTER_VIRTUALIZATION, IA32_VMX_VIRTUAL_INTERRUPT_DELIVERY,
            IA32_VMX_PAUSE_LOOP_EXITING, IA32_VMX_RDRAND_EXITING, IA32_VMX_ENABLE_INVPCID,
            IA32_VMX_ENABLE_VM_FUNCTIONS, IA32_VMX_VMCS_SHADOWING,
            IA32_VMX_ENABLE_ENCLS_EXITING, IA32_VMX_RDSEED_EXITING, IA32_VMX_ENABLE_PML,
            IA32_VMX_EPT_VIOLATION_VE, IA32_VMX_CONCEAL_VMX_FROM_PT,
            IA32_VMX_ENABLE_XSAVES_XRSTORS, IA32_VMX_ENABLE_TSC_SCALING,
            IA32_VMX_SAVE_DEBUG_CONTROLS, IA32_VMX_HOST_SPACE_ADDRESS_SIZE,
            IA32_VMX_LOAD_IA32_PERF_GLOBAL_CTRL_ON_EXIT, IA32_VMX_ACKNOWLEDGE_INTERRUPT_ON_EXIT,
            IA32_VMX_SAVE_IA32_PAT_ON_EXIT, IA32_VMX_LOAD_IA32_PAT_ON_EXIT,
            IA32_VMX_SAVE_IA32_EFER_ON_EXIT, IA32_VMX_LOAD_IA32_EFER_ON_EXIT,
            IA32_VMX_SAVE_VMX_PREEMPTION_TIMER, IA32_VMX_CLEAR_IA32_BNDCFGS_ON_EXIT,
            IA32_VMX_CONCEAL_VM_EXITS_FROM_PT, IA32_VMX_LOAD_HOST_CET_STATE,
            IA32_VMX_LOAD_DEBUG_CONTROLS, IA32_VMX_IA32E_MODE_GUEST, IA32_VMX_ENTRY_TO_SMM,
            IA32_VMX_DEACTIVATE_DUAL_MONITOR_TREATMENT,
            IA32_VMX_LOAD_IA32_PERF_GLOBAL_CTRL_ON_ENTRY, IA32_VMX_LOAD_IA32_PAT_ON_ENTRY,
            IA32_VMX_LOAD_IA32_EFER_ON_ENTRY, IA32_VMX_LOAD_IA32_BNDCFGS_ON_ENTRY,
            IA32_VMX_CONCEAL_VM_ENTRIES_FROM_PT, IA32_VMX_LOAD_GUEST_CET_STATE, VMCS_GUEST_VPID,
            VMCS_POSTED_INT_NOTIF_VECTOR, VMCS_EPTP_INDEX, VMCS_GUEST_IA32_ES_SEL,
            VMCS_GUEST_IA32_CS_SEL, VMCS_GUEST_IA32_SS_SEL, VMCS_GUEST_IA32_DS_SEL,
            VMCS_GUEST_IA32_FS_SEL, VMCS_GUEST_IA32_GS_SEL, VMCS_GUEST_IA32_LDTR_SEL,
            VMCS_GUEST_IA32_TR_SEL, VMCS_GUEST_INTERRUPT_STATUS, VMCS_GUEST_PML_INDEX,
            VMCS_HOST_IA32_ES_SEL, VMCS_HOST_IA32_CS_SEL, VMCS_HOST_IA32_SS_SEL,
            VMCS_HOST_IA32_DS_SEL, VMCS_HOST_IA32_FS_SEL, VMCS_HOST_IA32_GS_SEL,
            VMCS_HOST_IA32_TR_SEL, VMCS_IO_BITMAP_A, VMCS_IO_BITMAP_B, VMCS_MSR_BITMAP_ADDRESS,
            VMCS_EXIT_STORE_MSR_ADDRESS, VMCS_EXIT_LOAD_MSR_ADDRESS,
            VMCS_ENTRY_LOAD_MSR_ADDRESS, VMCS_EXECUTIVE_VMCS_POINTER, VMCS_PML_ADDRESS,
            VMCS_TSC_OFFSET, VMCS_VIRTUAL_APIC_ADDRESS, VMCS_APIC_ACCESS_ADDRESS,
            VMCS_POSTED_INTERRUPT_DESC, VMCS_VM_FUNCTION_CONTROLS, VMCS_GUEST_IA32_EPTP,
            VMCS_EOI_EXIT_BITMAP_0, VMCS_EOI_EXIT_BITMAP_1, VMCS_EOI_EXIT_BITMAP_2,
            VMCS_EOI_EXIT_BITMAP_3, VMCS_EPTP_LIST_ADDRESS, VMCS_VMREAD_BITMAP_ADDRESS,
            VMCS_VMWRITE_BITMAP_ADDRESS, VMCS_VIRTUALIZATION_EXC_ADDRESS,
            VMCS_XSS_EXITING_BITMAP, VMCS_ENCLS_EXITING_BITMAP, VMCS_TSC_MULTIPLIER,
            VMCS_GUEST_PHYSICAL_ADDRESS, VMCS_LINK_POINTER, VMCS_GUEST_IA32_DEBUGCTL,
            VMCS_GUEST_IA32_PAT, VMCS_GUEST_IA32_EFER, VMCS_GUEST_IA32_PERF_GBL_CTRL,
            VMCS_GUEST_PDPTE0, VMCS_GUEST_PDPTE1, VMCS_GUEST_PDPTE2, VMCS_GUEST_PDPTE3,
            VMCS_GUEST_IA32_BNDCFGS, VMCS_HOST_IA32_PAT, VMCS_HOST_IA32_EFER,
            VMCS_HOST_IA32_PERF_GBL_CTRL, VMCS_PINBASED_CTLS, VMCS_PROCBASED_CTLS,
            VMCS_EXCEPTION_BITMAP, VMCS_PF_ERROR_CODE_MASK, VMCS_PF_ERROR_CODE_MATCH,
            VMCS_CR3_TARGET_COUNT, VMCS_EXIT_CTLS, VMCS_EXIT_MSR_STORE_COUNT,
            VMCS_EXIT_MSR_LOAD_COUNT, VMCS_ENTRY_CTLS, VMCS_ENTRY_MSR_LOAD_COUNT,
            VMCS_ENTRY_INTERRUPTION_INFO, VMCS_ENTRY_EXCEPTION_ERROR_CODE,
            VMCS_ENTRY_INSTRUCTION_LENGTH, VMCS_TPR_THRESHOLD, VMCS_PROCBASED2_CTLS,
            VMCS_PLE_GAP, VMCS_PLE_WINDOW, VMCS_INSTRUCTION_ERROR, VMCS_EXIT_REASON,
            VMCS_EXIT_INTERRUPTION_INFO, VMCS_EXIT_INTERRUPTION_ERR_CODE,
            VMCS_IDT_VECTORING_INFO, VMCS_IDT_VECTORING_ERROR_CODE, VMCS_INSTRUCTION_LENGTH,
            VMCS_EXIT_INSTRUCTION_INFO, VMCS_GUEST_IA32_ES_LIMIT, VMCS_GUEST_IA32_CS_LIMIT,
            VMCS_GUEST_IA32_SS_LIMIT, VMCS_GUEST_IA32_DS_LIMIT, VMCS_GUEST_IA32_FS_LIMIT,
            VMCS_GUEST_IA32_GS_LIMIT, VMCS_GUEST_IA32_LDTR_LIMIT, VMCS_GUEST_IA32_TR_LIMIT,
            VMCS_GUEST_IA32_GDTR_LIMIT, VMCS_GUEST_IA32_IDTR_LIMIT, VMCS_GUEST_IA32_ES_AR,
            VMCS_GUEST_IA32_CS_AR, VMCS_GUEST_IA32_SS_AR, VMCS_GUEST_IA32_DS_AR,
            VMCS_GUEST_IA32_FS_AR, VMCS_GUEST_IA32_GS_AR, VMCS_GUEST_IA32_LDTR_AR,
            VMCS_GUEST_IA32_TR_AR, VMCS_GUEST_INTERRUPTIBILITY_ST, VMX_INT_STATE_BLOCK_STI,
            VMX_INT_STATE_BLOCK_MOVSS, VMCS_GUEST_ACTIVITY_STATE, VMCS_GUEST_SMBASE,
            VMCS_GUEST_IA32_SYSENTER_CS, VMCS_VMX_PREEMPTION_TIMER_VAL,
            VMCS_HOST_IA32_SYSENTER_CS, VMCS_CR0_MASK, VMCS_CR4_MASK, VMCS_CR0_READ_SHADOW,
            VMCS_CR4_READ_SHADOW, VMCS_CR3_TARGET_0, VMCS_CR3_TARGET_1, VMCS_CR3_TARGET_2,
            VMCS_CR3_TARGET_3, VMCS_GUEST_EXIT_QUALIFICATION, VMCS_IO_RCX, VMCS_IO_RSI,
            VMCS_IO_RDI, VMCS_IO_RIP, VMCS_GUEST_LINEAR_ADDRESS, VMCS_GUEST_IA32_CR0,
            VMCS_GUEST_IA32_CR3, VMCS_GUEST_IA32_CR4, VMCS_GUEST_IA32_ES_BASE,
            VMCS_GUEST_IA32_CS_BASE, VMCS_GUEST_IA32_SS_BASE, VMCS_GUEST_IA32_DS_BASE,
            VMCS_GUEST_IA32_FS_BASE, VMCS_GUEST_IA32_GS_BASE, VMCS_GUEST_IA32_LDTR_BASE,
            VMCS_GUEST_IA32_TR_BASE, VMCS_GUEST_IA32_GDTR_BASE, VMCS_GUEST_IA32_IDTR_BASE,
            VMCS_GUEST_IA32_DR7, VMCS_GUEST_IA32_RSP, VMCS_GUEST_IA32_RIP,
            VMCS_GUEST_IA32_RFLAGS, VMCS_GUEST_PENDING_DBG_EXC, VMCS_GUEST_IA32_SYSENTER_ESP,
            VMCS_GUEST_IA32_SYSENTER_EIP, VMCS_GUEST_IA32_S_CET, VMCS_GUEST_SSP,
            VMCS_GUEST_IA32_INTR_SSP_TABLE, VMCS_HOST_IA32_CR0, VMCS_HOST_IA32_CR3,
            VMCS_HOST_IA32_CR4, VMCS_HOST_IA32_FS_BASE, VMCS_HOST_IA32_GS_BASE,
            VMCS_HOST_IA32_TR_BASE, VMCS_HOST_IA32_GDTR_BASE, VMCS_HOST_IA32_IDTR_BASE,
            VMCS_HOST_IA32_SYSENTER_ESP, VMCS_HOST_IA32_SYSENTER_EIP, VMCS_HOST_IA32_RSP,
            VMCS_HOST_IA32_RIP, VMCS_HOST_IA32_S_CET, VMCS_HOST_SSP,
            VMCS_HOST_IA32_INTR_SSP_TABLE, IA32_VMX_INVVPID_INDIV_ADDR_CTX,
            IA32_VMX_INVVPID_SINGLE_CTX, IA32_VMX_INVVPID_ALL_CTX,
            IA32_VMX_INVVPID_SINGLE_CTX_GLB, IA32_VMX_INVEPT_SINGLE_CTX,
            IA32_VMX_INVEPT_GLOBAL_CTX, IA32_VMX_EPT_FAULT_READ, IA32_VMX_EPT_FAULT_WRITE,
            IA32_VMX_EPT_FAULT_EXEC, IA32_VMX_EPT_FAULT_WAS_READABLE,
            IA32_VMX_EPT_FAULT_WAS_WRITABLE, IA32_VMX_EPT_FAULT_WAS_EXECABLE,
            IA32_VMX_MSR_LIST_SIZE_MASK, IA32_VMX_CR3_TGT_SIZE_MASK, VMX_SKIP_L1D_FLUSH,
            VMX_L1D_FLUSH_SIZE, MSR_AMD_VM_CR, MSR_AMD_VM_HSAVE_PA, CPUID_AMD_SVM_CAP,
            AMD_SVM_NESTED_PAGING_CAP, AMD_SVM_NRIP_SAVE_CAP, AMD_SVM_VMCB_CLEAN_CAP,
            AMD_SVM_FLUSH_BY_ASID_CAP, AMD_SVM_DECODE_ASSIST_CAP, AMD_SVMDIS,
            SVM_TLB_CONTROL_FLUSH_NONE, SVM_TLB_CONTROL_FLUSH_ALL, SVM_TLB_CONTROL_FLUSH_ASID,
            SVM_TLB_CONTROL_FLUSH_ASID_GLB, SVM_CLEANBITS_I, SVM_CLEANBITS_IOPM,
            SVM_CLEANBITS_ASID, SVM_CLEANBITS_TPR, SVM_CLEANBITS_NP, SVM_CLEANBITS_CR,
            SVM_CLEANBITS_DR, SVM_CLEANBITS_DT, SVM_CLEANBITS_SEG, SVM_CLEANBITS_CR2,
            SVM_CLEANBITS_LBR, SVM_CLEANBITS_AVIC, SVM_CLEANBITS_ALL, SVM_INTR_MISC_V_IGN_TPR,
            SVM_INTERCEPT_CR0_READ, SVM_INTERCEPT_CR1_READ, SVM_INTERCEPT_CR2_READ,
            SVM_INTERCEPT_CR3_READ, SVM_INTERCEPT_CR4_READ, SVM_INTERCEPT_CR5_READ,
            SVM_INTERCEPT_CR6_READ, SVM_INTERCEPT_CR7_READ, SVM_INTERCEPT_CR8_READ,
            SVM_INTERCEPT_CR9_READ, SVM_INTERCEPT_CR10_READ, SVM_INTERCEPT_CR11_READ,
            SVM_INTERCEPT_CR12_READ, SVM_INTERCEPT_CR13_READ, SVM_INTERCEPT_CR14_READ,
            SVM_INTERCEPT_CR15_READ, SVM_INTERCEPT_CR0_WRITE, SVM_INTERCEPT_CR1_WRITE,
            SVM_INTERCEPT_CR2_WRITE, SVM_INTERCEPT_CR3_WRITE, SVM_INTERCEPT_CR4_WRITE,
            SVM_INTERCEPT_CR5_WRITE, SVM_INTERCEPT_CR6_WRITE, SVM_INTERCEPT_CR7_WRITE,
            SVM_INTERCEPT_CR8_WRITE, SVM_INTERCEPT_CR9_WRITE, SVM_INTERCEPT_CR10_WRITE,
            SVM_INTERCEPT_CR11_WRITE, SVM_INTERCEPT_CR12_WRITE, SVM_INTERCEPT_CR13_WRITE,
            SVM_INTERCEPT_CR14_WRITE, SVM_INTERCEPT_CR15_WRITE, SVM_INTERCEPT_DR0_READ,
            SVM_INTERCEPT_DR1_READ, SVM_INTERCEPT_DR2_READ, SVM_INTERCEPT_DR4_READ,
            SVM_INTERCEPT_DR5_READ, SVM_INTERCEPT_DR6_READ, SVM_INTERCEPT_DR7_READ,
            SVM_INTERCEPT_DR8_READ, SVM_INTERCEPT_DR9_READ, SVM_INTERCEPT_DR10_READ,
            SVM_INTERCEPT_DR11_READ, SVM_INTERCEPT_DR12_READ, SVM_INTERCEPT_DR13_READ,
            SVM_INTERCEPT_DR14_READ, SVM_INTERCEPT_DR15_READ, SVM_INTERCEPT_DR0_WRITE,
            SVM_INTERCEPT_DR1_WRITE, SVM_INTERCEPT_DR2_WRITE, SVM_INTERCEPT_DR3_WRITE,
            SVM_INTERCEPT_DR4_WRITE, SVM_INTERCEPT_DR5_WRITE, SVM_INTERCEPT_DR6_WRITE,
            SVM_INTERCEPT_DR7_WRITE, SVM_INTERCEPT_DR8_WRITE, SVM_INTERCEPT_DR9_WRITE,
            SVM_INTERCEPT_DR10_WRITE, SVM_INTERCEPT_DR11_WRITE, SVM_INTERCEPT_DR12_WRITE,
            SVM_INTERCEPT_DR13_WRITE, SVM_INTERCEPT_DR14_WRITE, SVM_INTERCEPT_DR15_WRITE,
            SVM_INTERCEPT_INTR, SVM_INTERCEPT_NMI, SVM_INTERCEPT_SMI, SVM_INTERCEPT_INIT,
            SVM_INTERCEPT_VINTR, SVM_INTERCEPT_CR0_SEL_WRITE, SVM_INTERCEPT_IDTR_READ,
            SVM_INTERCEPT_GDTR_READ, SVM_INTERCEPT_LDTR_READ, SVM_INTERCEPT_TR_READ,
            SVM_INTERCEPT_IDTR_WRITE, SVM_INTERCEPT_GDTR_WRITE, SVM_INTERCEPT_LDTR_WRITE,
            SVM_INTERCEPT_TR_WRITE, SVM_INTERCEPT_RDTSC, SVM_INTERCEPT_RDPMC,
            SVM_INTERCEPT_PUSHF, SVM_INTERCEPT_POPF, SVM_INTERCEPT_CPUID, SVM_INTERCEPT_RSM,
            SVM_INTERCEPT_IRET, SVM_INTERCEPT_INTN, SVM_INTERCEPT_INVD, SVM_INTERCEPT_PAUSE,
            SVM_INTERCEPT_HLT, SVM_INTERCEPT_INVLPG, SVM_INTERCEPT_INVLPGA, SVM_INTERCEPT_INOUT,
            SVM_INTERCEPT_MSR, SVM_INTERCEPT_TASK_SWITCH, SVM_INTERCEPT_FERR_FREEZE,
            SVM_INTERCEPT_SHUTDOWN, SVM_INTERCEPT_VMRUN, SVM_INTERCEPT_VMMCALL,
            SVM_INTERCEPT_VMLOAD, SVM_INTERCEPT_VMSAVE, SVM_INTERCEPT_STGI, SVM_INTERCEPT_CLGI,
            SVM_INTERCEPT_SKINIT, SVM_INTERCEPT_RDTSCP, SVM_INTERCEPT_ICEBP,
            SVM_INTERCEPT_WBINVD, SVM_INTERCEPT_MONITOR, SVM_INTERCEPT_MWAIT_UNCOND,
            SVM_INTERCEPT_MWAIT_COND, SVM_INTERCEPT_XSETBV, SVM_INTERCEPT_EFER_WRITE,
            SVM_INTERCEPT_CR0_WRITE_POST, SVM_INTERCEPT_CR1_WRITE_POST,
            SVM_INTERCEPT_CR2_WRITE_POST, SVM_INTERCEPT_CR3_WRITE_POST,
            SVM_INTERCEPT_CR4_WRITE_POST, SVM_INTERCEPT_CR5_WRITE_POST,
            SVM_INTERCEPT_CR6_WRITE_POST, SVM_INTERCEPT_CR7_WRITE_POST,
            SVM_INTERCEPT_CR8_WRITE_POST, SVM_INTERCEPT_CR9_WRITE_POST,
            SVM_INTERCEPT_CR10_WRITE_POST, SVM_INTERCEPT_CR11_WRITE_POST,
            SVM_INTERCEPT_CR12_WRITE_POST, SVM_INTERCEPT_CR13_WRITE_POST,
            SVM_INTERCEPT_CR14_WRITE_POST, SVM_INTERCEPT_CR15_WRITE_POST, CPUID_AMD_SEV_CAP,
            AMD_SME_CAP, AMD_SEV_CAP, PAT_UC, PAT_WC, PAT_WT, PAT_WP, PAT_WB, PAT_UCMINUS,
            XSAVE_XSAVEOPT, XSAVE_XSAVEC, XSAVE_XGETBV1, XSAVE_XSAVES, XSAVE_XFD, CR0_DEFAULT,
            CR4_DEFAULT,
        );
        let descriptions: &[(&'static str, &[u8])] = &[
            ("CPUID_EDX_BITS", CPUID_EDX_BITS),
            ("CPUID_ECX_BITS", CPUID_ECX_BITS),
            ("SEFF0_EBX_BITS", SEFF0_EBX_BITS),
            ("SEFF0_ECX_BITS", SEFF0_ECX_BITS),
            ("SEFF0_EDX_BITS", SEFF0_EDX_BITS),
            ("SEFF2_EDX_BITS", SEFF2_EDX_BITS),
            ("TPM_EAX_BITS", TPM_EAX_BITS),
            ("TPM_ECX_BITS", TPM_ECX_BITS),
            ("CPUIDE_EDX_BITS", CPUIDE_EDX_BITS),
            ("CPUIDE_ECX_BITS", CPUIDE_ECX_BITS),
            ("CPUID_APMI_EDX_BITS", CPUID_APMI_EDX_BITS),
            ("CPUID_AMDSPEC_EBX_BITS", CPUID_AMDSPEC_EBX_BITS),
            ("CPUID_AMDSEV_EAX_BITS", CPUID_AMDSEV_EAX_BITS),
            ("CPUID_AMDSEV_ECX_BITS", CPUID_AMDSEV_ECX_BITS),
            ("CPUID_AMDSEV_EDX_BITS", CPUID_AMDSEV_EDX_BITS),
            ("ARCH_CAP_MSR_BITS", ARCH_CAP_MSR_BITS),
            ("XSAVE_BITS", XSAVE_BITS),
        ];
        let mut names = ints;
        for (name, ours) in descriptions {
            assert_eq!(c_bits(&defs[*name]), *ours, "{name}");
            names.push(*name);
        }
        // The slip this file does not carry over (EXT-231) is still the header's: when the
        // pin fixes it, this fails and the deviation goes.
        assert_eq!(
            crate::reftest::int(&defs, "SVM_INTERCEPT_DR3_READ"),
            Some(1 << 2)
        );
        assert_eq!(SVM_INTERCEPT_DR3_READ, 1 << 3);
        names.push("SVM_INTERCEPT_DR3_READ");
        // Every define of the header is one of the above, or its include guard.
        names.push("_MACHINE_SPECIALREG_H_");
        assert_complete(&defs, "", &names);
    }
}
/* </TESTS> */
