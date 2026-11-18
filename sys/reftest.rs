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
//! Helpers for reference-backed tests (`just test-ref`).
//!
//! They read a header from the C tree at `$OPENBSD_SRC` and collect its `#define NAME VALUE`
//! lines. Values are evaluated only as far as integer literals, the names of other defines, and
//! the binary operators `|`, `<<`, `+`, `-` and `*` between such operands (with C's precedence
//! and associativity); anything richer is parsing C, which is not the point. A test that needs
//! more compares the raw text.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::string::{String, ToString};

/// `$OPENBSD_SRC`, set by `just test-ref`. A relative path is taken from the workspace root,
/// not from the package directory `cargo test` runs in.
pub(crate) fn openbsd_src() -> PathBuf {
    let dir = match std::env::var_os("OPENBSD_SRC") {
        Some(dir) => PathBuf::from(dir),
        None => panic!("OPENBSD_SRC is not set; run `just test-ref`"),
    };
    if dir.is_absolute() {
        dir
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(dir)
    }
}

/// The `#define NAME VALUE` lines of a header, as `NAME -> VALUE` text with trailing comments
/// stripped. Function-like macros (`#define F(x) ...`) are left out.
pub(crate) fn defines(rel: &str) -> BTreeMap<String, String> {
    let path = openbsd_src().join(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    parse_defines(&text)
}

/// The defines of a header's `text` ([`defines`]). A line that ends in a backslash goes on
/// in the next one, as the preprocessor reads it.
pub(crate) fn parse_defines(text: &str) -> BTreeMap<String, String> {
    let text = text.replace("\\\n", " ");
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix('#') else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix("define") else {
            continue;
        };
        let rest = strip_comment(rest).trim();
        let mut parts = rest.splitn(2, char::is_whitespace);
        let Some(name) = parts.next() else {
            continue;
        };
        if name.is_empty() || name.contains('(') {
            continue;
        }
        let value = parts.next().unwrap_or("").trim().to_string();
        out.insert(name.to_string(), value);
    }
    out
}

/// The integer value of `name`, following aliases and evaluating the operators above; `None`
/// when the define is absent or too complex.
pub(crate) fn int(defs: &BTreeMap<String, String>, name: &str) -> Option<i64> {
    eval(defs, defs.get(name)?, 0)
}

fn strip_comment(s: &str) -> &str {
    match s.find("/*") {
        Some(i) => &s[..i],
        None => s,
    }
}

fn eval(defs: &BTreeMap<String, String>, expr: &str, depth: u32) -> Option<i64> {
    if depth > 32 {
        return None;
    }
    let e = strip_parens(expr.trim());
    if let Some(n) = parse_int(e) {
        return Some(n);
    }
    if is_ident(e) {
        return eval(defs, defs.get(e)?, depth + 1);
    }
    // Lowest precedence first; `+` and `-` split at their last occurrence, so that a chain
    // such as `A - B - C` is `(A - B) - C`.
    for op in ["|", "<<"] {
        if let Some((l, r)) = split_top_level(e, op) {
            let (l, r) = (eval(defs, l, depth + 1)?, eval(defs, r, depth + 1)?);
            return Some(if op == "|" { l | r } else { l << r });
        }
    }
    if let Some((l, op, r)) = split_last_additive(e) {
        let (l, r) = (eval(defs, l, depth + 1)?, eval(defs, r, depth + 1)?);
        return Some(if op == '+' { l + r } else { l - r });
    }
    if let Some((l, r)) = split_top_level(e, "*") {
        return Some(eval(defs, l, depth + 1)? * eval(defs, r, depth + 1)?);
    }
    None
}

/// Strips one or more pairs of parentheses enclosing the whole expression.
fn strip_parens(mut e: &str) -> &str {
    while let Some(inner) = e.strip_prefix('(').and_then(|i| i.strip_suffix(')')) {
        if !balanced(inner) {
            break;
        }
        e = inner.trim();
    }
    e
}

fn balanced(e: &str) -> bool {
    let mut depth = 0i32;
    for c in e.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// Splits `e` at the first occurrence of `op` outside parentheses, never at position 0 (so a
/// leading unary minus is not an operator).
fn split_top_level<'a>(e: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0i32;
    for (i, c) in e.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 && i > 0 && e[i..].starts_with(op) => {
                return Some((&e[..i], &e[i + op.len()..]));
            }
            _ => {}
        }
    }
    None
}

/// Splits `e` at the last `+` or `-` outside parentheses that is a binary operator (it follows
/// an operand, so a unary minus is never taken).
fn split_last_additive(e: &str) -> Option<(&str, char, &str)> {
    let mut depth = 0i32;
    let mut found = None;
    let mut prev_operand = false;
    for (i, c) in e.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '+' | '-' if depth == 0 && prev_operand => found = Some((i, c)),
            _ => {}
        }
        if !c.is_whitespace() {
            prev_operand = c == ')' || c == '_' || c.is_ascii_alphanumeric();
        }
    }
    let (i, op) = found?;
    Some((&e[..i], op, &e[i + 1..]))
}

fn is_ident(e: &str) -> bool {
    let mut chars = e.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// A C integer literal: decimal, hex or octal, optional leading `-`, optional `U`/`L` suffixes.
/// Hex literals wider than `i64` wrap, so `0xffffffff80000000` compares equal to the same bits
/// read as a signed value.
pub(crate) fn parse_int(text: &str) -> Option<i64> {
    let t = text.trim();
    let (neg, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.trim()),
        None => (false, t),
    };
    let t = t.trim_end_matches(['u', 'U', 'l', 'L']);
    let n = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok()? as i64
    } else if t.len() > 1 && t.starts_with('0') && t.bytes().all(|b| b.is_ascii_digit()) {
        i64::from_str_radix(&t[1..], 8).ok()?
    } else if !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()) {
        t.parse::<i64>().ok()?
    } else {
        return None;
    };
    Some(if neg { -n } else { n })
}

/// Asserts that each named Rust constant equals the C `#define` of the same name in `defs`, and
/// returns the names it checked (for [`assert_complete`]). The constants must convert to `i64`
/// with `as`.
macro_rules! assert_defines {
    ($defs:expr; $($name:ident),* $(,)?) => {{
        let defs = &$defs;
        let mut names: std::vec::Vec<&'static str> = std::vec::Vec::new();
        $(
            names.push(stringify!($name));
            assert_eq!(
                $crate::reftest::int(defs, stringify!($name)),
                Some($name as i64),
                "{}",
                stringify!($name)
            );
        )*
        names
    }};
}
pub(crate) use assert_defines;

/// Asserts that every C define whose name starts with `prefix` is among `ours`: a constant
/// added upstream fails the test instead of going unnoticed.
pub(crate) fn assert_complete(defs: &BTreeMap<String, String>, prefix: &str, ours: &[&str]) {
    for name in defs.keys().filter(|n| n.starts_with(prefix)) {
        assert!(ours.contains(&name.as_str()), "{name} is not ported");
    }
}

/// Asserts that `ours` is the C's `#define NAME { { "name", CTLTYPE_T }, ... }` of the header
/// `rel`: the same entries in the same order, `{ 0, 0 }` for an unused slot.
pub(crate) fn assert_ctlnames(rel: &str, name: &str, ours: &[crate::sys::sysctl::Ctlname]) {
    let path = openbsd_src().join(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let types = defines("sys/sys/sysctl.h");
    let head = std::format!("{name} {{");
    let at = text
        .lines()
        .position(|l| {
            l.split_whitespace().collect::<std::vec::Vec<_>>().join(" ")
                == std::format!("#define {head} \\")
        })
        .unwrap_or_else(|| panic!("{rel}: no {name}"));
    let mut n = 0;
    for line in text.lines().skip(at + 1) {
        if line.starts_with('}') {
            break;
        }
        // A comment may hold an entry that was taken out (`/* { "mtu", ... }, */ { 0, 0 }`).
        let mut line = line.to_string();
        while let (Some(open), Some(close)) = (line.find("/*"), line.find("*/")) {
            line.replace_range(open..close + 2, "");
        }
        let Some(entry) = line.split('{').nth(1).and_then(|e| e.split('}').next()) else {
            continue;
        };
        let (cname, ctype) = entry
            .split_once(',')
            .unwrap_or_else(|| panic!("{name}: {line}"));
        let (cname, ctype) = (cname.trim(), ctype.trim());
        let want_name =
            (cname != "0" && cname != "NULL").then(|| cname.trim_matches('"').as_bytes());
        let want_type = if ctype == "0" {
            Some(0)
        } else {
            int(&types, ctype)
        };
        let got = ours
            .get(n)
            .unwrap_or_else(|| panic!("{name}: entry {n} is missing"));
        assert_eq!(got.ctl_name, want_name, "{name}[{n}]");
        assert_eq!(Some(i64::from(got.ctl_type)), want_type, "{name}[{n}]");
        n += 1;
    }
    assert_eq!(n, ours.len(), "{name}");
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn defs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn a_define_may_run_over_lines() {
        let d = parse_defines(
            "#define A 0x1 /* one */\n#define\tB\t(A | \\\n\t\t 0x2 | \\\n\t\t 0x4)\n\
             #define F(x) \\\n\t((x) + 1)\n#define C B\n",
        );
        assert_eq!(d.len(), 3);
        assert_eq!(int(&d, "A"), Some(1));
        assert_eq!(int(&d, "B"), Some(7));
        assert_eq!(int(&d, "C"), Some(7));
    }

    #[test]
    fn a_long_chain_of_names_is_evaluated() {
        let mut pairs = std::vec::Vec::new();
        let mut all = String::new();
        for i in 0..12 {
            pairs.push((std::format!("BIT{i}"), std::format!("(1 << {i})")));
            all += &std::format!("{}BIT{i}", if i == 0 { "(" } else { " | " });
        }
        pairs.push(("ALL".to_string(), all + ")"));
        let d: BTreeMap<String, String> = pairs.into_iter().collect();
        assert_eq!(int(&d, "ALL"), Some(0xfff));
    }

    #[test]
    fn literals() {
        assert_eq!(parse_int("42"), Some(42));
        assert_eq!(parse_int("-1"), Some(-1));
        assert_eq!(parse_int("0x0ff"), Some(255));
        assert_eq!(parse_int("0xffffffffU"), Some(0xffff_ffff));
        assert_eq!(parse_int("0x7fffffffffffffffL"), Some(i64::MAX));
        assert_eq!(
            parse_int("0xffffffff80000000"),
            Some(0xffff_ffff_8000_0000_u64 as i64)
        );
        assert_eq!(parse_int("0777"), Some(0o777));
        assert_eq!(parse_int("EAGAIN"), None);
        assert_eq!(parse_int(""), None);
    }

    #[test]
    fn aliases_and_expressions() {
        let d = defs(&[
            ("EAGAIN", "35"),
            ("EWOULDBLOCK", "EAGAIN"),
            ("PAGE_SHIFT", "12"),
            ("PAGE_SIZE", "(1 << PAGE_SHIFT)"),
            ("PAGE_MASK", "(PAGE_SIZE - 1)"),
            ("MSGBUFSIZE", "(32 * PAGE_SIZE)"),
            ("MAXCOMLEN", "_MAXCOMLEN-1"),
            ("_MAXCOMLEN", "24"),
            ("ARG_MAX", "(512 * 1024)"),
            ("ERESTART", "-1"),
            ("LOOP", "LOOP"),
            ("HARD", "((x) + 1) / 2"),
        ]);
        assert_eq!(int(&d, "EWOULDBLOCK"), Some(35));
        assert_eq!(int(&d, "PAGE_SIZE"), Some(4096));
        assert_eq!(int(&d, "PAGE_MASK"), Some(4095));
        assert_eq!(int(&d, "MSGBUFSIZE"), Some(32 * 4096));
        assert_eq!(int(&d, "MAXCOMLEN"), Some(23));
        assert_eq!(int(&d, "ARG_MAX"), Some(512 * 1024));
        assert_eq!(int(&d, "ERESTART"), Some(-1));
        assert_eq!(int(&d, "LOOP"), None);
        assert_eq!(int(&d, "HARD"), None);
        let d = defs(&[
            ("A", "1518"),
            ("B", "((6 * 2) + 2)"),
            ("C", "4"),
            ("MTU", "(A - B - C)"),
            ("IN", "0x80000000UL"),
            ("OUT", "0x40000000UL"),
            ("INOUT", "(IN|OUT)"),
            ("NEG", "(A - -1)"),
        ]);
        assert_eq!(int(&d, "B"), Some(14));
        assert_eq!(int(&d, "MTU"), Some(1500));
        assert_eq!(int(&d, "INOUT"), Some(0xc000_0000));
        assert_eq!(int(&d, "NEG"), Some(1519));
        assert_eq!(int(&d, "MISSING"), None);
    }
}
/* </TESTS> */
