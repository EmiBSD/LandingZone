# Functions moved to the module of their C file

For the ../Kernel redesign. On 2026-11-19 the user decided that every C function lives in
the `.rs` file of its `.c` file (`reference/openbsd-src/sys/<dir>/<name>.c` ↔
`sys/<dir>/<name>.rs`), so the trace from C to Rust is one file to one file. The
functions below lived in another Rust file. They were moved without any change to their
logic; where callers named the old path, the old module re-exports the function, so no
caller changed.

If your tree was taken from LandingZone before these commits, look for each function at
"Was in" and expect it at "Is now in".

| C function | C file | Was in (Rust) | Is now in (Rust) | Commit |
|---|---|---|---|---|
| `pmap_update` | `sys/arch/arm64/arm64/pmap.c` | `sys/arch/arm64/mod.rs` (inline in the `Pmap` trait impl) | `sys/arch/arm64/arm64/pmap.rs`; `mod.rs` delegates to it | `30b22d2d` |
| `uvm_pagecmp` | `sys/uvm/uvm_page.c` | `sys/uvm/uvm_object.rs` | `sys/uvm/uvm_page.rs`; the `uvm_objtree` adapter in `uvm_object.rs` imports it | `30b22d2d` |
| `pf_anchor_compare` | `sys/net/pf_ruleset.c` | `sys/net/pfvar.rs` | `sys/net/pf_ruleset.rs`; the tree adapters stay in `pfvar.rs` and import it | `30b22d2d` |
| `nfsnode_cmp` | `sys/nfs/nfs_node.c` | `sys/nfs/nfsnode.rs` | `sys/nfs/nfs_node.rs`; the `NfsNodetree` adapter stays in `nfsnode.rs` and imports it | `30b22d2d` |
| `copyin` | `sys/kern/kern_subr.c` (`PMAP_CHECK_COPYIN`) | `sys/machine/copy.rs` | `sys/kern/kern_subr.rs`; `machine::copy` re-exports it | `979305c1` |
| `copyinstr` | `sys/kern/kern_subr.c` (`PMAP_CHECK_COPYIN`) | `sys/machine/copy.rs` | `sys/kern/kern_subr.rs`; `machine::copy` re-exports it | `979305c1` |
| `zcalloc` | `sys/lib/libsa/cread.c` | libz's own (`sys/lib/libz/zopenbsd.rs`) | pending | — |
| `zcfree` | `sys/lib/libsa/cread.c` | libz's own (`sys/lib/libz/zopenbsd.rs`) | pending | — |
| `read_inode` | `sys/lib/libsa/ufs2.c` | `sys/lib/libsa/ufs.rs` (one body, generic over the inode) | `sys/lib/libsa/ufs2.rs`, in `impl Dinode for Ufs2Dinode`; it calls `ufs.rs`'s body, and the reader calls it for FFS2 | `67f98102` |
| `chmod_inode` | `sys/lib/libsa/ufs2.c` | `sys/lib/libsa/ufs.rs` (one body, generic over the inode) | `sys/lib/libsa/ufs2.rs`, in `impl Dinode for Ufs2Dinode`; it calls `ufs.rs`'s body, and the reader calls it for FFS2 | `67f98102` |
| `block_map` | `sys/lib/libsa/ufs2.c` | `sys/lib/libsa/ufs.rs` (one body, generic over the inode) | `sys/lib/libsa/ufs2.rs`, in `impl Dinode for Ufs2Dinode`; it calls `ufs.rs`'s body, and the reader calls it for FFS2 | `67f98102` |
| `buf_read_file` | `sys/lib/libsa/ufs2.c` | `sys/lib/libsa/ufs.rs` (one body, generic over the inode) | `sys/lib/libsa/ufs2.rs`, in `impl Dinode for Ufs2Dinode`; it calls `ufs.rs`'s body, and the reader calls it for FFS2 | `67f98102` |
| `search_directory` | `sys/lib/libsa/ufs2.c` | `sys/lib/libsa/ufs.rs` (one body, generic over the inode) | `sys/lib/libsa/ufs2.rs`, in `impl Dinode for Ufs2Dinode`; it calls `ufs.rs`'s body, and the reader calls it for FFS2 | `67f98102` |

The five `ufs2.c` functions keep one body in `ufs.rs`: the C's two copies differ only in
types (`daddr_t` for `daddr32_t`, `struct ufs2_dinode` for `struct ufs1_dinode`, no
`(daddr32_t)` cast), and the user chose a named function per file that calls it.
`zcalloc` and `zcfree` are pending: libsa's versions sit on libsa's `alloc` and are chosen
by the linker in the C, which needs a change in how the Rust libz allocates.

Commits are on LandingZone's `td0` branch. `cargo xtask ports missing` lists any C function
that still has no `fn` in its own Rust file.
