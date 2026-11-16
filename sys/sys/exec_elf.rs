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
 * Copyright (c) 1995, 1996 Erik Theisen.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/exec_elf.h>`: the ELF ABI header file, formerly known as "elf_abi.h".
//!
//! Upstream: sys/sys/exec_elf.h @ 3ce1f3f79392
//!
//! Status: `ported`, the whole header: the 32-bit and the 64-bit scalar types, the ELF
//! header, the section header, the symbol, the relocations, the program header, the dynamic
//! entry and the notes in both sizes, `struct elfcore_procinfo`, the auxiliary vector and
//! every constant, in the header's order inside each section of this file.
//!
//! How the C's shapes are written here:
//! - A constant has the type of the member of the 64-bit structures it is compared with
//!   (`ET_*` and `EM_*` are `Elf64_Half`, `PT_*` and `SHT_*` are `Elf64_Word`, `SHF_*`,
//!   `DT_*` and `DF_*` are `Elf64_Xword`, the `st_info` and `st_other` values are bytes,
//!   the `EI_*` are indexes into `e_ident[]`).
//! - `IS_ELF` takes either header, since the macro only names `e_ident`: [`ElfIdent`] is
//!   that requirement, and both headers have it.
//! - The names that follow `ELFSIZE` (`Elf_Ehdr`, ..., `ELF_R_SYM`, `ELF_ST_BIND`,
//!   `ELFCLASS`, `ELF_NO_ADDR`, `AuxInfo`) are the 64-bit ones: `ELFSIZE` is the machine's
//!   `ARCH_ELFSIZE`, 64 on both targets, which a compile-time check holds.
//! - The functions the kernel half declares are `kern/exec_elf.rs`'s.
//!
//! ## Deviations
//! - `ELFMAG` is the four magic bytes, not a C string: without the NUL, `SELFMAG` long.
//! - `d_un`, the union of `d_val` and `d_ptr` in `Elf32_Dyn` and `Elf64_Dyn`, is one word:
//!   both members have the same size and no code needs to tell them apart by type.
//! - `Aux64Info` names the four bytes the C compiler leaves after `au_id`, so that the
//!   bytes copied out to the process are all written.
//! - `CONCAT`, `ELFNAME` and `ELFDEFNNAME` paste tokens to build a name from `ELFSIZE`;
//!   Rust has no such thing and the code writes the name (`elf64_...`) itself.
//! - The `ELFSIZE == 32` set of the `Elf_*` names is not declared (no 32-bit target), nor
//!   are the mips64el forms of `ELF64_R_SYM`, `ELF64_R_TYPE` and `ELF64_R_INFO`.
//! - `_DYNAMIC` is declared outside the kernel only.

use crate::machine::Machine;
use crate::machine::exec::MachineExec;

// e_ident[] identification indexes
// See http://www.sco.com/developers/gabi/latest/ch4.eheader.html
/// `EI_MAG0`: file ID.
pub const EI_MAG0: usize = 0;
/// `EI_MAG1`: file ID.
pub const EI_MAG1: usize = 1;
/// `EI_MAG2`: file ID.
pub const EI_MAG2: usize = 2;
/// `EI_MAG3`: file ID.
pub const EI_MAG3: usize = 3;
/// `EI_CLASS`: file class.
pub const EI_CLASS: usize = 4;
/// `EI_DATA`: data encoding.
pub const EI_DATA: usize = 5;
/// `EI_VERSION`: ELF header version.
pub const EI_VERSION: usize = 6;
/// `EI_OSABI`: OS/ABI ID.
pub const EI_OSABI: usize = 7;
/// `EI_ABIVERSION`: ABI version.
pub const EI_ABIVERSION: usize = 8;
/// `EI_PAD`: start of pad bytes.
pub const EI_PAD: usize = 9;
/// `EI_NIDENT`: Size of e_ident[].
pub const EI_NIDENT: usize = 16;

// e_ident[] magic number
/// `ELFMAG0`: e_ident[EI_MAG0].
pub const ELFMAG0: u8 = 0x7f;
/// `ELFMAG1`: e_ident[EI_MAG1].
pub const ELFMAG1: u8 = b'E';
/// `ELFMAG2`: e_ident[EI_MAG2].
pub const ELFMAG2: u8 = b'L';
/// `ELFMAG3`: e_ident[EI_MAG3].
pub const ELFMAG3: u8 = b'F';
/// `ELFMAG`: the magic, `e_ident[EI_MAG0..=EI_MAG3]`.
pub const ELFMAG: [u8; SELFMAG] = [ELFMAG0, ELFMAG1, ELFMAG2, ELFMAG3];
/// `SELFMAG`: size of magic.
pub const SELFMAG: usize = 4;

// e_ident[] file class
/// `ELFCLASSNONE`: invalid.
pub const ELFCLASSNONE: u8 = 0;
/// `ELFCLASS32`: 32-bit objs.
pub const ELFCLASS32: u8 = 1;
/// `ELFCLASS64`: 64-bit objs.
pub const ELFCLASS64: u8 = 2;
/// `ELFCLASSNUM`: number of classes.
pub const ELFCLASSNUM: u8 = 3;

// e_ident[] data encoding
/// `ELFDATANONE`: invalid.
pub const ELFDATANONE: u8 = 0;
/// `ELFDATA2LSB`: Little-Endian.
pub const ELFDATA2LSB: u8 = 1;
/// `ELFDATA2MSB`: Big-Endian.
pub const ELFDATA2MSB: u8 = 2;
/// `ELFDATANUM`: number of data encode defines.
pub const ELFDATANUM: u8 = 3;

// e_ident[] Operating System/ABI
/// `ELFOSABI_SYSV`: UNIX System V ABI.
pub const ELFOSABI_SYSV: u8 = 0;
/// `ELFOSABI_HPUX`: HP-UX operating system.
pub const ELFOSABI_HPUX: u8 = 1;
/// `ELFOSABI_NETBSD`: NetBSD.
pub const ELFOSABI_NETBSD: u8 = 2;
/// `ELFOSABI_LINUX`: GNU/Linux.
pub const ELFOSABI_LINUX: u8 = 3;
/// `ELFOSABI_HURD`: GNU/Hurd.
pub const ELFOSABI_HURD: u8 = 4;
/// `ELFOSABI_86OPEN`: 86Open common IA32 ABI.
pub const ELFOSABI_86OPEN: u8 = 5;
/// `ELFOSABI_SOLARIS`: Solaris.
pub const ELFOSABI_SOLARIS: u8 = 6;
/// `ELFOSABI_MONTEREY`: Monterey.
pub const ELFOSABI_MONTEREY: u8 = 7;
/// `ELFOSABI_IRIX`: IRIX.
pub const ELFOSABI_IRIX: u8 = 8;
/// `ELFOSABI_FREEBSD`: FreeBSD.
pub const ELFOSABI_FREEBSD: u8 = 9;
/// `ELFOSABI_TRU64`: TRU64 UNIX.
pub const ELFOSABI_TRU64: u8 = 10;
/// `ELFOSABI_MODESTO`: Novell Modesto.
pub const ELFOSABI_MODESTO: u8 = 11;
/// `ELFOSABI_OPENBSD`: OpenBSD.
pub const ELFOSABI_OPENBSD: u8 = 12;
/// `ELFOSABI_ARM`: ARM.
pub const ELFOSABI_ARM: u8 = 97;
/// `ELFOSABI_STANDALONE`: Standalone (embedded) application.
pub const ELFOSABI_STANDALONE: u8 = 255;

// e_type
/// `ET_NONE`: No file type.
pub const ET_NONE: u16 = 0;
/// `ET_REL`: relocatable file.
pub const ET_REL: u16 = 1;
/// `ET_EXEC`: executable file.
pub const ET_EXEC: u16 = 2;
/// `ET_DYN`: shared object file.
pub const ET_DYN: u16 = 3;
/// `ET_CORE`: core file.
pub const ET_CORE: u16 = 4;
/// `ET_NUM`: number of types.
pub const ET_NUM: u16 = 5;
/// `ET_LOPROC`: reserved range for processor.
pub const ET_LOPROC: u16 = 0xff00;
/// `ET_HIPROC`: specific e_type.
pub const ET_HIPROC: u16 = 0xffff;

// e_machine
/// `EM_NONE`: No Machine.
pub const EM_NONE: u16 = 0;
/// `EM_M32`: AT&T WE 32100.
pub const EM_M32: u16 = 1;
/// `EM_SPARC`: SPARC.
pub const EM_SPARC: u16 = 2;
/// `EM_386`: Intel 80386.
pub const EM_386: u16 = 3;
/// `EM_68K`: Motorola 68000.
pub const EM_68K: u16 = 4;
/// `EM_88K`: Motorola 88000.
pub const EM_88K: u16 = 5;
/// `EM_486`: Intel 80486 - unused?.
pub const EM_486: u16 = 6;
/// `EM_860`: Intel 80860.
pub const EM_860: u16 = 7;
/// `EM_MIPS`: MIPS R3000 Big-Endian only.
pub const EM_MIPS: u16 = 8;

// Don't know if EM_MIPS_RS4_BE,
// EM_SPARC64, EM_PARISC,
// or EM_PPC are ABI compliant
/// `EM_MIPS_RS4_BE`: MIPS R4000 Big-Endian.
pub const EM_MIPS_RS4_BE: u16 = 10;
/// `EM_SPARC64`: SPARC v9 64-bit unofficial.
pub const EM_SPARC64: u16 = 11;
/// `EM_PARISC`: HPPA.
pub const EM_PARISC: u16 = 15;
/// `EM_SPARC32PLUS`: Enhanced instruction set SPARC.
pub const EM_SPARC32PLUS: u16 = 18;
/// `EM_PPC`: PowerPC.
pub const EM_PPC: u16 = 20;
/// `EM_PPC64`: PowerPC 64.
pub const EM_PPC64: u16 = 21;
/// `EM_ARM`: Advanced RISC Machines ARM.
pub const EM_ARM: u16 = 40;
/// `EM_ALPHA`: DEC ALPHA.
pub const EM_ALPHA: u16 = 41;
/// `EM_SH`: Hitachi/Renesas Super-H.
pub const EM_SH: u16 = 42;
/// `EM_SPARCV9`: SPARC version 9.
pub const EM_SPARCV9: u16 = 43;
/// `EM_IA_64`: Intel IA-64 Processor.
pub const EM_IA_64: u16 = 50;
/// `EM_AMD64`: AMD64 architecture.
pub const EM_AMD64: u16 = 62;
/// `EM_X86_64`.
pub const EM_X86_64: u16 = EM_AMD64;
/// `EM_VAX`: DEC VAX.
pub const EM_VAX: u16 = 75;
/// `EM_AARCH64`: ARM 64-bit architecture (AArch64).
pub const EM_AARCH64: u16 = 183;
/// `EM_RISCV`: RISC-V.
pub const EM_RISCV: u16 = 243;

// Non-standard
/// `EM_ALPHA_EXP`: DEC ALPHA.
pub const EM_ALPHA_EXP: u16 = 0x9026;
/// `EM__LAST__`.
pub const EM__LAST__: u16 = EM_ALPHA_EXP + 1;

// Version
/// `EV_NONE`: Invalid.
pub const EV_NONE: u32 = 0;
/// `EV_CURRENT`: Current.
pub const EV_CURRENT: u32 = 1;
/// `EV_NUM`: number of versions.
pub const EV_NUM: u32 = 2;

// Magic for e_phnum: get real value from sh_info of first section header
/// `PN_XNUM`.
pub const PN_XNUM: u16 = 0xffff;

// Special Section Indexes
/// `SHN_UNDEF`: undefined.
pub const SHN_UNDEF: u16 = 0;
/// `SHN_LORESERVE`: lower bounds of reserved indexes.
pub const SHN_LORESERVE: u16 = 0xff00;
/// `SHN_LOPROC`: reserved range for processor.
pub const SHN_LOPROC: u16 = 0xff00;
/// `SHN_HIPROC`: specific section indexes.
pub const SHN_HIPROC: u16 = 0xff1f;
/// `SHN_ABS`: absolute value.
pub const SHN_ABS: u16 = 0xfff1;
/// `SHN_COMMON`: common symbol.
pub const SHN_COMMON: u16 = 0xfff2;
/// `SHN_XINDEX`: Escape -- index stored elsewhere.
pub const SHN_XINDEX: u16 = 0xffff;
/// `SHN_HIRESERVE`: upper bounds of reserved indexes.
pub const SHN_HIRESERVE: u16 = 0xffff;

// sh_type
/// `SHT_NULL`: inactive.
pub const SHT_NULL: u32 = 0;
/// `SHT_PROGBITS`: program defined information.
pub const SHT_PROGBITS: u32 = 1;
/// `SHT_SYMTAB`: symbol table section.
pub const SHT_SYMTAB: u32 = 2;
/// `SHT_STRTAB`: string table section.
pub const SHT_STRTAB: u32 = 3;
/// `SHT_RELA`: relocation section with addends.
pub const SHT_RELA: u32 = 4;
/// `SHT_HASH`: symbol hash table section.
pub const SHT_HASH: u32 = 5;
/// `SHT_DYNAMIC`: dynamic section.
pub const SHT_DYNAMIC: u32 = 6;
/// `SHT_NOTE`: note section.
pub const SHT_NOTE: u32 = 7;
/// `SHT_NOBITS`: no space section.
pub const SHT_NOBITS: u32 = 8;
/// `SHT_REL`: relocation section without addends.
pub const SHT_REL: u32 = 9;
/// `SHT_SHLIB`: reserved - purpose unknown.
pub const SHT_SHLIB: u32 = 10;
/// `SHT_DYNSYM`: dynamic symbol table section.
pub const SHT_DYNSYM: u32 = 11;
/// `SHT_NUM`: number of section types.
pub const SHT_NUM: u32 = 12;
/// `SHT_INIT_ARRAY`: pointers to init functions.
pub const SHT_INIT_ARRAY: u32 = 14;
/// `SHT_FINI_ARRAY`: pointers to termination functions.
pub const SHT_FINI_ARRAY: u32 = 15;
/// `SHT_PREINIT_ARRAY`: ptrs to funcs called before init.
pub const SHT_PREINIT_ARRAY: u32 = 16;
/// `SHT_GROUP`: defines a section group.
pub const SHT_GROUP: u32 = 17;
/// `SHT_SYMTAB_SHNDX`: Section indexes (see SHN_XINDEX).
pub const SHT_SYMTAB_SHNDX: u32 = 18;
/// `SHT_RELR`: relative-only relocation section.
pub const SHT_RELR: u32 = 19;
/// `SHT_LOOS`: reserved range for OS specific.
pub const SHT_LOOS: u32 = 0x6000_0000;
/// `SHT_SUNW_dof`: used by dtrace.
#[allow(non_upper_case_globals)] // the C name
pub const SHT_SUNW_dof: u32 = 0x6fff_fff4;
/// `SHT_GNU_LIBLIST`: libraries to be prelinked.
pub const SHT_GNU_LIBLIST: u32 = 0x6fff_fff7;
/// `SHT_SUNW_move`: inf for partially init'ed symbols.
#[allow(non_upper_case_globals)] // the C name
pub const SHT_SUNW_move: u32 = 0x6fff_fffa;
/// `SHT_SUNW_syminfo`: ad symbol information.
#[allow(non_upper_case_globals)] // the C name
pub const SHT_SUNW_syminfo: u32 = 0x6fff_fffc;
/// `SHT_SUNW_verdef`: symbol versioning inf.
#[allow(non_upper_case_globals)] // the C name
pub const SHT_SUNW_verdef: u32 = 0x6fff_fffd;
/// `SHT_SUNW_verneed`: symbol versioning req.
#[allow(non_upper_case_globals)] // the C name
pub const SHT_SUNW_verneed: u32 = 0x6fff_fffe;
/// `SHT_SUNW_versym`: symbol versioning table.
#[allow(non_upper_case_globals)] // the C name
pub const SHT_SUNW_versym: u32 = 0x6fff_ffff;
/// `SHT_HIOS`: section header types.
pub const SHT_HIOS: u32 = 0x6fff_ffff;
/// `SHT_LOPROC`: reserved range for processor.
pub const SHT_LOPROC: u32 = 0x7000_0000;
/// `SHT_HIPROC`: specific section header types.
pub const SHT_HIPROC: u32 = 0x7fff_ffff;
/// `SHT_LOUSER`: reserved range for application.
pub const SHT_LOUSER: u32 = 0x8000_0000;
/// `SHT_HIUSER`: specific indexes.
pub const SHT_HIUSER: u32 = 0xffff_ffff;

/// `SHT_GNU_HASH`: GNU-style hash table section.
pub const SHT_GNU_HASH: u32 = 0x6fff_fff6;

// Section names
/// `ELF_BSS`: uninitialized data.
pub const ELF_BSS: &str = ".bss";
/// `ELF_DATA`: initialized data.
pub const ELF_DATA: &str = ".data";
/// `ELF_CTF`: CTF data.
pub const ELF_CTF: &str = ".SUNW_ctf";
/// `ELF_DEBUG`: debug.
pub const ELF_DEBUG: &str = ".debug";
/// `ELF_DYNAMIC`: dynamic linking information.
pub const ELF_DYNAMIC: &str = ".dynamic";
/// `ELF_DYNSTR`: dynamic string table.
pub const ELF_DYNSTR: &str = ".dynstr";
/// `ELF_DYNSYM`: dynamic symbol table.
pub const ELF_DYNSYM: &str = ".dynsym";
/// `ELF_FINI`: termination code.
pub const ELF_FINI: &str = ".fini";
/// `ELF_GOT`: global offset table.
pub const ELF_GOT: &str = ".got";
/// `ELF_HASH`: symbol hash table.
pub const ELF_HASH: &str = ".hash";
/// `ELF_INIT`: initialization code.
pub const ELF_INIT: &str = ".init";
/// `ELF_REL_DATA`: relocation data.
pub const ELF_REL_DATA: &str = ".rel.data";
/// `ELF_REL_FINI`: relocation termination code.
pub const ELF_REL_FINI: &str = ".rel.fini";
/// `ELF_REL_INIT`: relocation initialization code.
pub const ELF_REL_INIT: &str = ".rel.init";
/// `ELF_REL_DYN`: relocation dynamic link info.
pub const ELF_REL_DYN: &str = ".rel.dyn";
/// `ELF_REL_RODATA`: relocation read-only data.
pub const ELF_REL_RODATA: &str = ".rel.rodata";
/// `ELF_REL_TEXT`: relocation code.
pub const ELF_REL_TEXT: &str = ".rel.text";
/// `ELF_RODATA`: read-only data.
pub const ELF_RODATA: &str = ".rodata";
/// `ELF_SHSTRTAB`: section header string table.
pub const ELF_SHSTRTAB: &str = ".shstrtab";
/// `ELF_STRTAB`: string table.
pub const ELF_STRTAB: &str = ".strtab";
/// `ELF_SYMTAB`: symbol table.
pub const ELF_SYMTAB: &str = ".symtab";
/// `ELF_TEXT`: code.
pub const ELF_TEXT: &str = ".text";
/// `ELF_OPENBSDRANDOMDATA`: constant randomdata.
pub const ELF_OPENBSDRANDOMDATA: &str = ".openbsd.randomdata";
/// `ELF_OPENBSDMUTABLE`: mutable bss.
pub const ELF_OPENBSDMUTABLE: &str = ".openbsd.mutable";

// Section Attribute Flags - sh_flags
/// `SHF_WRITE`: Writable.
pub const SHF_WRITE: u64 = 0x1;
/// `SHF_ALLOC`: occupies memory.
pub const SHF_ALLOC: u64 = 0x2;
/// `SHF_EXECINSTR`: executable.
pub const SHF_EXECINSTR: u64 = 0x4;
/// `SHF_MERGE`: may be merged.
pub const SHF_MERGE: u64 = 0x10;
/// `SHF_STRINGS`: contains strings.
pub const SHF_STRINGS: u64 = 0x20;
/// `SHF_INFO_LINK`: sh_info holds section index.
pub const SHF_INFO_LINK: u64 = 0x40;
/// `SHF_LINK_ORDER`: ordering requirements.
pub const SHF_LINK_ORDER: u64 = 0x80;
/// `SHF_OS_NONCONFORMING`: OS-specific processing required.
pub const SHF_OS_NONCONFORMING: u64 = 0x100;
/// `SHF_GROUP`: member of section group.
pub const SHF_GROUP: u64 = 0x200;
/// `SHF_TLS`: thread local storage.
pub const SHF_TLS: u64 = 0x400;
/// `SHF_COMPRESSED`: contains compressed data.
pub const SHF_COMPRESSED: u64 = 0x800;
/// `SHF_MASKOS`: OS-specific semantics.
pub const SHF_MASKOS: u64 = 0x0ff0_0000;
/// `SHF_MASKPROC`: reserved bits for processor specific section attributes.
pub const SHF_MASKPROC: u64 = 0xf000_0000;

// Symbol table index
/// `STN_UNDEF`: undefined.
pub const STN_UNDEF: u32 = 0;

// Symbol Binding - ELF32_ST_BIND - st_info
/// `STB_LOCAL`: Local symbol.
pub const STB_LOCAL: u8 = 0;
/// `STB_GLOBAL`: Global symbol.
pub const STB_GLOBAL: u8 = 1;
/// `STB_WEAK`: like global - lower precedence.
pub const STB_WEAK: u8 = 2;
/// `STB_NUM`: number of symbol bindings.
pub const STB_NUM: u8 = 3;
/// `STB_LOPROC`: reserved range for processor.
pub const STB_LOPROC: u8 = 13;
/// `STB_HIPROC`: specific symbol bindings.
pub const STB_HIPROC: u8 = 15;

// Symbol type - ELF32_ST_TYPE - st_info
/// `STT_NOTYPE`: not specified.
pub const STT_NOTYPE: u8 = 0;
/// `STT_OBJECT`: data object.
pub const STT_OBJECT: u8 = 1;
/// `STT_FUNC`: function.
pub const STT_FUNC: u8 = 2;
/// `STT_SECTION`: section.
pub const STT_SECTION: u8 = 3;
/// `STT_FILE`: file.
pub const STT_FILE: u8 = 4;
/// `STT_TLS`: thread local storage.
pub const STT_TLS: u8 = 6;
/// `STT_LOPROC`: reserved range for processor.
pub const STT_LOPROC: u8 = 13;
/// `STT_HIPROC`: specific symbol types.
pub const STT_HIPROC: u8 = 15;

// Extract symbol visibility - st_other
/// `STV_DEFAULT`: Visibility set by binding type.
pub const STV_DEFAULT: u8 = 0;
/// `STV_INTERNAL`: OS specific version of STV_HIDDEN.
pub const STV_INTERNAL: u8 = 1;
/// `STV_HIDDEN`: can only be seen inside own .so.
pub const STV_HIDDEN: u8 = 2;
/// `STV_PROTECTED`: HIDDEN inside, DEFAULT outside.
pub const STV_PROTECTED: u8 = 3;

// Segment types - p_type
/// `PT_NULL`: unused.
pub const PT_NULL: u32 = 0;
/// `PT_LOAD`: loadable segment.
pub const PT_LOAD: u32 = 1;
/// `PT_DYNAMIC`: dynamic linking section.
pub const PT_DYNAMIC: u32 = 2;
/// `PT_INTERP`: the RTLD.
pub const PT_INTERP: u32 = 3;
/// `PT_NOTE`: auxiliary information.
pub const PT_NOTE: u32 = 4;
/// `PT_SHLIB`: reserved - purpose undefined.
pub const PT_SHLIB: u32 = 5;
/// `PT_PHDR`: program header.
pub const PT_PHDR: u32 = 6;
/// `PT_TLS`: thread local storage.
pub const PT_TLS: u32 = 7;
/// `PT_LOOS`: reserved range for OS.
pub const PT_LOOS: u32 = 0x6000_0000;
/// `PT_HIOS`: specific segment types.
pub const PT_HIOS: u32 = 0x6fff_ffff;
/// `PT_LOPROC`: reserved range for processor.
pub const PT_LOPROC: u32 = 0x7000_0000;
/// `PT_HIPROC`: specific segment types.
pub const PT_HIPROC: u32 = 0x7fff_ffff;

/// `PT_GNU_EH_FRAME`: Exception handling info.
pub const PT_GNU_EH_FRAME: u32 = 0x6474_e550;
/// `PT_GNU_RELRO`: Read-only after relocation.
pub const PT_GNU_RELRO: u32 = 0x6474_e552;
/// `PT_GNU_PROPERTY`: Program property note.
pub const PT_GNU_PROPERTY: u32 = 0x6474_e553;

/// `PT_OPENBSD_MUTABLE`: like bss, but not immutable.
pub const PT_OPENBSD_MUTABLE: u32 = 0x65a3_dbe5;
/// `PT_OPENBSD_RANDOMIZE`: fill with random data.
pub const PT_OPENBSD_RANDOMIZE: u32 = 0x65a3_dbe6;
/// `PT_OPENBSD_WXNEEDED`: program performs W^X violations.
pub const PT_OPENBSD_WXNEEDED: u32 = 0x65a3_dbe7;
/// `PT_OPENBSD_NOBTCFI`: no branch target CFI.
pub const PT_OPENBSD_NOBTCFI: u32 = 0x65a3_dbe8;
/// `PT_OPENBSD_SYSCALLS`: syscall locations.
pub const PT_OPENBSD_SYSCALLS: u32 = 0x65a3_dbe9;
/// `PT_OPENBSD_BOOTDATA`: section for boot arguments.
pub const PT_OPENBSD_BOOTDATA: u32 = 0x65a4_1be6;

// Segment flags - p_flags
/// `PF_X`: Executable.
pub const PF_X: u32 = 0x1;
/// `PF_W`: Writable.
pub const PF_W: u32 = 0x2;
/// `PF_R`: Readable.
pub const PF_R: u32 = 0x4;
/// `PF_MASKOS`: reserved bits for OS specific segment flags.
pub const PF_MASKOS: u32 = 0x0ff0_0000;
/// `PF_MASKPROC`: reserved bits for processor specific segment flags.
pub const PF_MASKPROC: u32 = 0xf000_0000;

/// `PF_OPENBSD_MUTABLE`: Mutable.
pub const PF_OPENBSD_MUTABLE: u32 = 0x0800_0000;

/// `PF_ISVNODE`: For coredump segments.
pub const PF_ISVNODE: u32 = 0x0010_0000;

// Dynamic Array Tags - d_tag
/// `DT_NULL`: marks end of _DYNAMIC array.
pub const DT_NULL: u64 = 0;
/// `DT_NEEDED`: string table offset of needed lib.
pub const DT_NEEDED: u64 = 1;
/// `DT_PLTRELSZ`: size of relocation entries in PLT.
pub const DT_PLTRELSZ: u64 = 2;
/// `DT_PLTGOT`: address PLT/GOT.
pub const DT_PLTGOT: u64 = 3;
/// `DT_HASH`: address of symbol hash table.
pub const DT_HASH: u64 = 4;
/// `DT_STRTAB`: address of string table.
pub const DT_STRTAB: u64 = 5;
/// `DT_SYMTAB`: address of symbol table.
pub const DT_SYMTAB: u64 = 6;
/// `DT_RELA`: address of relocation table.
pub const DT_RELA: u64 = 7;
/// `DT_RELASZ`: size of relocation table.
pub const DT_RELASZ: u64 = 8;
/// `DT_RELAENT`: size of relocation entry.
pub const DT_RELAENT: u64 = 9;
/// `DT_STRSZ`: size of string table.
pub const DT_STRSZ: u64 = 10;
/// `DT_SYMENT`: size of symbol table entry.
pub const DT_SYMENT: u64 = 11;
/// `DT_INIT`: address of initialization func.
pub const DT_INIT: u64 = 12;
/// `DT_FINI`: address of termination function.
pub const DT_FINI: u64 = 13;
/// `DT_SONAME`: string table offset of shared obj.
pub const DT_SONAME: u64 = 14;
/// `DT_RPATH`: string table offset of library search path.
pub const DT_RPATH: u64 = 15;
/// `DT_SYMBOLIC`: start sym search in shared obj.
pub const DT_SYMBOLIC: u64 = 16;
/// `DT_REL`: address of rel. tbl. w addends.
pub const DT_REL: u64 = 17;
/// `DT_RELSZ`: size of DT_REL relocation table.
pub const DT_RELSZ: u64 = 18;
/// `DT_RELENT`: size of DT_REL relocation entry.
pub const DT_RELENT: u64 = 19;
/// `DT_PLTREL`: PLT referenced relocation entry.
pub const DT_PLTREL: u64 = 20;
/// `DT_DEBUG`: bugger.
pub const DT_DEBUG: u64 = 21;
/// `DT_TEXTREL`: Allow rel. mod. to unwritable seg.
pub const DT_TEXTREL: u64 = 22;
/// `DT_JMPREL`: add. of PLT's relocation entries.
pub const DT_JMPREL: u64 = 23;
/// `DT_BIND_NOW`: Bind now regardless of env setting.
pub const DT_BIND_NOW: u64 = 24;
/// `DT_INIT_ARRAY`: address of array of init func.
pub const DT_INIT_ARRAY: u64 = 25;
/// `DT_FINI_ARRAY`: address of array of term func.
pub const DT_FINI_ARRAY: u64 = 26;
/// `DT_INIT_ARRAYSZ`: size of array of init func.
pub const DT_INIT_ARRAYSZ: u64 = 27;
/// `DT_FINI_ARRAYSZ`: size of array of term func.
pub const DT_FINI_ARRAYSZ: u64 = 28;
/// `DT_RUNPATH`: strtab offset of lib search path.
pub const DT_RUNPATH: u64 = 29;
/// `DT_FLAGS`: Set of DF_* flags.
pub const DT_FLAGS: u64 = 30;
/// `DT_ENCODING`: further DT_* follow encoding rules.
pub const DT_ENCODING: u64 = 31;
/// `DT_PREINIT_ARRAY`: address of array of preinit func.
pub const DT_PREINIT_ARRAY: u64 = 32;
/// `DT_PREINIT_ARRAYSZ`: size of array of preinit func.
pub const DT_PREINIT_ARRAYSZ: u64 = 33;
/// `DT_RELRSZ`: size of DT_RELR relocation table.
pub const DT_RELRSZ: u64 = 35;
/// `DT_RELR`: addr of DT_RELR relocation table.
pub const DT_RELR: u64 = 36;
/// `DT_RELRENT`: size of DT_RELR relocation entry.
pub const DT_RELRENT: u64 = 37;
/// `DT_LOOS`: reserved range for OS.
pub const DT_LOOS: u64 = 0x6000_000d;
/// `DT_HIOS`: specific dynamic array tags.
pub const DT_HIOS: u64 = 0x6fff_f000;
/// `DT_LOPROC`: reserved range for processor.
pub const DT_LOPROC: u64 = 0x7000_0000;
/// `DT_HIPROC`: specific dynamic array tags.
pub const DT_HIPROC: u64 = 0x7fff_ffff;

// some other useful tags
/// `DT_GNU_HASH`: address of GNU hash table.
pub const DT_GNU_HASH: u64 = 0x6fff_fef5;
/// `DT_RELACOUNT`: if present, number of RELATIVE.
pub const DT_RELACOUNT: u64 = 0x6fff_fff9;
/// `DT_RELCOUNT`: relocs, which must come first.
pub const DT_RELCOUNT: u64 = 0x6fff_fffa;
/// `DT_FLAGS_1`.
pub const DT_FLAGS_1: u64 = 0x6fff_fffb;

// Dynamic Flags - DT_FLAGS .dynamic entry
/// `DF_ORIGIN`.
pub const DF_ORIGIN: u64 = 0x0000_0001;
/// `DF_SYMBOLIC`.
pub const DF_SYMBOLIC: u64 = 0x0000_0002;
/// `DF_TEXTREL`.
pub const DF_TEXTREL: u64 = 0x0000_0004;
/// `DF_BIND_NOW`.
pub const DF_BIND_NOW: u64 = 0x0000_0008;
/// `DF_STATIC_TLS`.
pub const DF_STATIC_TLS: u64 = 0x0000_0010;

// Dynamic Flags - DT_FLAGS_1 .dynamic entry
/// `DF_1_NOW`.
pub const DF_1_NOW: u64 = 0x0000_0001;
/// `DF_1_GLOBAL`.
pub const DF_1_GLOBAL: u64 = 0x0000_0002;
/// `DF_1_GROUP`.
pub const DF_1_GROUP: u64 = 0x0000_0004;
/// `DF_1_NODELETE`.
pub const DF_1_NODELETE: u64 = 0x0000_0008;
/// `DF_1_LOADFLTR`.
pub const DF_1_LOADFLTR: u64 = 0x0000_0010;
/// `DF_1_INITFIRST`.
pub const DF_1_INITFIRST: u64 = 0x0000_0020;
/// `DF_1_NOOPEN`.
pub const DF_1_NOOPEN: u64 = 0x0000_0040;
/// `DF_1_ORIGIN`.
pub const DF_1_ORIGIN: u64 = 0x0000_0080;
/// `DF_1_DIRECT`.
pub const DF_1_DIRECT: u64 = 0x0000_0100;
/// `DF_1_TRANS`.
pub const DF_1_TRANS: u64 = 0x0000_0200;
/// `DF_1_INTERPOSE`.
pub const DF_1_INTERPOSE: u64 = 0x0000_0400;
/// `DF_1_NODEFLIB`.
pub const DF_1_NODEFLIB: u64 = 0x0000_0800;
/// `DF_1_NODUMP`.
pub const DF_1_NODUMP: u64 = 0x0000_1000;
/// `DF_1_CONLFAT`.
pub const DF_1_CONLFAT: u64 = 0x0000_2000;
/// `DF_1_ENDFILTEE`.
pub const DF_1_ENDFILTEE: u64 = 0x0000_4000;
/// `DF_1_DISPRELDNE`.
pub const DF_1_DISPRELDNE: u64 = 0x0000_8000;
/// `DF_1_DISPRELPND`.
pub const DF_1_DISPRELPND: u64 = 0x0001_0000;
/// `DF_1_NODIRECT`.
pub const DF_1_NODIRECT: u64 = 0x0002_0000;
/// `DF_1_IGNMULDEF`.
pub const DF_1_IGNMULDEF: u64 = 0x0004_0000;
/// `DF_1_NOKSYMS`.
pub const DF_1_NOKSYMS: u64 = 0x0008_0000;
/// `DF_1_NOHDR`.
pub const DF_1_NOHDR: u64 = 0x0010_0000;
/// `DF_1_EDITED`.
pub const DF_1_EDITED: u64 = 0x0020_0000;
/// `DF_1_NORELOC`.
pub const DF_1_NORELOC: u64 = 0x0040_0000;
/// `DF_1_SYMINTPOSE`.
pub const DF_1_SYMINTPOSE: u64 = 0x0080_0000;
/// `DF_1_GLOBAUDIT`.
pub const DF_1_GLOBAUDIT: u64 = 0x0100_0000;
/// `DF_1_SINGLETON`.
pub const DF_1_SINGLETON: u64 = 0x0200_0000;
/// `DF_1_PIE`.
pub const DF_1_PIE: u64 = 0x0800_0000;

// Values for n_type.
/// `NT_PRSTATUS`: Process status.
pub const NT_PRSTATUS: u32 = 1;
/// `NT_FPREGSET`: Floating point registers.
pub const NT_FPREGSET: u32 = 2;
/// `NT_PRPSINFO`: Process state info.
pub const NT_PRPSINFO: u32 = 3;

// OpenBSD's core file notes. The note name is "OpenBSD" for what is global
// to the process and "OpenBSD@nn" for what belongs to the thread of id nn
// (such as its registers).
/// `NT_OPENBSD_PROF`: the binary is profiled.
pub const NT_OPENBSD_PROF: u32 = 2;

/// `NT_OPENBSD_PROCINFO`: note is a "elfcore_procinfo" structure.
pub const NT_OPENBSD_PROCINFO: u32 = 10;
/// `NT_OPENBSD_AUXV`: note is a bunch of Auxiliary Vectors, terminated by an AT_NULL entry.
pub const NT_OPENBSD_AUXV: u32 = 11;

/// `NT_OPENBSD_REGS`: note is a "reg" structure.
pub const NT_OPENBSD_REGS: u32 = 20;
/// `NT_OPENBSD_FPREGS`: note is a "fpreg" structure.
pub const NT_OPENBSD_FPREGS: u32 = 21;
/// `NT_OPENBSD_XFPREGS`.
pub const NT_OPENBSD_XFPREGS: u32 = 22;
/// `NT_OPENBSD_WCOOKIE`.
pub const NT_OPENBSD_WCOOKIE: u32 = 23;
/// `NT_OPENBSD_PACMASK`.
pub const NT_OPENBSD_PACMASK: u32 = 24;

/// `ELFCORE_PROCINFO_VERSION`: the `cpi_version` of `struct elfcore_procinfo`.
pub const ELFCORE_PROCINFO_VERSION: u32 = 1;

// The kernel's and ld.so's own items, which are not part of the ABI.

/// `ELF32_NO_ADDR`: indicates addr. not yet filled in.
pub const ELF32_NO_ADDR: u32 = !0;
/// `ELF64_NO_ADDR`: indicates addr. not yet filled in.
pub const ELF64_NO_ADDR: u64 = !0;

/// The `enum AuxID` members, with the C's spelling (`AUX_null`, `AUX_phdr`, ...).
#[allow(non_upper_case_globals)] // the C enumerators are lower case after the prefix
pub mod aux_id {
    /// `AUX_null`.
    pub const AUX_null: i32 = 0;
    /// `AUX_ignore`.
    pub const AUX_ignore: i32 = 1;
    /// `AUX_execfd`.
    pub const AUX_execfd: i32 = 2;
    /// `AUX_phdr`: `&phdr[0]`.
    pub const AUX_phdr: i32 = 3;
    /// `AUX_phent`: `sizeof(phdr[0])`.
    pub const AUX_phent: i32 = 4;
    /// `AUX_phnum`: # phdr entries.
    pub const AUX_phnum: i32 = 5;
    /// `AUX_pagesz`: PAGESIZE.
    pub const AUX_pagesz: i32 = 6;
    /// `AUX_base`: base addr for ld.so or static PIE.
    pub const AUX_base: i32 = 7;
    /// `AUX_flags`: processor flags.
    pub const AUX_flags: i32 = 8;
    /// `AUX_entry`: a.out entry.
    pub const AUX_entry: i32 = 9;
    /// `AUX_hwcap`: processor flags.
    pub const AUX_hwcap: i32 = 25;
    /// `AUX_hwcap2`: processor flags (continued).
    pub const AUX_hwcap2: i32 = 26;
    /// `AUX_sun_uid`: euid.
    pub const AUX_sun_uid: i32 = 2000;
    /// `AUX_sun_ruid`: ruid.
    pub const AUX_sun_ruid: i32 = 2001;
    /// `AUX_sun_gid`: egid.
    pub const AUX_sun_gid: i32 = 2002;
    /// `AUX_sun_rgid`: rgid.
    pub const AUX_sun_rgid: i32 = 2003;
    /// `AUX_openbsd_timekeep`: userland clock_gettime.
    pub const AUX_openbsd_timekeep: i32 = 4000;
    /// `AUX_openbsd_execpath`: realpath'd executable path.
    pub const AUX_openbsd_execpath: i32 = 4001;
}

/// `ELFSIZE`: the machine's `ARCH_ELFSIZE`, since no file of the kernel sets its own.
pub const ELFSIZE: usize = <Machine as MachineExec>::ARCH_ELFSIZE;

/// `ELFCLASS` (`ELFSIZE == 64`).
pub const ELFCLASS: u8 = ELFCLASS64;
/// `ELF_NO_ADDR` (`ELFSIZE == 64`).
pub const ELF_NO_ADDR: u64 = ELF64_NO_ADDR;

/// `ELF_AUX_ENTRIES`: how many entries are in the `AuxInfo` array we pass to the process.
pub const ELF_AUX_ENTRIES: usize = 12;
/// `ELF_AUX_WORDS`: the same, in pointer-sized words of the new stack.
pub const ELF_AUX_WORDS: usize = size_of::<AuxInfo>() * ELF_AUX_ENTRIES / size_of::<usize>();

/// `ELFROUNDSIZE`: `sizeof(Elf_Word)`.
pub const ELFROUNDSIZE: usize = size_of::<ElfWord>();

/// `ELF_TARG_VER`: the ver for which this code is intended.
pub const ELF_TARG_VER: u32 = 1;

/// `Elf32_Addr`: unsigned program address.
pub type Elf32Addr = u32;
/// `Elf32_Off`: unsigned file offset.
pub type Elf32Off = u32;
/// `Elf32_Sword`: signed large integer.
pub type Elf32Sword = i32;
/// `Elf32_Word`: unsigned large integer.
pub type Elf32Word = u32;
/// `Elf32_Half`: unsigned medium integer.
pub type Elf32Half = u16;
/// `Elf32_Lword`.
pub type Elf32Lword = u64;
/// `Elf64_Addr`.
pub type Elf64Addr = u64;
/// `Elf64_Off`.
pub type Elf64Off = u64;
/// `Elf64_Shalf`.
pub type Elf64Shalf = i32;
/// `Elf64_Sword`.
pub type Elf64Sword = i32;
/// `Elf64_Word`.
pub type Elf64Word = u32;
/// `Elf64_Sxword`.
pub type Elf64Sxword = i64;
/// `Elf64_Xword`.
pub type Elf64Xword = u64;
/// `Elf64_Lword`.
pub type Elf64Lword = u64;
/// `Elf64_Half`.
pub type Elf64Half = u16;

/// `Elf32_Ehdr` (`struct elfhdr`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Ehdr {
    /// `e_ident`: ELF Identification.
    pub e_ident: [u8; EI_NIDENT],
    /// `e_type`: object file type.
    pub e_type: Elf32Half,
    /// `e_machine`: machine.
    pub e_machine: Elf32Half,
    /// `e_version`: object file version.
    pub e_version: Elf32Word,
    /// `e_entry`: virtual entry point.
    pub e_entry: Elf32Addr,
    /// `e_phoff`: program header table offset.
    pub e_phoff: Elf32Off,
    /// `e_shoff`: section header table offset.
    pub e_shoff: Elf32Off,
    /// `e_flags`: processor-specific flags.
    pub e_flags: Elf32Word,
    /// `e_ehsize`: ELF header size.
    pub e_ehsize: Elf32Half,
    /// `e_phentsize`: program header entry size.
    pub e_phentsize: Elf32Half,
    /// `e_phnum`: number of program header entries.
    pub e_phnum: Elf32Half,
    /// `e_shentsize`: section header entry size.
    pub e_shentsize: Elf32Half,
    /// `e_shnum`: number of section header entries.
    pub e_shnum: Elf32Half,
    /// `e_shstrndx`: section header table's "section header string table" entry offset.
    pub e_shstrndx: Elf32Half,
}

/// `Elf64_Ehdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Ehdr {
    /// `e_ident`: Id bytes.
    pub e_ident: [u8; EI_NIDENT],
    /// `e_type`: file type.
    pub e_type: Elf64Half,
    /// `e_machine`: machine type.
    pub e_machine: Elf64Half,
    /// `e_version`: version number.
    pub e_version: Elf64Word,
    /// `e_entry`: entry point.
    pub e_entry: Elf64Addr,
    /// `e_phoff`: Program hdr offset.
    pub e_phoff: Elf64Off,
    /// `e_shoff`: Section hdr offset.
    pub e_shoff: Elf64Off,
    /// `e_flags`: Processor flags.
    pub e_flags: Elf64Word,
    /// `e_ehsize`: sizeof ehdr.
    pub e_ehsize: Elf64Half,
    /// `e_phentsize`: Program header entry size.
    pub e_phentsize: Elf64Half,
    /// `e_phnum`: Number of program headers.
    pub e_phnum: Elf64Half,
    /// `e_shentsize`: Section header entry size.
    pub e_shentsize: Elf64Half,
    /// `e_shnum`: Number of section headers.
    pub e_shnum: Elf64Half,
    /// `e_shstrndx`: String table index.
    pub e_shstrndx: Elf64Half,
}

/// `Elf32_Shdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Shdr {
    /// `sh_name`: name - index into section header string table section.
    pub sh_name: Elf32Word,
    /// `sh_type`: type.
    pub sh_type: Elf32Word,
    /// `sh_flags`: flags.
    pub sh_flags: Elf32Word,
    /// `sh_addr`: address.
    pub sh_addr: Elf32Addr,
    /// `sh_offset`: file offset.
    pub sh_offset: Elf32Off,
    /// `sh_size`: section size.
    pub sh_size: Elf32Word,
    /// `sh_link`: section header table index link.
    pub sh_link: Elf32Word,
    /// `sh_info`: extra information.
    pub sh_info: Elf32Word,
    /// `sh_addralign`: address alignment.
    pub sh_addralign: Elf32Word,
    /// `sh_entsize`: section entry size.
    pub sh_entsize: Elf32Word,
}

/// `Elf64_Shdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Shdr {
    /// `sh_name`: section name.
    pub sh_name: Elf64Word,
    /// `sh_type`: section type.
    pub sh_type: Elf64Word,
    /// `sh_flags`: section flags.
    pub sh_flags: Elf64Xword,
    /// `sh_addr`: virtual address.
    pub sh_addr: Elf64Addr,
    /// `sh_offset`: file offset.
    pub sh_offset: Elf64Off,
    /// `sh_size`: section size.
    pub sh_size: Elf64Xword,
    /// `sh_link`: link to another.
    pub sh_link: Elf64Word,
    /// `sh_info`: misc info.
    pub sh_info: Elf64Word,
    /// `sh_addralign`: memory alignment.
    pub sh_addralign: Elf64Xword,
    /// `sh_entsize`: table entry size.
    pub sh_entsize: Elf64Xword,
}

/// `Elf32_Sym` (`struct elf32_sym`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Sym {
    /// `st_name`: name - index into string table.
    pub st_name: Elf32Word,
    /// `st_value`: symbol value.
    pub st_value: Elf32Addr,
    /// `st_size`: symbol size.
    pub st_size: Elf32Word,
    /// `st_info`: type and binding.
    pub st_info: u8,
    /// `st_other`: 0 - no defined meaning.
    pub st_other: u8,
    /// `st_shndx`: section header index.
    pub st_shndx: Elf32Half,
}

/// `Elf64_Sym`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Sym {
    /// `st_name`: Symbol name index in str table.
    pub st_name: Elf64Word,
    /// `st_info`: type / binding attrs.
    pub st_info: u8,
    /// `st_other`: unused.
    pub st_other: u8,
    /// `st_shndx`: section index of symbol.
    pub st_shndx: Elf64Half,
    /// `st_value`: value of symbol.
    pub st_value: Elf64Addr,
    /// `st_size`: size of symbol.
    pub st_size: Elf64Xword,
}

/// `Elf32_Rel`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Rel {
    /// `r_offset`: offset of relocation.
    pub r_offset: Elf32Addr,
    /// `r_info`: symbol table index and type.
    pub r_info: Elf32Word,
}

/// `Elf32_Rela`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Rela {
    /// `r_offset`: offset of relocation.
    pub r_offset: Elf32Addr,
    /// `r_info`: symbol table index and type.
    pub r_info: Elf32Word,
    /// `r_addend`.
    pub r_addend: Elf32Sword,
}

/// `Elf64_Rel`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Rel {
    /// `r_offset`: where to do it.
    pub r_offset: Elf64Addr,
    /// `r_info`: index & type of relocation.
    pub r_info: Elf64Xword,
}

/// `Elf64_Rela`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Rela {
    /// `r_offset`: where to do it.
    pub r_offset: Elf64Addr,
    /// `r_info`: index & type of relocation.
    pub r_info: Elf64Xword,
    /// `r_addend`: adjustment value.
    pub r_addend: Elf64Sxword,
}

/// `Elf32_Relr`.
pub type Elf32Relr = Elf32Word;
/// `Elf64_Relr`.
pub type Elf64Relr = Elf64Xword;

/// `Elf32_Phdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Phdr {
    /// `p_type`: segment type.
    pub p_type: Elf32Word,
    /// `p_offset`: segment offset.
    pub p_offset: Elf32Off,
    /// `p_vaddr`: virtual address of segment.
    pub p_vaddr: Elf32Addr,
    /// `p_paddr`: physical address - ignored?.
    pub p_paddr: Elf32Addr,
    /// `p_filesz`: number of bytes in file for seg.
    pub p_filesz: Elf32Word,
    /// `p_memsz`: number of bytes in mem. for seg.
    pub p_memsz: Elf32Word,
    /// `p_flags`: flags.
    pub p_flags: Elf32Word,
    /// `p_align`: memory alignment.
    pub p_align: Elf32Word,
}

/// `Elf64_Phdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Phdr {
    /// `p_type`: entry type.
    pub p_type: Elf64Word,
    /// `p_flags`: flags.
    pub p_flags: Elf64Word,
    /// `p_offset`: offset.
    pub p_offset: Elf64Off,
    /// `p_vaddr`: virtual address.
    pub p_vaddr: Elf64Addr,
    /// `p_paddr`: physical address.
    pub p_paddr: Elf64Addr,
    /// `p_filesz`: file size.
    pub p_filesz: Elf64Xword,
    /// `p_memsz`: memory size.
    pub p_memsz: Elf64Xword,
    /// `p_align`: memory & file alignment.
    pub p_align: Elf64Xword,
}

/// `Elf32_Dyn`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Dyn {
    /// `d_tag`: controls meaning of d_val.
    pub d_tag: Elf32Sword,
    /// `d_un`: the union of `d_val` (multiple meanings, see `d_tag`) and `d_ptr` (program virtual
    /// address); both are words of this size.
    pub d_un: Elf32Word,
}

/// `Elf64_Dyn`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Dyn {
    /// `d_tag`: controls meaning of d_val.
    pub d_tag: Elf64Xword,
    /// `d_un`: the union of `d_val` (multiple meanings, see `d_tag`) and `d_ptr` (program virtual
    /// address); both are words of this size.
    pub d_un: Elf64Xword,
}

/// `Elf32_Nhdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Nhdr {
    /// `n_namesz`.
    pub n_namesz: Elf32Word,
    /// `n_descsz`.
    pub n_descsz: Elf32Word,
    /// `n_type`.
    pub n_type: Elf32Word,
}

/// `Elf64_Nhdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Nhdr {
    /// `n_namesz`.
    pub n_namesz: Elf64Word,
    /// `n_descsz`.
    pub n_descsz: Elf64Word,
    /// `n_type`.
    pub n_type: Elf64Word,
}

/// `Elf32_Note`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf32Note {
    /// `namesz`.
    pub namesz: Elf32Word,
    /// `descsz`.
    pub descsz: Elf32Word,
    /// `type`.
    pub r#type: Elf32Word,
}

/// `Elf64_Note`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Elf64Note {
    /// `namesz`.
    pub namesz: Elf64Word,
    /// `descsz`.
    pub descsz: Elf64Word,
    /// `type`.
    pub r#type: Elf64Word,
}

/// `struct elfcore_procinfo`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ElfcoreProcinfo {
    /// `cpi_version`: elfcore_procinfo version.
    pub cpi_version: u32,
    /// `cpi_cpisize`: sizeof(elfcore_procinfo).
    pub cpi_cpisize: u32,
    /// `cpi_signo`: killing signal.
    pub cpi_signo: u32,
    /// `cpi_sigcode`: signal code.
    pub cpi_sigcode: u32,
    /// `cpi_sigpend`: pending signals.
    pub cpi_sigpend: u32,
    /// `cpi_sigmask`: blocked signals.
    pub cpi_sigmask: u32,
    /// `cpi_sigignore`: ignored signals.
    pub cpi_sigignore: u32,
    /// `cpi_sigcatch`: signals being caught by user.
    pub cpi_sigcatch: u32,
    /// `cpi_pid`: process ID.
    pub cpi_pid: i32,
    /// `cpi_ppid`: parent process ID.
    pub cpi_ppid: i32,
    /// `cpi_pgrp`: process group ID.
    pub cpi_pgrp: i32,
    /// `cpi_sid`: session ID.
    pub cpi_sid: i32,
    /// `cpi_ruid`: real user ID.
    pub cpi_ruid: u32,
    /// `cpi_euid`: effective user ID.
    pub cpi_euid: u32,
    /// `cpi_svuid`: saved user ID.
    pub cpi_svuid: u32,
    /// `cpi_rgid`: real group ID.
    pub cpi_rgid: u32,
    /// `cpi_egid`: effective group ID.
    pub cpi_egid: u32,
    /// `cpi_svgid`: saved group ID.
    pub cpi_svgid: u32,
    /// `cpi_name`: copy of pr->ps_comm.
    pub cpi_name: [i8; 32],
}

/// `Aux32Info`: one entry of a 32-bit auxiliary vector.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Aux32Info {
    /// `au_id`: 32-bit id.
    pub au_id: Elf32Sword,
    /// `au_v`: 32-bit value.
    pub au_v: Elf32Word,
}

/// `Aux64Info`: one entry of the auxiliary vector `exec_elf_fixup` copies out after the
/// environment.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Aux64Info {
    /// `au_id`: 32-bit id.
    pub au_id: Elf64Shalf,
    /// The four bytes the C compiler leaves between `au_id` and `au_v`, zero.
    pub _pad: u32,
    /// `au_v`: 64-bit value.
    pub au_v: Elf64Xword,
}

impl Aux64Info {
    /// The entry's bytes as `copyout` writes them: `au_id`, four zero bytes, `au_v`.
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.au_id.to_ne_bytes());
        out[8..16].copy_from_slice(&self.au_v.to_ne_bytes());
        out
    }
}

/// `Elf_Ehdr` (`ELFSIZE == 64`).
pub type ElfEhdr = Elf64Ehdr;
/// `Elf_Phdr` (`ELFSIZE == 64`).
pub type ElfPhdr = Elf64Phdr;
/// `Elf_Shdr` (`ELFSIZE == 64`).
pub type ElfShdr = Elf64Shdr;
/// `Elf_Sym` (`ELFSIZE == 64`).
pub type ElfSym = Elf64Sym;
/// `Elf_Rel` (`ELFSIZE == 64`).
pub type ElfRel = Elf64Rel;
/// `Elf_RelA` (`ELFSIZE == 64`).
pub type ElfRelA = Elf64Rela;
/// `Elf_Relr` (`ELFSIZE == 64`).
pub type ElfRelr = Elf64Relr;
/// `Elf_Dyn` (`ELFSIZE == 64`).
pub type ElfDyn = Elf64Dyn;
/// `Elf_Half` (`ELFSIZE == 64`).
pub type ElfHalf = Elf64Half;
/// `Elf_Word` (`ELFSIZE == 64`).
pub type ElfWord = Elf64Word;
/// `Elf_Sword` (`ELFSIZE == 64`).
pub type ElfSword = Elf64Sword;
/// `Elf_Addr` (`ELFSIZE == 64`).
pub type ElfAddr = Elf64Addr;
/// `Elf_Off` (`ELFSIZE == 64`).
pub type ElfOff = Elf64Off;
/// `Elf_Nhdr` (`ELFSIZE == 64`).
pub type ElfNhdr = Elf64Nhdr;
/// `Elf_Note` (`ELFSIZE == 64`).
pub type ElfNote = Elf64Note;
/// `AuxInfo` (`ELFSIZE == 64`).
pub type AuxInfo = Aux64Info;

/// What `IS_ELF` reads of a header: its identification bytes.
pub trait ElfIdent {
    /// `e_ident`.
    fn e_ident(&self) -> &[u8; EI_NIDENT];
}

/// `IS_ELF(ehdr)`.
pub fn is_elf<E: ElfIdent>(ehdr: &E) -> bool {
    let ident = ehdr.e_ident();
    ident[EI_MAG0] == ELFMAG0
        && ident[EI_MAG1] == ELFMAG1
        && ident[EI_MAG2] == ELFMAG2
        && ident[EI_MAG3] == ELFMAG3
}

impl ElfIdent for Elf32Ehdr {
    fn e_ident(&self) -> &[u8; EI_NIDENT] {
        &self.e_ident
    }
}

impl ElfIdent for Elf64Ehdr {
    fn e_ident(&self) -> &[u8; EI_NIDENT] {
        &self.e_ident
    }
}

/// `ELF32_ST_BIND(x)`: the binding of a symbol's `st_info`.
pub const fn elf32_st_bind(x: u8) -> u8 {
    x >> 4
}

/// `ELF32_ST_TYPE(x)`: the type of a symbol's `st_info`.
pub const fn elf32_st_type(x: u8) -> u8 {
    x & 0xf
}

/// `ELF32_ST_INFO(b, t)`: the `st_info` of a binding and a type.
pub const fn elf32_st_info(b: u8, t: u8) -> u8 {
    (b << 4) + (t & 0xf)
}

/// `ELF64_ST_BIND(x)`.
pub const fn elf64_st_bind(x: u8) -> u8 {
    x >> 4
}

/// `ELF64_ST_TYPE(x)`.
pub const fn elf64_st_type(x: u8) -> u8 {
    x & 0xf
}

/// `ELF64_ST_INFO(b, t)`.
pub const fn elf64_st_info(b: u8, t: u8) -> u8 {
    (b << 4) + (t & 0xf)
}

/// `ELF_ST_VISIBILITY(v)`: the visibility of a symbol's `st_other`.
pub const fn elf_st_visibility(v: u8) -> u8 {
    v & 0x3
}

/// `ELF32_ST_VISIBILITY` and `ELF64_ST_VISIBILITY`: `ELF_ST_VISIBILITY`.
pub use self::elf_st_visibility as elf32_st_visibility;
pub use self::elf_st_visibility as elf64_st_visibility;

/// `ELF32_R_SYM(i)`: the symbol table index of a relocation's `r_info`.
pub const fn elf32_r_sym(i: Elf32Word) -> Elf32Word {
    i >> 8
}

/// `ELF32_R_TYPE(i)`: the type of a relocation's `r_info`.
pub const fn elf32_r_type(i: Elf32Word) -> u8 {
    i as u8
}

/// `ELF32_R_INFO(s, t)`: the `r_info` of a symbol index and a type.
pub const fn elf32_r_info(s: Elf32Word, t: Elf32Word) -> Elf32Word {
    (s << 8) + (t as u8 as Elf32Word)
}

/// `ELF64_R_SYM(info)`.
pub const fn elf64_r_sym(info: Elf64Xword) -> Elf64Xword {
    info >> 32
}

/// `ELF64_R_TYPE(info)`.
pub const fn elf64_r_type(info: Elf64Xword) -> Elf64Xword {
    info & 0xffff_ffff
}

/// `ELF64_R_INFO(s, t)`.
pub const fn elf64_r_info(s: Elf64Xword, t: Elf64Xword) -> Elf64Xword {
    (s << 32) + (t as u32 as Elf64Xword)
}

/// `ELF_R_SYM`, `ELF_R_TYPE`, `ELF_R_INFO`, `ELF_ST_BIND`, `ELF_ST_TYPE` and `ELF_ST_INFO`
/// (`ELFSIZE == 64`).
pub use self::{
    elf64_r_info as elf_r_info, elf64_r_sym as elf_r_sym, elf64_r_type as elf_r_type,
    elf64_st_bind as elf_st_bind, elf64_st_info as elf_st_info, elf64_st_type as elf_st_type,
};

/// `elfround(x)`: `roundup(x, ELFROUNDSIZE)`.
pub const fn elfround(x: usize) -> usize {
    x.div_ceil(ELFROUNDSIZE) * ELFROUNDSIZE
}

const _: () = {
    // The `Elf_*` names above are the 64-bit ones.
    assert!(ELFSIZE == 64);
    assert!(size_of::<Elf32Ehdr>() == 52);
    assert!(size_of::<Elf64Ehdr>() == 64);
    assert!(size_of::<Elf32Shdr>() == 40);
    assert!(size_of::<Elf64Shdr>() == 64);
    assert!(size_of::<Elf32Sym>() == 16);
    assert!(size_of::<Elf64Sym>() == 24);
    assert!(size_of::<Elf32Rel>() == 8);
    assert!(size_of::<Elf32Rela>() == 12);
    assert!(size_of::<Elf64Rel>() == 16);
    assert!(size_of::<Elf64Rela>() == 24);
    assert!(size_of::<Elf32Phdr>() == 32);
    assert!(size_of::<Elf64Phdr>() == 56);
    assert!(size_of::<Elf32Dyn>() == 8);
    assert!(size_of::<Elf64Dyn>() == 16);
    assert!(size_of::<Elf32Nhdr>() == 12);
    assert!(size_of::<Elf64Nhdr>() == 12);
    assert!(size_of::<Elf32Note>() == 12);
    assert!(size_of::<Elf64Note>() == 12);
    assert!(size_of::<ElfcoreProcinfo>() == 104);
    assert!(size_of::<Aux32Info>() == 8);
    assert!(size_of::<Aux64Info>() == 16);
    assert!(ELF_AUX_WORDS == 24);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::reftest::{assert_complete, assert_defines};

    #[test]
    fn the_magic_is_what_is_elf_reads() {
        let mut eh = Elf64Ehdr {
            e_ident: [0; EI_NIDENT],
            e_type: ET_DYN,
            e_machine: EM_AARCH64,
            e_version: EV_CURRENT,
            e_entry: 0,
            e_phoff: 0,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: 0,
            e_phentsize: 0,
            e_phnum: 0,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        };
        assert!(!is_elf(&eh));
        eh.e_ident[..SELFMAG].copy_from_slice(&ELFMAG);
        assert!(is_elf(&eh));
        assert_eq!(ELFMAG, *b"\x7fELF");
        eh.e_ident[EI_MAG3] = b'f';
        assert!(!is_elf(&eh));
    }

    #[test]
    fn a_symbols_info_splits_into_binding_and_type() {
        let info = elf64_st_info(STB_WEAK, STT_FUNC);
        assert_eq!(info, 0x22);
        assert_eq!(elf_st_bind(info), STB_WEAK);
        assert_eq!(elf_st_type(info), STT_FUNC);
        assert_eq!(elf32_st_info(STB_HIPROC, 0xff), 0xff);
        assert_eq!(elf32_st_bind(0xff), STB_HIPROC);
        assert_eq!(elf32_st_type(0xff), STT_HIPROC);
        assert_eq!(elf64_st_visibility(0xfe), STV_HIDDEN);
    }

    #[test]
    fn a_relocations_info_splits_into_symbol_and_type() {
        // R_AARCH64_RELATIVE (1027) against symbol 5.
        let info = elf64_r_info(5, 1027);
        assert_eq!(info, 0x5_0000_0403);
        assert_eq!(elf_r_sym(info), 5);
        assert_eq!(elf_r_type(info), 1027);
        // The type is cut to its width as the C's casts cut it.
        assert_eq!(elf64_r_info(1, 0x1_0000_0002), 0x1_0000_0002);
        assert_eq!(elf32_r_info(5, 0x107), 0x507);
        assert_eq!(elf32_r_sym(0x507), 5);
        assert_eq!(elf32_r_type(0x507), 7);
    }

    #[test]
    fn notes_are_padded_to_a_word() {
        assert_eq!(ELFROUNDSIZE, 4);
        assert_eq!(elfround(0), 0);
        assert_eq!(elfround(1), 4);
        assert_eq!(elfround(8), 8);
        // "OpenBSD" with its NUL is 8 bytes, "EmiBSD" with its NUL 7.
        assert_eq!(elfround(7), 8);
    }

    #[test]
    fn an_aux_entry_is_written_whole() {
        let a = Aux64Info {
            au_id: aux_id::AUX_pagesz,
            _pad: 0,
            au_v: 0x1000,
        };
        let mut bytes = [0u8; 16];
        bytes[0] = 6;
        bytes[9] = 0x10;
        assert_eq!(a.to_bytes(), bytes);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/exec_elf.h");
        let mut names = assert_defines!(defs;
            EI_MAG0, EI_MAG1, EI_MAG2, EI_MAG3, EI_CLASS, EI_DATA, EI_VERSION, EI_OSABI,
            EI_ABIVERSION, EI_PAD, EI_NIDENT, ELFMAG0, SELFMAG, ELFCLASSNONE, ELFCLASS32,
            ELFCLASS64, ELFCLASSNUM, ELFDATANONE, ELFDATA2LSB, ELFDATA2MSB, ELFDATANUM,
            ELFOSABI_SYSV, ELFOSABI_HPUX, ELFOSABI_NETBSD, ELFOSABI_LINUX, ELFOSABI_HURD,
            ELFOSABI_86OPEN, ELFOSABI_SOLARIS, ELFOSABI_MONTEREY, ELFOSABI_IRIX,
            ELFOSABI_FREEBSD, ELFOSABI_TRU64, ELFOSABI_MODESTO, ELFOSABI_OPENBSD, ELFOSABI_ARM,
            ELFOSABI_STANDALONE, ET_NONE, ET_REL, ET_EXEC, ET_DYN, ET_CORE, ET_NUM, ET_LOPROC,
            ET_HIPROC, EM_NONE, EM_M32, EM_SPARC, EM_386, EM_68K, EM_88K, EM_486, EM_860,
            EM_MIPS, EM_MIPS_RS4_BE, EM_SPARC64, EM_PARISC, EM_SPARC32PLUS, EM_PPC, EM_PPC64,
            EM_ARM, EM_ALPHA, EM_SH, EM_SPARCV9, EM_IA_64, EM_AMD64, EM_X86_64, EM_VAX,
            EM_AARCH64, EM_RISCV, EM_ALPHA_EXP, EM__LAST__, EV_NONE, EV_CURRENT, EV_NUM,
            PN_XNUM, SHN_UNDEF, SHN_LORESERVE, SHN_LOPROC, SHN_HIPROC, SHN_ABS, SHN_COMMON,
            SHN_XINDEX, SHN_HIRESERVE, SHT_NULL, SHT_PROGBITS, SHT_SYMTAB, SHT_STRTAB, SHT_RELA,
            SHT_HASH, SHT_DYNAMIC, SHT_NOTE, SHT_NOBITS, SHT_REL, SHT_SHLIB, SHT_DYNSYM,
            SHT_NUM, SHT_INIT_ARRAY, SHT_FINI_ARRAY, SHT_PREINIT_ARRAY, SHT_GROUP,
            SHT_SYMTAB_SHNDX, SHT_RELR, SHT_LOOS, SHT_SUNW_dof, SHT_GNU_LIBLIST, SHT_SUNW_move,
            SHT_SUNW_syminfo, SHT_SUNW_verdef, SHT_SUNW_verneed, SHT_SUNW_versym, SHT_HIOS,
            SHT_LOPROC, SHT_HIPROC, SHT_LOUSER, SHT_HIUSER, SHT_GNU_HASH, SHF_WRITE, SHF_ALLOC,
            SHF_EXECINSTR, SHF_MERGE, SHF_STRINGS, SHF_INFO_LINK, SHF_LINK_ORDER,
            SHF_OS_NONCONFORMING, SHF_GROUP, SHF_TLS, SHF_COMPRESSED, SHF_MASKOS, SHF_MASKPROC,
            STN_UNDEF, STB_LOCAL, STB_GLOBAL, STB_WEAK, STB_NUM, STB_LOPROC, STB_HIPROC,
            STT_NOTYPE, STT_OBJECT, STT_FUNC, STT_SECTION, STT_FILE, STT_TLS, STT_LOPROC,
            STT_HIPROC, STV_DEFAULT, STV_INTERNAL, STV_HIDDEN, STV_PROTECTED, PT_NULL, PT_LOAD,
            PT_DYNAMIC, PT_INTERP, PT_NOTE, PT_SHLIB, PT_PHDR, PT_TLS, PT_LOOS, PT_HIOS,
            PT_LOPROC, PT_HIPROC, PT_GNU_EH_FRAME, PT_GNU_RELRO, PT_GNU_PROPERTY,
            PT_OPENBSD_MUTABLE, PT_OPENBSD_RANDOMIZE, PT_OPENBSD_WXNEEDED, PT_OPENBSD_NOBTCFI,
            PT_OPENBSD_SYSCALLS, PT_OPENBSD_BOOTDATA, PF_X, PF_W, PF_R, PF_MASKOS, PF_MASKPROC,
            PF_OPENBSD_MUTABLE, PF_ISVNODE, DT_NULL, DT_NEEDED, DT_PLTRELSZ, DT_PLTGOT, DT_HASH,
            DT_STRTAB, DT_SYMTAB, DT_RELA, DT_RELASZ, DT_RELAENT, DT_STRSZ, DT_SYMENT, DT_INIT,
            DT_FINI, DT_SONAME, DT_RPATH, DT_SYMBOLIC, DT_REL, DT_RELSZ, DT_RELENT, DT_PLTREL,
            DT_DEBUG, DT_TEXTREL, DT_JMPREL, DT_BIND_NOW, DT_INIT_ARRAY, DT_FINI_ARRAY,
            DT_INIT_ARRAYSZ, DT_FINI_ARRAYSZ, DT_RUNPATH, DT_FLAGS, DT_ENCODING,
            DT_PREINIT_ARRAY, DT_PREINIT_ARRAYSZ, DT_RELRSZ, DT_RELR, DT_RELRENT, DT_LOOS,
            DT_HIOS, DT_LOPROC, DT_HIPROC, DT_GNU_HASH, DT_RELACOUNT, DT_RELCOUNT, DT_FLAGS_1,
            DF_ORIGIN, DF_SYMBOLIC, DF_TEXTREL, DF_BIND_NOW, DF_STATIC_TLS, DF_1_NOW,
            DF_1_GLOBAL, DF_1_GROUP, DF_1_NODELETE, DF_1_LOADFLTR, DF_1_INITFIRST, DF_1_NOOPEN,
            DF_1_ORIGIN, DF_1_DIRECT, DF_1_TRANS, DF_1_INTERPOSE, DF_1_NODEFLIB, DF_1_NODUMP,
            DF_1_CONLFAT, DF_1_ENDFILTEE, DF_1_DISPRELDNE, DF_1_DISPRELPND, DF_1_NODIRECT,
            DF_1_IGNMULDEF, DF_1_NOKSYMS, DF_1_NOHDR, DF_1_EDITED, DF_1_NORELOC,
            DF_1_SYMINTPOSE, DF_1_GLOBAUDIT, DF_1_SINGLETON, DF_1_PIE, NT_PRSTATUS, NT_FPREGSET,
            NT_PRPSINFO, NT_OPENBSD_PROF, NT_OPENBSD_PROCINFO, NT_OPENBSD_AUXV, NT_OPENBSD_REGS,
            NT_OPENBSD_FPREGS, NT_OPENBSD_XFPREGS, NT_OPENBSD_WCOOKIE, NT_OPENBSD_PACMASK,
            ELFCORE_PROCINFO_VERSION, ELF_AUX_ENTRIES, ELF_TARG_VER, ELFCLASS,
        );
        // The section names, and the magic's letters, are text.
        let text: &[(&'static str, &str)] = &[
            ("ELF_BSS", ELF_BSS),
            ("ELF_DATA", ELF_DATA),
            ("ELF_CTF", ELF_CTF),
            ("ELF_DEBUG", ELF_DEBUG),
            ("ELF_DYNAMIC", ELF_DYNAMIC),
            ("ELF_DYNSTR", ELF_DYNSTR),
            ("ELF_DYNSYM", ELF_DYNSYM),
            ("ELF_FINI", ELF_FINI),
            ("ELF_GOT", ELF_GOT),
            ("ELF_HASH", ELF_HASH),
            ("ELF_INIT", ELF_INIT),
            ("ELF_REL_DATA", ELF_REL_DATA),
            ("ELF_REL_FINI", ELF_REL_FINI),
            ("ELF_REL_INIT", ELF_REL_INIT),
            ("ELF_REL_DYN", ELF_REL_DYN),
            ("ELF_REL_RODATA", ELF_REL_RODATA),
            ("ELF_REL_TEXT", ELF_REL_TEXT),
            ("ELF_RODATA", ELF_RODATA),
            ("ELF_SHSTRTAB", ELF_SHSTRTAB),
            ("ELF_STRTAB", ELF_STRTAB),
            ("ELF_SYMTAB", ELF_SYMTAB),
            ("ELF_TEXT", ELF_TEXT),
            ("ELF_OPENBSDRANDOMDATA", ELF_OPENBSDRANDOMDATA),
            ("ELF_OPENBSDMUTABLE", ELF_OPENBSDMUTABLE),
        ];
        for (name, value) in text {
            let quoted = std::format!("\"{value}\"");
            assert_eq!(defs.get(*name), Some(&quoted), "{name}");
            names.push(*name);
        }
        for (name, value) in [
            ("ELFMAG1", ELFMAG1),
            ("ELFMAG2", ELFMAG2),
            ("ELFMAG3", ELFMAG3),
        ] {
            let quoted = std::format!("'{}'", value as char);
            assert_eq!(defs.get(name), Some(&quoted), "{name}");
            names.push(name);
        }
        assert_eq!(defs["ELFMAG"], "\"\\177ELF\"");
        // Every define of the header is one of the above, or one of these: the include
        // guard, the names that follow `ELFSIZE` (aliases and re-exports here), and the
        // values the C writes with casts and `sizeof`.
        names.extend([
            "_SYS_EXEC_ELF_H_",
            "ELFMAG",
            "ELF32_ST_VISIBILITY",
            "ELF64_ST_VISIBILITY",
            "ELF32_NO_ADDR",
            "ELF64_NO_ADDR",
            "ELF_NO_ADDR",
            "ELFSIZE",
            "ELF_AUX_WORDS",
            "ELFROUNDSIZE",
            "Elf_Ehdr",
            "Elf_Phdr",
            "Elf_Shdr",
            "Elf_Sym",
            "Elf_Rel",
            "Elf_RelA",
            "Elf_Relr",
            "Elf_Dyn",
            "Elf_Half",
            "Elf_Word",
            "Elf_Sword",
            "Elf_Addr",
            "Elf_Off",
            "Elf_Nhdr",
            "Elf_Note",
            "ELF_R_SYM",
            "ELF_R_TYPE",
            "ELF_R_INFO",
            "ELF_ST_BIND",
            "ELF_ST_TYPE",
            "ELF_ST_INFO",
            "AuxInfo",
        ]);
        assert_complete(&defs, "", &names);
    }
}
/* </TESTS> */
