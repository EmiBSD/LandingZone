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
 * Copyright (c) 2002 Marc Espie.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE OPENBSD PROJECT AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 * A PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE OPENBSD
 * PROJECT OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
 * LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/limits.h>`: common definitions for limits.h.
//!
//! Upstream: sys/sys/limits.h @ 3ce1f3f79392
//!
//! Status: `ported`. Every limit the kernel sees: the `char`, `short`, `int`, `long` and
//! `long long` limits, `MB_LEN_MAX`, `LONG_BIT` and `WORD_BIT`. The header's last part, the
//! `FLT_*` and `DBL_*` legacy names, is under `__XPG_VISIBLE < 600`, which is false in the
//! kernel (and it would only rename the compiler's `__FLT_*`).
//!
//! ## Deviations
//! - `<machine/limits.h>` of amd64 and arm64 both define `SSIZE_MAX` as `LONG_MAX` and
//!   `SIZE_MAX` as `ULONG_MAX`; with every target 64-bit (`__LP64__`) they are given here
//!   instead of through a per-architecture module.
//! - `CHAR_MAX` and `CHAR_MIN` are `i32` (in C they are `int` expressions), since their
//!   values depend on whether the machine's `char` is signed.

/// `CHAR_BIT`: number of bits in a char.
pub const CHAR_BIT: u32 = 8;

/// `SCHAR_MAX`: max value for a signed char.
pub const SCHAR_MAX: i8 = 0x7f;
/// `SCHAR_MIN`: min value for a signed char.
pub const SCHAR_MIN: i8 = -0x7f - 1;
/// `UCHAR_MAX`: max value for an unsigned char.
pub const UCHAR_MAX: u8 = 0xff;
/// `CHAR_MAX`: max value for a char. `char` is unsigned on arm64 (the compiler defines
/// `__CHAR_UNSIGNED__` there) and signed on amd64; the host tests see amd64's.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub const CHAR_MAX: i32 = 0xff;
/// `CHAR_MIN`: min value for a char.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub const CHAR_MIN: i32 = 0;
/// `CHAR_MAX`: max value for a char. `char` is unsigned on arm64 (the compiler defines
/// `__CHAR_UNSIGNED__` there) and signed on amd64; the host tests see amd64's.
#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub const CHAR_MAX: i32 = 0x7f;
/// `CHAR_MIN`: min value for a char.
#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub const CHAR_MIN: i32 = -0x7f - 1;

/// `MB_LEN_MAX`: allow UTF-8 (RFC 3629).
pub const MB_LEN_MAX: usize = 4;

/// `USHRT_MAX`: max value for an unsigned short.
pub const USHRT_MAX: u16 = 0xffff;
/// `SHRT_MAX`: max value for a short.
pub const SHRT_MAX: i16 = 0x7fff;
/// `SHRT_MIN`: min value for a short.
pub const SHRT_MIN: i16 = -0x7fff - 1;

/// `UINT_MAX`: max value for an unsigned int.
pub const UINT_MAX: u32 = 0xffff_ffff;
/// `INT_MAX`: max value for an int.
pub const INT_MAX: i32 = 0x7fff_ffff;
/// `INT_MIN`: min value for an int.
pub const INT_MIN: i32 = -0x7fff_ffff - 1;

/// `ULONG_MAX`: max value for unsigned long (`__LP64__`).
pub const ULONG_MAX: u64 = 0xffff_ffff_ffff_ffff;
/// `LONG_MAX`: max value for a signed long.
pub const LONG_MAX: i64 = 0x7fff_ffff_ffff_ffff;
/// `LONG_MIN`: min value for a signed long.
pub const LONG_MIN: i64 = -0x7fff_ffff_ffff_ffff - 1;

/// `ULLONG_MAX`: max value for unsigned long long.
pub const ULLONG_MAX: u64 = 0xffff_ffff_ffff_ffff;
/// `LLONG_MAX`: max value for a signed long long.
pub const LLONG_MAX: i64 = 0x7fff_ffff_ffff_ffff;
/// `LLONG_MIN`: min value for a signed long long.
pub const LLONG_MIN: i64 = -0x7fff_ffff_ffff_ffff - 1;

/// `LONG_BIT`.
pub const LONG_BIT: u32 = 64;
/// `WORD_BIT`.
pub const WORD_BIT: u32 = 32;

/// `SSIZE_MAX` (`<machine/limits.h>`): max value for a ssize_t.
pub const SSIZE_MAX: isize = LONG_MAX as isize;
/// `SIZE_MAX` (`<machine/limits.h>`): max value for a size_t.
pub const SIZE_MAX: usize = ULONG_MAX as usize;

const _: () = {
    assert!(size_of::<usize>() == 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limits_are_the_types_own() {
        assert_eq!(SCHAR_MAX, i8::MAX);
        assert_eq!(SCHAR_MIN, i8::MIN);
        assert_eq!(UCHAR_MAX, u8::MAX);
        assert_eq!(USHRT_MAX, u16::MAX);
        assert_eq!((SHRT_MIN, SHRT_MAX), (i16::MIN, i16::MAX));
        assert_eq!(UINT_MAX, u32::MAX);
        assert_eq!((INT_MIN, INT_MAX), (i32::MIN, i32::MAX));
        assert_eq!(ULONG_MAX, u64::MAX);
        assert_eq!((LONG_MIN, LONG_MAX), (i64::MIN, i64::MAX));
        assert_eq!(ULLONG_MAX, u64::MAX);
        assert_eq!((LLONG_MIN, LLONG_MAX), (i64::MIN, i64::MAX));
        assert_eq!(SSIZE_MAX, isize::MAX);
        assert_eq!(SIZE_MAX, usize::MAX);
        // The host tests see a signed char, as amd64.
        assert_eq!((CHAR_MIN, CHAR_MAX), (-128, 127));
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn the_plain_defines_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/limits.h");
        let ours: &[(&str, i64)] = &[
            ("CHAR_BIT", CHAR_BIT as i64),
            ("SCHAR_MAX", SCHAR_MAX as i64),
            ("UCHAR_MAX", UCHAR_MAX as i64),
            ("MB_LEN_MAX", MB_LEN_MAX as i64),
            ("USHRT_MAX", USHRT_MAX as i64),
            ("SHRT_MAX", SHRT_MAX as i64),
            ("INT_MAX", INT_MAX as i64),
            ("WORD_BIT", WORD_BIT as i64),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
