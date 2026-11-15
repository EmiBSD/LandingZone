# debugger: lessons across runs

One line per lesson, appended at the end; task state goes in HANDOFF.md
(`.claude/rules/subagents.md`, "Memory"). Committed with the tree.
- hvf `Assertion failed: (isv)` in hvf_handle_exception = the guest touched MMIO with a writeback/pair insn; a kernel stack overflowing its u-area into an adjacent device mapping looks exactly like that (TD1: ECAM ends where the first u-area begins).
- QEMU `-trace hvf_data_abort -D file` gives va/pa/isv of the aborting access under hvf; hvf has no `-d int`, so for a recursion put a temporary sp-near-p_addr check at the top of do_el1h_sync/arm_cpu_irq that prints esr/far/elr/spsr and the stack's text addresses, then `xtask symbolize`.
- `-cpu host` on Apple silicon has PAN+EPAN (FEAT_PAN3), TCG's cortex-a72 has none: with EPAN, PAN faults privileged reads of any EL0-executable page (UXN clear), even AP 00; Limine's HHDM blocks are like that, fixed by UXN in pmap_bootstrap (df8508cf).
- xtask buffers the guest's serial until the run ends: a hung debug boot shows only at its timeout (EMIBSD_TIMEOUT_SCALE below 1 does not shorten the 180 s floor).
