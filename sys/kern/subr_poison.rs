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
 * Copyright (c) 2013 Ted Unangst <tedu@openbsd.org>
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
//! Poisoning of free memory: `subr_poison.c`. A known pattern is written into the start of a
//! free object, so that a write after the free is found when the object is handed out again.
//!
//! Upstream: sys/kern/subr_poison.c @ 3ce1f3f79392
//!
//! Status: `ported`.
//!
//! ## Deviations
//! - `sys/conf/files` compiles the C file under `option DIAGNOSTIC` only. The module is always
//!   compiled, so the host tests run it; its callers (the pool and malloc(9)) are under
//!   feature `diagnostic`, as in the C.
//! - `poison_check` returns `Some((index, poison))` where the C returns 1 after writing
//!   `*pidx` and `*pval`, and `None` for its 0.
//! - `poison_value` takes the address as a `usize`.
//! - `DEADBEEF0` and `DEADBEEF1` are a machine's own patterns; neither amd64 nor arm64
//!   defines them, so the two constants are the C's defaults.

use crate::sys::param::PAGE_SHIFT;

/// `POISON0`: the first known text.
const POISON0: u32 = 0xdead_beef;
/// `POISON1`: the second known text.
const POISON1: u32 = 0xdeaf_bead;
/// `POISON_SIZE`: how many bytes of an object are filled and checked, at most.
const POISON_SIZE: usize = 64;

/// `poison_value(v)`: the pattern of the object at `v`, one of four by the page it is in.
pub fn poison_value(v: usize) -> u32 {
    match (v >> PAGE_SHIFT) & 3 {
        0 => POISON0,
        1 => POISON1,
        2 => (POISON0 & 0xffff_0000) | (!POISON0 & 0x0000_ffff),
        _ => (POISON1 & 0xffff_0000) | (!POISON1 & 0x0000_ffff),
    }
}

/// `poison_mem(v, len)`: fills the start of the free object at `v` with its pattern: `len`
/// bytes or `POISON_SIZE`, whichever is less, in whole 32-bit words.
///
/// # Safety
/// `v` is aligned for a `u32`, and `len` bytes from it are memory the caller owns and nothing
/// else reads or writes meanwhile.
pub unsafe fn poison_mem(v: *mut u8, len: usize) {
    let ip = v.cast::<u32>();
    let poison = poison_value(v as usize);

    let len = len.min(POISON_SIZE) / size_of::<u32>();
    for i in 0..len {
        // SAFETY: word `i` lies inside the `len` bytes the caller owns; `v` is aligned.
        unsafe { ip.add(i).write(poison) };
    }
}

/// `poison_check(v, len, pidx, pval)`: whether the start of the object at `v` no longer
/// holds its pattern: the index of the first changed word and the pattern it should hold.
///
/// # Safety
/// `v` is aligned for a `u32`, and `len` bytes from it are memory the caller owns and nothing
/// else writes meanwhile.
pub unsafe fn poison_check(v: *const u8, len: usize) -> Option<(usize, u32)> {
    let ip = v.cast::<u32>();
    let poison = poison_value(v as usize);

    let len = len.min(POISON_SIZE) / size_of::<u32>();
    for i in 0..len {
        // SAFETY: word `i` lies inside the `len` bytes the caller owns; `v` is aligned.
        if unsafe { ip.add(i).read() } != poison {
            return Some((i, poison));
        }
    }
    None
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::param::PAGE_SIZE;

    #[test]
    fn the_pattern_follows_the_page_of_the_address() {
        assert_eq!(poison_value(0), 0xdead_beef);
        assert_eq!(poison_value(PAGE_SIZE), 0xdeaf_bead);
        assert_eq!(poison_value(2 * PAGE_SIZE), 0xdead_4110);
        assert_eq!(poison_value(3 * PAGE_SIZE), 0xdeaf_4152);
        // The offset inside the page does not count; the fifth page is the first again.
        assert_eq!(poison_value(PAGE_SIZE + 123), 0xdeaf_bead);
        assert_eq!(poison_value(4 * PAGE_SIZE + 8), 0xdead_beef);
    }

    #[test]
    fn poison_mem_fills_whole_words_and_at_most_poison_size() {
        let mut buf = [0u32; 32];
        let poison = poison_value(buf.as_ptr() as usize);

        // Ten bytes are two whole words; the third is not touched.
        // SAFETY: the buffer is ours, aligned and long enough.
        unsafe { poison_mem(buf.as_mut_ptr().cast::<u8>(), 10) };
        assert_eq!(buf[..3], [poison, poison, 0]);

        // A long object is filled as far as POISON_SIZE.
        // SAFETY: as above.
        unsafe { poison_mem(buf.as_mut_ptr().cast::<u8>(), size_of_val(&buf)) };
        assert!(buf[..POISON_SIZE / 4].iter().all(|w| *w == poison));
        assert!(buf[POISON_SIZE / 4..].iter().all(|w| *w == 0));
    }

    #[test]
    fn poison_check_names_the_first_changed_word() {
        /// `poison_check` over the first `len` bytes of `buf`.
        fn check(buf: &[u32; 32], len: usize) -> Option<(usize, u32)> {
            // SAFETY: the buffer is the caller's, aligned and at least `len` bytes long.
            unsafe { poison_check(buf.as_ptr().cast::<u8>(), len) }
        }

        let mut buf = [0u32; 32];
        let poison = poison_value(buf.as_ptr() as usize);
        let len = size_of_val(&buf);
        // SAFETY: the buffer is ours, aligned and long enough.
        unsafe { poison_mem(buf.as_mut_ptr().cast::<u8>(), len) };
        assert_eq!(check(&buf, len), None);

        // A write past POISON_SIZE is not seen.
        buf[POISON_SIZE / 4] = 1;
        assert_eq!(check(&buf, len), None);

        // A write after the free is: the first changed word is the one named.
        buf[9] = 0x1234;
        buf[5] ^= 1;
        assert_eq!(check(&buf, len), Some((5, poison)));
        // A shorter check stops before it.
        assert_eq!(check(&buf, 20), None);
        assert_eq!(check(&buf, 24), Some((5, poison)));
    }
}
/* </TESTS> */
