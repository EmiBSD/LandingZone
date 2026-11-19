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

/* crc32.c -- compute the CRC-32 of a data stream
 * Copyright (C) 1995-2026 Mark Adler
 * For conditions of distribution and use, see copyright notice in zlib.h
 *
 * This interleaved implementation of a CRC makes use of pipelined multiple
 * arithmetic-logic units, commonly found in modern CPU cores. It is due to
 * Kadatch and Jenkins (2010). See doc/crc-doc.1.0.pdf in this distribution.
 */

/* zlib.h -- interface of the 'zlib' general purpose compression library
  version 1.3.2, February 17th, 2026

  Copyright (C) 1995-2026 Jean-loup Gailly and Mark Adler

  This software is provided 'as-is', without any express or implied
  warranty.  In no event will the authors be held liable for any damages
  arising from the use of this software.

  Permission is granted to anyone to use this software for any purpose,
  including commercial applications, and to alter it and redistribute it
  freely, subject to the following restrictions:

  1. The origin of this software must not be misrepresented; you must not
     claim that you wrote the original software. If you use this software
     in a product, an acknowledgment in the product documentation would be
     appreciated but is not required.
  2. Altered source versions must be plainly marked as such, and must not be
     misrepresented as being the original software.
  3. This notice may not be removed or altered from any source distribution.

  Jean-loup Gailly        Mark Adler
  jloup@gzip.org          madler@alumni.caltech.edu


  The data format used by the zlib library is described by RFCs (Request for
  Comments) 1950 to 1952 at https://datatracker.ietf.org/doc/html/rfc1950
  (zlib format), rfc1951 (deflate format) and rfc1952 (gzip format).
*/
/* </LICENSES> */

/* <CODE> */
//! The plain CRC-32 (polynomial `0xedb88320`, reflected), as zlib's `crc32()` computes it.
//!
//! Upstream: sys/lib/libz/crc32.c @ 3ce1f3f79392
//!
//! **This is an altered source version, not the original zlib `crc32.c`** (zlib licence, clause
//! 2): it is a Rust rewrite of the file's observable behaviour, written for EmiBSD. The
//! original's notice is kept above in full (clause 3). Do not mistake it for the zlib
//! distribution; bugs here are not Mark Adler's.
//!
//! `crc32(crc, buf)` starts from `0` and may be chained: feeding consecutive chunks through
//! successive calls yields the CRC of their concatenation. The kernel's one user is
//! `subr_disk.c` (`gpt_get_hdr` and `gpt_get_parts`, which checksum the GPT header and the
//! partition entries). The kernel's own CRC-32C is `lib/libkern/crc32c.h`, a different polynomial.
//!
//! ## Deviations
//! - The configuration is the kernel's on amd64 and arm64: `W` is 8 (`z_word_t` is `u64`),
//!   `N` is 5, and `ARMCRC32` is not defined (the arm64 kernel is not built for the CRC
//!   instructions), so `crc32_z` is the braided calculation. The ARM `crc32_z`,
//!   `DYNAMIC_CRC_TABLE` with `make_crc_table`, `braid` and `z_once`, `MAKECRCH` with its
//!   `write_table*` and `main`, and `HAVE_S390X_VX` are not configured (approved exceptions
//!   of 2026-11-19).
//! - `crc32.h`'s tables (`crc_table`, `crc_big_table`, `crc_braid_table`,
//!   `crc_braid_big_table` and `x2n_table`) are computed at compile time by `const fn`s
//!   that do what `make_crc_table` and `braid` do when `crc32.h` is generated, so the
//!   header is not copied.
//! - `buf` is a slice, so `crc32_z(crc, Z_NULL, 0)` returning the initial value `0` has no
//!   counterpart: an empty slice leaves the CRC unchanged (`crc32(0, &[]) == 0`), and
//!   `crc32` and `crc32_z` take the same arguments (`uInt` against `z_size_t` only matters
//!   past 4 GiB). `crc` and the results are `u32`: they never exceed 32 bits.
//! - The braids read their words with `u64::from_ne_bytes` from the aligned part of the
//!   slice, where the C casts the pointer to `z_word_t const *`; the endianness test is
//!   `cfg!(target_endian)`, and both branches are compiled, as the C's run-time test.
//! - The `z_off_t` and `z_off64_t` lengths are both `i64` (LP64).

/// `POLY`: the CRC-32 polynomial, reflected, with `x^32` implied.
const POLY: u32 = 0xedb8_8320;

/// `N`: the number of braids.
const N: usize = 5;

/// `W`: the bytes of a `z_word_t`.
const W: usize = 8;

/// `crc_table`: the CRC of each byte value (`crc32.h`).
const CRC_TABLE: [u32; 256] = crc_table_const();

/// `crc_big_table`: [`CRC_TABLE`] byte-swapped into a `z_word_t`, for the big-endian braids.
const CRC_BIG_TABLE: [u64; 256] = crc_big_table_const();

/// `x2n_table`: `x^2^n` modulo p(x), for `n` in 0..32, for combining CRC-32s.
const X2N_TABLE: [u32; 32] = x2n_table_const();

/// `crc_braid_table`: the little-endian braid tables for `N` and `W`.
const CRC_BRAID_TABLE: [[u32; 256]; W] = braid_tables_const().0;

/// `crc_braid_big_table`: the big-endian braid tables for `N` and `W`.
const CRC_BRAID_BIG_TABLE: [[u64; 256]; W] = braid_tables_const().1;

/// What `make_crc_table` puts in `crc_table`: the CRC of each byte value.
const fn crc_table_const() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut p = i as u32;
        let mut j = 0;
        while j < 8 {
            p = if p & 1 != 0 { (p >> 1) ^ POLY } else { p >> 1 };
            j += 1;
        }
        table[i] = p;
        i += 1;
    }
    table
}

/// What `make_crc_table` puts in `crc_big_table`.
const fn crc_big_table_const() -> [u64; 256] {
    let mut table = [0u64; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = byte_swap(CRC_TABLE[i] as u64);
        i += 1;
    }
    table
}

/// What `make_crc_table` puts in `x2n_table`.
const fn x2n_table_const() -> [u32; 32] {
    let mut table = [0u32; 32];
    let mut p: u32 = 1 << 30; // x^1
    table[0] = p;
    let mut n = 1;
    while n < 32 {
        p = multmodp(p, p);
        table[n] = p;
        n += 1;
    }
    table
}

/// What `braid(crc_braid_table, crc_braid_big_table, N, W)` computes.
const fn braid_tables_const() -> ([[u32; 256]; W], [[u64; 256]; W]) {
    let mut ltl = [[0u32; 256]; W];
    let mut big = [[0u64; 256]; W];
    let mut k = 0;
    while k < W {
        let p = x2nmodp_table(((N * W + 3 - k) << 3) as i64, 0, &x2n_table_const());
        let mut i = 1;
        while i < 256 {
            let q = multmodp((i as u32) << 24, p);
            ltl[k][i] = q;
            big[W - 1 - k][i] = byte_swap(q as u64);
            i += 1;
        }
        k += 1;
    }
    (ltl, big)
}

/// `byte_swap`: the bytes of a `z_word_t` in the other order, between little and big endian.
const fn byte_swap(word: u64) -> u64 {
    (word & 0xff00_0000_0000_0000) >> 56
        | (word & 0x00ff_0000_0000_0000) >> 40
        | (word & 0x0000_ff00_0000_0000) >> 24
        | (word & 0x0000_00ff_0000_0000) >> 8
        | (word & 0x0000_0000_ff00_0000) << 8
        | (word & 0x0000_0000_00ff_0000) << 24
        | (word & 0x0000_0000_0000_ff00) << 40
        | (word & 0x0000_0000_0000_00ff) << 56
}

/// `multmodp`: a(x) multiplied by b(x) modulo p(x), the CRC polynomial, reflected. `a` must
/// not be zero.
const fn multmodp(a: u32, b: u32) -> u32 {
    let mut b = b;
    let mut m: u32 = 1 << 31;
    let mut p: u32 = 0;
    loop {
        if a & m != 0 {
            p ^= b;
            if a & (m - 1) == 0 {
                break;
            }
        }
        m >>= 1;
        b = if b & 1 != 0 { (b >> 1) ^ POLY } else { b >> 1 };
    }
    p
}

/// `x2nmodp` over a given `x2n_table`: the body, usable while the table is being built.
const fn x2nmodp_table(n: i64, k: u32, x2n: &[u32; 32]) -> u32 {
    let mut n = n;
    let mut k = k;
    let mut p: u32 = 1 << 31; // x^0 == 1
    while n != 0 {
        if n & 1 != 0 {
            p = multmodp(x2n[(k & 31) as usize], p);
        }
        n >>= 1;
        k += 1;
    }
    p
}

/// `x2nmodp`: x^(n * 2^k) modulo p(x). `n` must not be negative.
fn x2nmodp(n: i64, k: u32) -> u32 {
    x2nmodp_table(n, k, &X2N_TABLE)
}

/// `get_crc_table`: the byte-wise CRC table, for asm versions of `crc32()`.
pub fn get_crc_table() -> &'static [u32; 256] {
    &CRC_TABLE
}

/// `crc_word`: the CRC of the `W` bytes of `data`, its least significant byte first,
/// without pre or post conditioning; it combines the CRCs of the braids.
fn crc_word(data: u64) -> u32 {
    let mut data = data;
    for _ in 0..W {
        data = (data >> 8) ^ u64::from(CRC_TABLE[(data & 0xff) as usize]);
    }
    data as u32
}

/// `crc_word_big`: [`crc_word`] for big-endian words.
fn crc_word_big(data: u64) -> u64 {
    let mut data = data;
    for _ in 0..W {
        data = (data << 8) ^ CRC_BIG_TABLE[((data >> ((W - 1) << 3)) & 0xff) as usize];
    }
    data
}

/// `crc32_z`: the CRC-32 of `buf`, continuing from `crc` (pass `0` to start). The running
/// value is inverted on entry and on exit, so feeding consecutive chunks through successive
/// calls yields the CRC of their concatenation.
pub fn crc32_z(crc: u32, buf: &[u8]) -> u32 {
    let mut buf = buf;

    // Pre-condition the CRC
    let mut crc = !crc;

    // If provided enough bytes, do a braided CRC calculation.
    if buf.len() >= N * W + W - 1 {
        // Compute the CRC up to a z_word_t boundary.
        let head = buf.as_ptr().align_offset(W).min(buf.len());
        for &b in &buf[..head] {
            crc = (crc >> 8) ^ CRC_TABLE[((crc ^ u32::from(b)) & 0xff) as usize];
        }
        buf = &buf[head..];

        // Compute the CRC on as many N z_word_t blocks as are available.
        let mut blks = buf.len() / (N * W);
        let word = |w: &[u8], i: usize| {
            let mut b = [0u8; W];
            b.copy_from_slice(&w[i * W..(i + 1) * W]);
            u64::from_ne_bytes(b)
        };
        let mut words = buf;

        if cfg!(target_endian = "little") {
            // Little endian.
            let mut crcs = [0u32; N];
            // Initialize the CRC for each braid.
            crcs[0] = crc;

            // Process the first blks-1 blocks, computing the CRCs on each braid independently.
            loop {
                blks -= 1;
                if blks == 0 {
                    break;
                }
                // Load the word for each braid into registers.
                let mut ws = [0u64; N];
                for (b, w) in ws.iter_mut().enumerate() {
                    *w = u64::from(crcs[b]) ^ word(words, b);
                }
                words = &words[N * W..];

                // Compute and update the CRC for each word.
                for (b, &w) in ws.iter().enumerate() {
                    crcs[b] = CRC_BRAID_TABLE[0][(w & 0xff) as usize];
                }
                for (k, table) in CRC_BRAID_TABLE.iter().enumerate().skip(1) {
                    for (b, &w) in ws.iter().enumerate() {
                        crcs[b] ^= table[((w >> (k << 3)) & 0xff) as usize];
                    }
                }
            }

            // Process the last block, combining the CRCs of the N braids at the same time.
            crc = crc_word(u64::from(crcs[0]) ^ word(words, 0));
            for (b, &c) in crcs.iter().enumerate().skip(1) {
                crc = crc_word(u64::from(c) ^ word(words, b) ^ u64::from(crc));
            }
            words = &words[N * W..];
        } else {
            // Big endian.
            let mut crcs = [0u64; N];
            // Initialize the CRC for each braid.
            crcs[0] = byte_swap(u64::from(crc));

            // Process the first blks-1 blocks, computing the CRCs on each braid independently.
            loop {
                blks -= 1;
                if blks == 0 {
                    break;
                }
                let mut ws = [0u64; N];
                for (b, w) in ws.iter_mut().enumerate() {
                    *w = crcs[b] ^ word(words, b);
                }
                words = &words[N * W..];

                for (b, &w) in ws.iter().enumerate() {
                    crcs[b] = CRC_BRAID_BIG_TABLE[0][(w & 0xff) as usize];
                }
                for (k, table) in CRC_BRAID_BIG_TABLE.iter().enumerate().skip(1) {
                    for (b, &w) in ws.iter().enumerate() {
                        crcs[b] ^= table[((w >> (k << 3)) & 0xff) as usize];
                    }
                }
            }

            // Process the last block, combining the CRCs of the N braids at the same time.
            let mut comb = crc_word_big(crcs[0] ^ word(words, 0));
            for (b, &c) in crcs.iter().enumerate().skip(1) {
                comb = crc_word_big(c ^ word(words, b) ^ comb);
            }
            words = &words[N * W..];
            crc = byte_swap(comb) as u32;
        }

        // Update the pointer to the remaining bytes to process.
        buf = words;
    }

    // Complete the computation of the CRC on any remaining bytes (eight at a time, then one).
    for &b in buf {
        crc = (crc >> 8) ^ CRC_TABLE[((crc ^ u32::from(b)) & 0xff) as usize];
    }

    // Return the CRC, post-conditioned.
    !crc
}

/// `crc32`: [`crc32_z`].
pub fn crc32(crc: u32, buf: &[u8]) -> u32 {
    crc32_z(crc, buf)
}

/// `crc32_combine_gen64`: the operator that [`crc32_combine_op`] applies to combine a CRC
/// with that of a following `len2` bytes; 0 for a negative `len2`.
pub fn crc32_combine_gen64(len2: i64) -> u32 {
    if len2 < 0 {
        return 0;
    }
    x2nmodp(len2, 3)
}

/// `crc32_combine_gen`: [`crc32_combine_gen64`] over a `z_off_t` length.
pub fn crc32_combine_gen(len2: i64) -> u32 {
    crc32_combine_gen64(len2)
}

/// `crc32_combine_op`: the CRC of the concatenation, from the CRC of each part and the
/// operator for the second part's length.
pub fn crc32_combine_op(crc1: u32, crc2: u32, op: u32) -> u32 {
    if op == 0 {
        return 0;
    }
    multmodp(op, crc1) ^ crc2
}

/// `crc32_combine64`: the CRC of the concatenation of two buffers, from their CRCs and the
/// second's length.
pub fn crc32_combine64(crc1: u32, crc2: u32, len2: i64) -> u32 {
    crc32_combine_op(crc1, crc2, crc32_combine_gen64(len2))
}

/// `crc32_combine`: [`crc32_combine64`] over a `z_off_t` length.
pub fn crc32_combine(crc1: u32, crc2: u32, len2: i64) -> u32 {
    crc32_combine64(crc1, crc2, len2)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        let cases: &[(&[u8], u32)] = &[
            (b"", 0),
            (b"a", 0xe8b7_be43),
            (b"abc", 0x3524_41c2),
            (b"123456789", 0xcbf4_3926),
            (b"The quick brown fox jumps over the lazy dog", 0x414f_a339),
        ];
        for &(input, want) in cases {
            assert_eq!(crc32(0, input), want, "{:?}", core::str::from_utf8(input));
        }
    }

    #[test]
    fn table_spot_checks() {
        assert_eq!(CRC_TABLE[0], 0x0000_0000);
        assert_eq!(CRC_TABLE[1], 0x7707_3096);
        assert_eq!(CRC_TABLE[2], 0xee0e_612c);
        assert_eq!(CRC_TABLE[128], 0xedb8_8320);
        assert_eq!(CRC_TABLE[255], 0x2d02_ef8d);
    }

    #[test]
    fn chaining_chunks_equals_one_call() {
        let whole = b"The quick brown fox jumps over the lazy dog";
        for split in 0..=whole.len() {
            let (a, b) = whole.split_at(split);
            assert_eq!(crc32(crc32(0, a), b), crc32(0, whole), "split at {split}");
        }
    }

    /// The byte-at-a-time CRC, the reference the braids must agree with.
    fn bytewise(crc: u32, buf: &[u8]) -> u32 {
        let mut crc = !crc;
        for &b in buf {
            crc = (crc >> 8) ^ CRC_TABLE[((crc ^ u32::from(b)) & 0xff) as usize];
        }
        !crc
    }

    #[test]
    fn braided_crc_agrees_with_the_bytewise_one() {
        let mut data = [0u8; 4096];
        let mut x: u32 = 0x1234_5678;
        for b in data.iter_mut() {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
            *b = (x >> 16) as u8;
        }
        // Every start offset (alignment) and lengths around the braided threshold.
        for start in 0..W {
            for len in [0, 1, 46, 47, 48, 79, 80, 81, 200, 1000, 4000] {
                let buf = &data[start..start + len];
                assert_eq!(crc32_z(0, buf), bytewise(0, buf), "start {start} len {len}");
                assert_eq!(crc32_z(0xdead_beef, buf), bytewise(0xdead_beef, buf));
            }
        }
    }

    #[test]
    fn combine_equals_the_crc_of_the_concatenation() {
        let whole = b"The quick brown fox jumps over the lazy dog, twice: the quick brown fox";
        for split in 0..=whole.len() {
            let (a, b) = whole.split_at(split);
            let got = crc32_combine(crc32(0, a), crc32(0, b), b.len() as i64);
            assert_eq!(got, crc32(0, whole), "split at {split}");
            assert_eq!(
                crc32_combine64(crc32(0, a), crc32(0, b), b.len() as i64),
                got
            );
            let op = crc32_combine_gen(b.len() as i64);
            assert_eq!(crc32_combine_op(crc32(0, a), crc32(0, b), op), got);
        }
        assert_eq!(crc32_combine_gen64(-1), 0);
        assert_eq!(crc32_combine_op(1, 2, 0), 0);
    }

    #[test]
    fn the_tables_are_crc32_h_s() {
        // Spot values of crc32.h at the pin (N = 5, W = 8).
        assert_eq!(get_crc_table()[255], 0x2d02_ef8d);
        assert_eq!(CRC_BIG_TABLE[1], 0x9630_0777_0000_0000);
        assert_eq!(X2N_TABLE[0], 0x4000_0000);
        assert_eq!(byte_swap(0x0102_0304_0506_0708), 0x0807_0605_0403_0201);
    }

    #[test]
    fn empty_buffer_leaves_the_crc_alone() {
        assert_eq!(crc32(0xdead_beef, b""), 0xdead_beef);
    }
}
/* </TESTS> */
