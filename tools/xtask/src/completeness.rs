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
//! The completeness checks of `ports check` (the user's standing order of 2026-11-18,
//! `.claude/rules/ported-definition.md`): a row is `ported` only if
//!
//! 1. its Rust file holds no `unported!` and no `db_stub!` in its code zone, and
//! 2. for a `.c` row, every function the C file defines has a `fn` (or a `macro_rules!`) of
//!    that name in the row's Rust file, or an exception in `ports-exceptions.toml` that the
//!    user approved (`approved` is the date of the approval).
//!
//! Neither check sees a check or a branch that is missing inside a function that exists.
//!
//! `cargo xtask ports missing [--proposal FILE]` lists every C function of a `ported` or
//! `wip` `.c` row that has no `fn` in the row's Rust file, with the preprocessor conditions
//! around its definition and what they come to on amd64 and arm64 under OpenBSD's
//! `GENERIC.MP` (its `option`s, its devices and pseudo-devices as `N<NAME>` counts, the
//! machine headers' `__HAVE_*`/`__USE_*`, the compiler's and the architecture's symbols).
//! With `--proposal` it also writes the candidate exceptions, unapproved, for the user to
//! read. The tool never approves anything.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::{Entry, REFERENCE_DIR, Result, Status};

/// The exceptions the user approved, at the repository's root.
pub const EXCEPTIONS_FILE: &str = "ports-exceptions.toml";

#[derive(Deserialize, Default)]
struct Exceptions {
    #[serde(default, rename = "exception")]
    list: Vec<Exception>,
}

/// One C function that may stay out of its row's Rust file.
#[derive(Deserialize)]
struct Exception {
    c: String,
    function: String,
    reason: String,
    /// The date the user approved it (`YYYY-MM-DD`).
    approved: String,
}

/// The C source with its comments and the contents of its string and character literals
/// blanked (newlines kept), so that a scan by lines sees only code and directives.
pub fn blank_c(src: &[u8]) -> String {
    blank(src, true)
}

/// The Rust source with its comments and the contents of its string literals blanked
/// (newlines kept). Character literals are left alone: a lifetime looks like one.
/// A Rust file with its comments (block comments nest), string literals (raw ones too,
/// `r"..."`, `r#"..."#`, `br"..."`) and char and byte literals (`'x'`, `b'"'`, `'\''`,
/// `'\u{e9}'`) blanked, keeping the line breaks. A `'` that opens no char literal is a
/// lifetime or a label and stays. Before 2026-11-19 the Rust files went through the C
/// blanker, which knows no char literals: a `b'"'` opened a string that ran to the next
/// `"`, and the `fn`s and `unported!` sites between were not seen.
fn blank_rust(b: &[u8]) -> String {
    let blank_byte = |c: u8| if c == b'\n' { b'\n' } else { b' ' };
    let ident_at = |k: usize| b.get(k).is_some_and(|&c| is_ident(c));
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied();
        if c == b'/' && next == Some(b'*') {
            let mut depth = 0usize;
            while i < b.len() {
                if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                    depth += 1;
                    out.extend_from_slice(b"  ");
                    i += 2;
                } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                    depth -= 1;
                    out.extend_from_slice(b"  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank_byte(b[i]));
                    i += 1;
                }
            }
        } else if c == b'/' && next == Some(b'/') {
            while i < b.len() && b[i] != b'\n' {
                out.push(b' ');
                i += 1;
            }
        } else if let Some((hashes, open)) = raw_string_at(b, i) {
            // r"..." / r#"..."# / br"...": no escapes; ends at `"` and as many `#`.
            out.extend_from_slice(&b[i..open + 1]);
            i = open + 1;
            loop {
                if i >= b.len() {
                    break;
                }
                if b[i] == b'"'
                    && b.get(i + 1..i + 1 + hashes)
                        .is_some_and(|h| h.iter().all(|&x| x == b'#'))
                {
                    out.push(b'"');
                    out.extend(std::iter::repeat_n(b'#', hashes));
                    i += 1 + hashes;
                    break;
                }
                out.push(blank_byte(b[i]));
                i += 1;
            }
        } else if c == b'"' {
            out.push(c);
            i += 1;
            while i < b.len() && b[i] != b'"' {
                if b[i] == b'\\' && i + 1 < b.len() {
                    out.push(b' ');
                    out.push(blank_byte(b[i + 1]));
                    i += 2;
                    continue;
                }
                out.push(blank_byte(b[i]));
                i += 1;
            }
            if i < b.len() {
                out.push(b'"');
                i += 1;
            }
        } else if c == b'\''
            && (i == 0 || !ident_at(i - 1) || (b[i - 1] == b'b' && (i < 2 || !ident_at(i - 2))))
        {
            match char_literal_end(b, i) {
                Some(end) => {
                    out.push(b'\'');
                    out.extend(std::iter::repeat_n(b' ', end - i - 1));
                    out.push(b'\'');
                    i = end + 1;
                }
                None => {
                    out.push(c);
                    i += 1;
                }
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// When a raw string starts at `i` (`r`, `br`, then `#`s, then `"`, with no identifier
/// character before the `r` or the `b`): its number of `#`s and the index of its `"`.
fn raw_string_at(b: &[u8], i: usize) -> Option<(usize, usize)> {
    let start_ok = |k: usize| k == 0 || !is_ident(b[k - 1]);
    let r = if b[i] == b'r' && start_ok(i) {
        i
    } else if b[i] == b'b' && b.get(i + 1) == Some(&b'r') && start_ok(i) {
        i + 1
    } else {
        return None;
    };
    let mut k = r + 1;
    while b.get(k) == Some(&b'#') {
        k += 1;
    }
    (b.get(k) == Some(&b'"')).then_some((k - r - 1, k))
}

/// The index of the `'` that closes a char or byte literal opened at `i`, or `None` when the
/// `'` is a lifetime or a label.
fn char_literal_end(b: &[u8], i: usize) -> Option<usize> {
    let first = *b.get(i + 1)?;
    if first == b'\\' {
        // An escape: `\n`, `\'`, `\x7f`, `\u{10ffff}`: the closing quote is within 12 bytes.
        let mut k = i + 3;
        while k < b.len() && k <= i + 12 {
            if b[k] == b'\'' {
                return Some(k);
            }
            k += 1;
        }
        return None;
    }
    if first == b'\'' || first == b'\n' {
        return None;
    }
    // One character, ASCII or a UTF-8 sequence of up to four bytes.
    let len = match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    };
    (b.get(i + 1 + len) == Some(&b'\'')).then_some(i + 1 + len)
}

fn blank(b: &[u8], char_literals: bool) -> String {
    let blank_byte = |c: u8| if c == b'\n' { b'\n' } else { b' ' };
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied();
        if c == b'/' && next == Some(b'*') {
            out.extend_from_slice(b"  ");
            i += 2;
            while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                out.push(blank_byte(b[i]));
                i += 1;
            }
            if i < b.len() {
                out.extend_from_slice(b"  ");
                i += 2;
            }
        } else if c == b'/' && next == Some(b'/') {
            while i < b.len() && b[i] != b'\n' {
                out.push(b' ');
                i += 1;
            }
        } else if c == b'"' || (char_literals && c == b'\'') {
            out.push(c);
            i += 1;
            while i < b.len() && b[i] != c && (b[i] != b'\n' || !char_literals) {
                if b[i] == b'\\' && i + 1 < b.len() {
                    out.push(b' ');
                    out.push(blank_byte(b[i + 1]));
                    i += 2;
                    continue;
                }
                out.push(blank_byte(b[i]));
                i += 1;
            }
            if i < b.len() && b[i] == c {
                out.push(c);
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// One term of the conditions around a definition: a `#if` expression, negated when the
/// definition sits in a later branch (`#elif`, `#else`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cond {
    pub negated: bool,
    pub expr: String,
}

impl std::fmt::Display for Cond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negated {
            write!(f, "!({})", self.expr)
        } else {
            write!(f, "{}", self.expr)
        }
    }
}

/// A function the C file defines.
#[derive(Debug)]
pub struct CFunc {
    pub name: String,
    pub line: usize,
    pub conds: Vec<Cond>,
}

/// An open `#if` group: the expressions of its earlier branches and of the branch in use
/// (`None` in an `#else`).
struct Frame {
    prior: Vec<String>,
    current: Option<String>,
}

const C_KEYWORDS: &[&str] = &[
    "if",
    "for",
    "while",
    "switch",
    "return",
    "sizeof",
    "do",
    "else",
    "case",
    "goto",
    "defined",
    "typeof",
    "__typeof__",
    "asm",
    "__asm",
    "__asm__",
    "_Static_assert",
];

/// The functions a C file defines, each with the preprocessor conditions around it, and the
/// object-like macros it defines (name and value) for the evaluation of those conditions.
/// A definition is a line that starts in column 0 with `[type words] name(`, whose matching
/// `)` is followed (after `__attribute__((...))` and old-style parameter declarations) by
/// `{`; a prototype, a macro invocation or an initializer is not.
pub fn c_functions(src: &[u8]) -> (Vec<CFunc>, Vec<(String, i64)>) {
    let text = blank_c(src);
    let bytes = text.as_bytes();
    // The start offset of every line.
    let mut starts = vec![0usize];
    for (i, &c) in bytes.iter().enumerate() {
        if c == b'\n' {
            starts.push(i + 1);
        }
    }
    let line_of = |off: usize| starts.partition_point(|&s| s <= off);

    let mut funcs = Vec::new();
    let mut defines = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut seen = HashSet::new();
    let mut n = 0;
    while n < starts.len() {
        let start = starts[n];
        let end = starts.get(n + 1).map_or(bytes.len(), |s| s - 1);
        let line = &text[start..end.max(start)];
        let trimmed = line.trim_start();
        if let Some(directive) = trimmed.strip_prefix('#') {
            // Join the continuation lines.
            let mut full = directive.trim_end().to_string();
            while full.ends_with('\\') && n + 1 < starts.len() {
                full.pop();
                n += 1;
                let s = starts[n];
                let e = starts.get(n + 1).map_or(bytes.len(), |x| x - 1);
                full.push(' ');
                full.push_str(text[s..e.max(s)].trim());
            }
            let full = full.trim_start();
            let (word, rest) = full
                .split_once(|c: char| c.is_whitespace() || c == '(' || c == '!')
                .map_or((full, ""), |(w, _)| (w, &full[w.len()..]));
            let rest = rest.trim();
            match word {
                "if" => stack.push(Frame {
                    prior: Vec::new(),
                    current: Some(rest.to_string()),
                }),
                "ifdef" => stack.push(Frame {
                    prior: Vec::new(),
                    current: Some(format!("defined({rest})")),
                }),
                "ifndef" => stack.push(Frame {
                    prior: Vec::new(),
                    current: Some(format!("!defined({rest})")),
                }),
                "elif" => {
                    if let Some(f) = stack.last_mut() {
                        if let Some(c) = f.current.take() {
                            f.prior.push(c);
                        }
                        f.current = Some(rest.to_string());
                    }
                }
                "else" => {
                    if let Some(f) = stack.last_mut()
                        && let Some(c) = f.current.take()
                    {
                        f.prior.push(c);
                    }
                }
                "endif" => {
                    stack.pop();
                }
                "define" => {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    let after = &rest[name.len()..];
                    if !name.is_empty() && !after.starts_with('(') {
                        let value = parse_int(after.trim()).unwrap_or(1);
                        defines.push((name, value));
                    }
                }
                _ => {}
            }
            n += 1;
            continue;
        }
        if let Some(name) = definition_at(bytes, start)
            && seen.insert(name.clone())
        {
            let conds = stack
                .iter()
                .flat_map(|f| {
                    f.prior
                        .iter()
                        .map(|p| Cond {
                            negated: true,
                            expr: p.clone(),
                        })
                        .chain(f.current.iter().map(|c| Cond {
                            negated: false,
                            expr: c.clone(),
                        }))
                })
                .collect();
            funcs.push(CFunc {
                name,
                line: line_of(start),
                conds,
            });
        }
        n += 1;
    }
    (funcs, defines)
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// The name of the function whose definition starts at `start` (a line start), if one does.
fn definition_at(b: &[u8], start: usize) -> Option<String> {
    let first = *b.get(start)?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    // The prefix up to the first `(` on this line: words and `*` only, and attributes
    // (`__attribute__((weak)) void`), which are stepped over.
    let mut i = start;
    let mut last_word = (0, 0);
    loop {
        while i < b.len() && b[i] != b'(' {
            let c = b[i];
            if c == b'\n' {
                return None;
            }
            if is_ident(c) {
                let s = i;
                while i < b.len() && is_ident(b[i]) {
                    i += 1;
                }
                last_word = (s, i);
                continue;
            }
            if !(c == b' ' || c == b'\t' || c == b'*') {
                return None;
            }
            i += 1;
        }
        if i >= b.len() || last_word.1 == 0 {
            return None;
        }
        if &b[last_word.0..last_word.1] == b"__attribute__" {
            i = skip_parens(b, i)?;
            last_word = (0, 0);
            continue;
        }
        break;
    }
    // Nothing but blanks between the name and `(`.
    if b[last_word.1..i].iter().any(|c| !c.is_ascii_whitespace()) {
        return None;
    }
    let name = String::from_utf8_lossy(&b[last_word.0..last_word.1]).into_owned();
    if name.as_bytes()[0].is_ascii_digit() || C_KEYWORDS.contains(&name.as_str()) {
        return None;
    }
    // The matching `)`.
    let mut j = skip_parens(b, i)?;
    loop {
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if b[j..].starts_with(b"__attribute__") {
            j += "__attribute__".len();
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if b.get(j) != Some(&b'(') {
                return None;
            }
            j = skip_parens(b, j)?;
            continue;
        }
        break;
    }
    match b.get(j)? {
        b'{' => Some(name),
        c if is_ident(*c) => {
            // Old-style parameter declarations: nothing after the `)` on its line, then
            // indented lines that end in `;`, then `{` in column 0.
            let close = skip_parens(b, i)?;
            let eol = b[close..]
                .iter()
                .position(|&c| c == b'\n')
                .map(|p| close + p)?;
            if b[close..eol].iter().any(|c| !c.is_ascii_whitespace()) {
                return None;
            }
            let mut k = eol + 1;
            while k < b.len() {
                let end = b[k..]
                    .iter()
                    .position(|&c| c == b'\n')
                    .map_or(b.len(), |p| k + p);
                let line = &b[k..end];
                if line.first() == Some(&b'{') {
                    return Some(name);
                }
                let text = String::from_utf8_lossy(line);
                let t = text.trim();
                if t.is_empty() || !line[0].is_ascii_whitespace() || !t.ends_with(';') {
                    return None;
                }
                k = end + 1;
            }
            None
        }
        _ => None,
    }
}

/// The offset after the `)` matching the `(` at `open`.
fn skip_parens(b: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut k = open;
    while k < b.len() {
        match b[k] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(k + 1);
                }
            }
            _ => {}
        }
        k += 1;
    }
    None
}

/// The code zone of a Rust file (the whole file when it has no zones).
fn code_zone(src: &str) -> &str {
    let from = src
        .find("/* <CODE> */")
        .map_or(0, |i| i + "/* <CODE> */".len());
    let to = src[from..]
        .find("/* </CODE> */")
        .map_or(src.len(), |i| from + i);
    &src[from..to]
}

/// The names of the `fn`s and `macro_rules!` of a Rust file's code zone.
pub fn rust_fns(src: &str) -> HashSet<String> {
    let text = blank_rust(code_zone(src).as_bytes());
    let b = text.as_bytes();
    let mut out = HashSet::new();
    for kw in ["fn", "macro_rules!"] {
        let mut from = 0;
        while let Some(pos) = text[from..].find(kw) {
            let at = from + pos;
            from = at + kw.len();
            if at > 0 && is_ident(b[at - 1]) {
                continue;
            }
            let mut k = at + kw.len();
            if kw == "fn" && !b.get(k).is_some_and(|c| c.is_ascii_whitespace()) {
                continue;
            }
            while k < b.len() && b[k].is_ascii_whitespace() {
                k += 1;
            }
            if b[k..].starts_with(b"r#") {
                k += 2;
            }
            let s = k;
            while k < b.len() && is_ident(b[k]) {
                k += 1;
            }
            if k > s {
                out.insert(text[s..k].to_string());
            }
        }
    }
    out.extend(macro_made_fns(&text));
    out
}

/// The `fn`s a file's own `macro_rules!` make from their invocations: for each macro of
/// one arm whose matcher is a comma-separated list of `$name:frag` and whose body has
/// `fn $name`, the argument at that position of every `name!(..)` invocation in the file.
/// A name the macro only takes as a type or calls is not one (the user's decision of
/// 2026-11-19: hmac.c's HMAC_*_Init/Update/Final, xform.c's *Update_int).
fn macro_made_fns(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(pos) = text[from..].find("macro_rules!") {
        let at = from + pos;
        from = at + "macro_rules!".len();
        let rest = &text[from..];
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let Some(open) = rest.find('{') else { continue };
        let Some(body_end) = matching(b, from + open, b'{', b'}') else {
            continue;
        };
        let def = &text[from + open + 1..body_end];
        // One arm: `( matcher ) => { body }`.
        let Some(m_open) = def.find('(') else {
            continue;
        };
        let Some(m_close) = matching(def.as_bytes(), m_open, b'(', b')') else {
            continue;
        };
        let matcher = &def[m_open + 1..m_close];
        let body = &def[m_close..];
        let params: Vec<&str> = matcher
            .split(',')
            .map(|p| {
                p.trim()
                    .strip_prefix('$')
                    .and_then(|p| p.split(':').next())
                    .unwrap_or("")
            })
            .collect();
        let made: Vec<usize> = params
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.is_empty() && body.contains(&format!("fn ${p}")))
            .map(|(i, _)| i)
            .collect();
        if made.is_empty() || name.is_empty() {
            continue;
        }
        let call = format!("{name}!(");
        let mut f = 0;
        while let Some(cp) = text[f..].find(&call) {
            let c_at = f + cp;
            f = c_at + call.len();
            if c_at > 0 && is_ident(b[c_at - 1]) {
                continue;
            }
            let Some(close) = matching(b, c_at + call.len() - 1, b'(', b')') else {
                continue;
            };
            let args: Vec<&str> = text[c_at + call.len()..close]
                .split(',')
                .map(str::trim)
                .collect();
            for &i in &made {
                if let Some(a) = args.get(i)
                    && !a.is_empty()
                    && a.bytes().all(is_ident)
                {
                    out.push((*a).to_string());
                }
            }
        }
    }
    out
}

/// The index of the bracket that closes the one at `open`.
fn matching(b: &[u8], open: usize, o: u8, c: u8) -> Option<usize> {
    let mut depth = 0usize;
    for (k, &x) in b.iter().enumerate().skip(open) {
        if x == o {
            depth += 1;
        } else if x == c {
            depth -= 1;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}

/// The `unported!` and `db_stub!` sites of a Rust file's code zone, as 1-based line numbers
/// of the file. The use of `unported!` inside `db_stub!`'s own definition (`$what`) is not
/// a site.
pub fn stub_sites(src: &str) -> Vec<usize> {
    let zone = code_zone(src);
    let offset_lines = src[..src.len() - src[src.find(zone).unwrap_or(0)..].len()]
        .matches('\n')
        .count();
    let text = blank_rust(zone.as_bytes());
    let b = text.as_bytes();
    let mut lines = Vec::new();
    for kw in ["unported!", "db_stub!"] {
        let mut from = 0;
        while let Some(pos) = text[from..].find(kw) {
            let at = from + pos;
            from = at + kw.len();
            if at > 0 && is_ident(b[at - 1]) {
                continue;
            }
            let rest = text[from..].trim_start();
            if !rest.starts_with('(') || rest[1..].trim_start().starts_with('$') {
                continue;
            }
            lines.push(offset_lines + text[..at].matches('\n').count() + 1);
        }
    }
    lines.sort_unstable();
    lines
}

/// One preprocessor directive: its word (`if`, `define`, `include`, ...) and the rest of
/// its line, continuation lines joined.
type Directive = (String, String);

/// The directives of a C file or header, comments and strings blanked.
fn directives(src: &[u8]) -> Vec<Directive> {
    let text = blank_c(src);
    // The blanking keeps every byte's offset, so an `#include`'s file name, a string the
    // blanking emptied, is read back from the source at the same place.
    let raw = String::from_utf8_lossy(src);
    let mut out = Vec::new();
    let mut lines = text.lines().zip(raw.lines());
    while let Some((line, raw_line)) = lines.next() {
        let Some(d) = line.trim_start().strip_prefix('#') else {
            continue;
        };
        let mut full = d.trim_end().to_string();
        while full.ends_with('\\') {
            full.pop();
            match lines.next() {
                Some((next, _)) => {
                    full.push(' ');
                    full.push_str(next.trim());
                }
                None => break,
            }
        }
        let full = full.trim_start();
        let word: String = full
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let rest = if word == "include" {
            let r = raw_line.trim_start().trim_start_matches('#').trim_start();
            r.strip_prefix("include").unwrap_or(r).trim().to_string()
        } else {
            full[word.len()..].trim().to_string()
        };
        out.push((word, rest));
    }
    out
}

/// The object-like macros a C file has defined by its end on one machine: its own
/// `#define`s and `#undef`s and those of the headers of the reference tree it includes
/// (`<a/b.h>` is `sys/a/b.h`, `<machine/x.h>` the machine's `include/x.h`, `"x.h"` the
/// including file's directory first), each applied only in a branch that is taken under
/// `env`. A `#define` in a branch the evaluation cannot decide is kept, so that no
/// function becomes "not configured" by a guess. The user's finding of 2026-11-19: the
/// earlier list took every `#define` of the file, whatever its branch (`crc32.c`'s
/// `DYNAMIC_CRC_TABLE` under `MAKECRCH`), and none of its headers (`wscons_features.h`'s
/// `HAVE_JUMP_SCROLL`, `zutil.h`'s `HAVE_MEMCPY`).
pub struct Preprocessor<'a> {
    reference: &'a Path,
    cache: std::cell::RefCell<HashMap<std::path::PathBuf, std::rc::Rc<Vec<Directive>>>>,
}

impl<'a> Preprocessor<'a> {
    /// A pass over the reference tree at `reference`.
    pub fn new(reference: &'a Path) -> Self {
        Preprocessor {
            reference,
            cache: std::cell::RefCell::new(HashMap::new()),
        }
    }

    fn load(&self, path: &Path) -> Option<std::rc::Rc<Vec<Directive>>> {
        if let Some(d) = self.cache.borrow().get(path) {
            return Some(d.clone());
        }
        let src = fs::read(path).ok()?;
        let d = std::rc::Rc::new(directives(&src));
        self.cache
            .borrow_mut()
            .insert(path.to_path_buf(), d.clone());
        Some(d)
    }

    /// The defines at the end of the C file `c` (a path under the reference tree, `sys/...`)
    /// on `arch`, evaluated under `env`.
    pub fn defines(&self, env: &Env, arch: &str, c: &str) -> Vec<(String, i64)> {
        let mut defs = Vec::new();
        let mut seen = HashSet::new();
        let path = self.reference.join(c);
        self.run(env, arch, &path, &mut defs, &mut seen, 0);
        defs
    }

    fn resolve(&self, arch: &str, from: &Path, rest: &str) -> Option<std::path::PathBuf> {
        let rest = rest.trim();
        let (name, quoted) = match rest.strip_prefix('<') {
            Some(r) => (r.split('>').next()?, false),
            None => (rest.strip_prefix('"')?.split('"').next()?, true),
        };
        if quoted
            && let Some(dir) = from.parent()
            && dir.join(name).is_file()
        {
            return Some(dir.join(name));
        }
        let p = match name.strip_prefix("machine/") {
            Some(m) => self.reference.join(format!("sys/arch/{arch}/include/{m}")),
            None => self.reference.join("sys").join(name),
        };
        p.is_file().then_some(p)
    }

    fn run(
        &self,
        env: &Env,
        arch: &str,
        path: &Path,
        defs: &mut Vec<(String, i64)>,
        seen: &mut HashSet<std::path::PathBuf>,
        depth: usize,
    ) {
        if depth > 24 || !seen.insert(path.to_path_buf()) {
            return;
        }
        let Some(dirs) = self.load(path) else { return };
        // Each frame: (the enclosing branch is live, this branch is live, a branch of this
        // `#if` was already taken, this `#if` is undecided so far).
        let mut stack: Vec<(bool, bool, bool, bool)> = Vec::new();
        let live = |stack: &[(bool, bool, bool, bool)]| stack.last().is_none_or(|f| f.0 && f.1);
        let test = |expr: &str, defs: &[(String, i64)]| -> Option<bool> {
            env.eval(expr, defs).map(|v| v != 0)
        };
        for (word, rest) in dirs.iter() {
            match word.as_str() {
                "if" | "ifdef" | "ifndef" => {
                    let outer = live(&stack);
                    let expr = match word.as_str() {
                        "ifdef" => format!("defined({rest})"),
                        "ifndef" => format!("!defined({rest})"),
                        _ => rest.clone(),
                    };
                    let v = if outer {
                        test(&expr, defs)
                    } else {
                        Some(false)
                    };
                    // Undecided: live, so that its defines are kept.
                    stack.push((outer, v != Some(false), v == Some(true), v.is_none()));
                }
                "elif" => {
                    if let Some(f) = stack.last_mut() {
                        if f.2 && !f.3 {
                            f.1 = false;
                        } else {
                            let v = if f.0 { test(rest, defs) } else { Some(false) };
                            f.1 = v != Some(false);
                            f.2 |= v == Some(true);
                            f.3 |= v.is_none();
                        }
                    }
                }
                "else" => {
                    if let Some(f) = stack.last_mut() {
                        f.1 = !f.2 || f.3;
                    }
                }
                "endif" => {
                    stack.pop();
                }
                "define" if live(&stack) => {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    let after = &rest[name.len()..];
                    if !name.is_empty() && !after.starts_with('(') {
                        let value = parse_int(after.trim()).unwrap_or(1);
                        defs.push((name, value));
                    }
                }
                "undef" if live(&stack) => {
                    let name = rest.split_whitespace().next().unwrap_or("");
                    defs.retain(|(n, _)| n != name);
                }
                "include" if live(&stack) => {
                    if let Some(h) = self.resolve(arch, path, rest) {
                        self.run(env, arch, &h, defs, seen, depth + 1);
                    }
                }
                _ => {}
            }
        }
    }
}

/// A C integer literal (decimal, hex or octal, with `U`/`L` suffixes).
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim_end_matches(['u', 'U', 'l', 'L']);
    let s = s.trim_start_matches('(').trim_end_matches(')');
    // A literal past i64 (ULONG_MAX's 0xffffffffffffffff) keeps its 64 bits, as the C's
    // unsigned arithmetic of `#if` compares them.
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok().map(|v| v as i64)
    } else if s.len() > 1 && s.starts_with('0') {
        u64::from_str_radix(&s[1..], 8).ok().map(|v| v as i64)
    } else {
        s.parse::<i64>()
            .ok()
            .or_else(|| s.parse::<u64>().ok().map(|v| v as i64))
    }
}

/// The macros a configuration defines, for the evaluation of `#if` expressions.
pub struct Env {
    /// Each macro's value: `None` when it is defined to something that is not a number.
    defs: HashMap<String, Option<i64>>,
}

impl Env {
    /// OpenBSD's `GENERIC.MP` for `arch` (`amd64` or `arm64`), read from the reference tree.
    pub fn generic_mp(reference: &Path, arch: &'static str) -> Result<Env> {
        let mut defs: HashMap<String, Option<i64>> = HashMap::new();
        for (k, v) in [
            ("_KERNEL", 1),
            ("__OpenBSD__", 1),
            ("__ELF__", 1),
            ("__STDC__", 1),
            ("__clang__", 1),
            ("__GNUC__", 4),
            ("__LP64__", 1),
            ("_LP64", 1),
            ("_LITTLE_ENDIAN", 1234),
            ("_BIG_ENDIAN", 4321),
            ("_PDP_ENDIAN", 3412),
            ("_BYTE_ORDER", 1234),
            ("LITTLE_ENDIAN", 1234),
            ("BIG_ENDIAN", 4321),
            ("PDP_ENDIAN", 3412),
            ("BYTE_ORDER", 1234),
            // OpenBSD's clang turns retguard on by default on x86_64 and aarch64 and then
            // defines _RET_PROTECTOR (gnu/llvm/clang/lib/Driver/ToolChains/Clang.cpp:6935),
            // unless -fno-stack-protector or -pg, which the kernel's Makefiles do not pass.
            ("_RET_PROTECTOR", 1),
        ] {
            defs.insert(k.to_string(), Some(v));
        }
        let arch_syms: &[&str] = match arch {
            "amd64" => &["__amd64__", "__amd64", "__x86_64__", "__x86_64"],
            _ => &["__aarch64__", "__arm64__"],
        };
        for s in arch_syms {
            defs.insert((*s).to_string(), Some(1));
        }
        let configs = [
            "sys/conf/GENERIC".to_string(),
            format!("sys/arch/{arch}/conf/GENERIC"),
            format!("sys/arch/{arch}/conf/GENERIC.MP"),
        ];
        for cfg in &configs {
            let path = reference.join(cfg);
            let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            for line in text.lines() {
                let line = line.split('#').next().unwrap_or("");
                let mut words = line.split_whitespace();
                let Some(first) = words.next() else { continue };
                match first {
                    "option" | "options" => {
                        for w in words {
                            let (name, value) = w.split_once('=').map_or((w, 1), |(n, v)| {
                                (n, parse_int(v.trim_matches('"')).unwrap_or(1))
                            });
                            defs.insert(name.to_string(), Some(value));
                        }
                    }
                    "pseudo-device" => {
                        if let Some(name) = words.next() {
                            let count = words.next().and_then(parse_int).unwrap_or(1);
                            defs.insert(format!("N{}", name.to_ascii_uppercase()), Some(count));
                        }
                    }
                    _ => {
                        // A device: `name0 at parent?`, `name* at parent?`.
                        if words.next() == Some("at") {
                            let name =
                                first.trim_end_matches(|c: char| c.is_ascii_digit() || c == '*');
                            if !name.is_empty() {
                                let key = format!("N{}", name.to_ascii_uppercase());
                                let count = defs.get(&key).copied().flatten().unwrap_or(0) + 1;
                                defs.insert(key, Some(count));
                            }
                        }
                    }
                }
            }
        }
        let include = reference.join(format!("sys/arch/{arch}/include"));
        if let Ok(dir) = fs::read_dir(&include) {
            for f in dir.flatten() {
                let Ok(src) = fs::read(f.path()) else {
                    continue;
                };
                for line in blank_c(&src).lines() {
                    let line = line.trim_start();
                    let Some(rest) = line.strip_prefix('#') else {
                        continue;
                    };
                    let Some(rest) = rest.trim_start().strip_prefix("define") else {
                        continue;
                    };
                    let mut words = rest.split_whitespace();
                    let Some(name) = words.next() else { continue };
                    // Object-like macros, wherever they sit in the header: defined for
                    // `defined()`, with their value when it is empty (1) or a number
                    // (`__HAVE_*`; amd64's `PMAP_CHECK_COPYIN` is `(pg_xo == 0)`).
                    if name.contains('(') {
                        continue;
                    }
                    match words.next() {
                        None => {
                            defs.insert(name.to_string(), Some(1));
                        }
                        Some(v) => {
                            defs.insert(name.to_string(), parse_int(v));
                        }
                    }
                }
            }
        }
        Ok(Env { defs })
    }

    /// The value of `conds` all together: `Some(true)`, `Some(false)`, or `None` when a term
    /// cannot be decided (a function-like macro, an identifier of unknown value).
    pub fn eval_all(&self, conds: &[Cond], local: &[(String, i64)]) -> Option<bool> {
        let mut all = Some(true);
        for c in conds {
            let v = self.eval(&c.expr, local).map(|v| (v != 0) != c.negated);
            all = match (all, v) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            };
        }
        all
    }

    pub fn eval(&self, expr: &str, local: &[(String, i64)]) -> Option<i64> {
        let toks = tokenize(expr);
        let mut p = Parser {
            toks: &toks,
            pos: 0,
            env: self,
            local,
        };
        let v = p.ternary();
        if p.pos == toks.len() { v } else { None }
    }

    fn lookup(&self, name: &str, local: &[(String, i64)]) -> Option<Option<i64>> {
        if let Some((_, v)) = local.iter().rev().find(|(n, _)| n == name) {
            return Some(Some(*v));
        }
        self.defs.get(name).copied()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(i64),
    Ident(String),
    Op(&'static str),
}

fn tokenize(s: &str) -> Vec<Tok> {
    const OPS: &[&str] = &[
        "&&", "||", "==", "!=", "<=", ">=", "<<", ">>", "!", "<", ">", "(", ")", "+", "-", "*",
        "/", "%", "&", "|", "^", "~", "?", ":", ",",
    ];
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    'outer: while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let st = i;
            while i < b.len() && is_ident(b[i]) {
                i += 1;
            }
            out.push(parse_int(&s[st..i]).map_or(Tok::Ident(s[st..i].to_string()), Tok::Num));
            continue;
        }
        if is_ident(c) {
            let st = i;
            while i < b.len() && is_ident(b[i]) {
                i += 1;
            }
            out.push(Tok::Ident(s[st..i].to_string()));
            continue;
        }
        for op in OPS {
            if s[i..].starts_with(op) {
                out.push(Tok::Op(op));
                i += op.len();
                continue 'outer;
            }
        }
        // Anything else makes the expression undecidable.
        out.push(Tok::Op("?unknown"));
        i += 1;
    }
    out
}

struct Parser<'a> {
    toks: &'a [Tok],
    pos: usize,
    env: &'a Env,
    local: &'a [(String, i64)],
}

impl Parser<'_> {
    fn peek_op(&self, op: &str) -> bool {
        matches!(self.toks.get(self.pos), Some(Tok::Op(o)) if *o == op)
    }

    fn eat(&mut self, op: &str) -> bool {
        if self.peek_op(op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn ternary(&mut self) -> Option<i64> {
        let c = self.binary(0);
        if self.eat("?") {
            let a = self.ternary();
            if !self.eat(":") {
                self.pos = self.toks.len() + 1;
                return None;
            }
            let b = self.ternary();
            return match c {
                Some(0) => b,
                Some(_) => a,
                None => None,
            };
        }
        c
    }

    fn binary(&mut self, min: u8) -> Option<i64> {
        const LEVELS: &[(&str, u8)] = &[
            ("||", 1),
            ("&&", 2),
            ("|", 3),
            ("^", 4),
            ("&", 5),
            ("==", 6),
            ("!=", 6),
            ("<", 7),
            (">", 7),
            ("<=", 7),
            (">=", 7),
            ("<<", 8),
            (">>", 8),
            ("+", 9),
            ("-", 9),
            ("*", 10),
            ("/", 10),
            ("%", 10),
        ];
        let mut lhs = self.unary();
        while let Some(Tok::Op(op)) = self.toks.get(self.pos) {
            let Some(&(_, prec)) = LEVELS.iter().find(|(o, _)| o == op) else {
                break;
            };
            if prec < min {
                break;
            }
            let op = *op;
            self.pos += 1;
            let rhs = self.binary(prec + 1);
            lhs = match op {
                "||" => match (lhs, rhs) {
                    (Some(a), _) if a != 0 => Some(1),
                    (_, Some(b)) if b != 0 => Some(1),
                    (Some(_), Some(_)) => Some(0),
                    _ => None,
                },
                "&&" => match (lhs, rhs) {
                    (Some(0), _) | (_, Some(0)) => Some(0),
                    (Some(_), Some(_)) => Some(1),
                    _ => None,
                },
                _ => match (lhs, rhs) {
                    (Some(a), Some(b)) => match op {
                        "|" => Some(a | b),
                        "^" => Some(a ^ b),
                        "&" => Some(a & b),
                        "==" => Some(i64::from(a == b)),
                        "!=" => Some(i64::from(a != b)),
                        "<" => Some(i64::from(a < b)),
                        ">" => Some(i64::from(a > b)),
                        "<=" => Some(i64::from(a <= b)),
                        ">=" => Some(i64::from(a >= b)),
                        "<<" => a.checked_shl(u32::try_from(b).ok()?),
                        ">>" => a.checked_shr(u32::try_from(b).ok()?),
                        "+" => a.checked_add(b),
                        "-" => a.checked_sub(b),
                        "*" => a.checked_mul(b),
                        "/" => a.checked_div(b),
                        _ => a.checked_rem(b),
                    },
                    _ => None,
                },
            };
        }
        lhs
    }

    fn unary(&mut self) -> Option<i64> {
        if self.eat("!") {
            return self.unary().map(|v| i64::from(v == 0));
        }
        if self.eat("-") {
            return self.unary().map(|v| -v);
        }
        if self.eat("+") {
            return self.unary();
        }
        if self.eat("~") {
            return self.unary().map(|v| !v);
        }
        if self.eat("(") {
            let v = self.ternary();
            if !self.eat(")") {
                self.pos = self.toks.len() + 1;
                return None;
            }
            return v;
        }
        let tok = self.toks.get(self.pos).cloned();
        self.pos += 1;
        match tok {
            Some(Tok::Num(n)) => Some(n),
            Some(Tok::Ident(name)) if name == "defined" => {
                let paren = self.eat("(");
                let Some(Tok::Ident(sym)) = self.toks.get(self.pos).cloned() else {
                    self.pos = self.toks.len() + 1;
                    return None;
                };
                self.pos += 1;
                if paren && !self.eat(")") {
                    self.pos = self.toks.len() + 1;
                    return None;
                }
                Some(i64::from(self.env.lookup(&sym, self.local).is_some()))
            }
            Some(Tok::Ident(name)) => {
                if self.peek_op("(") {
                    // A function-like macro: undecidable here.
                    if let Some(end) = self.matching_paren() {
                        self.pos = end;
                    }
                    return None;
                }
                match self.env.lookup(&name, self.local) {
                    Some(v) => v,
                    // An undefined identifier is 0 in `#if`; a device count of a device
                    // GENERIC does not have is that 0. Anything else may be a macro of a
                    // header this reading does not follow.
                    None if is_count(&name) => Some(0),
                    None => None,
                }
            }
            _ => {
                self.pos = self.toks.len() + 1;
                None
            }
        }
    }

    fn matching_paren(&self) -> Option<usize> {
        let mut depth = 0;
        for (k, t) in self.toks.iter().enumerate().skip(self.pos) {
            match t {
                Tok::Op("(") => depth += 1,
                Tok::Op(")") => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(k + 1);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

/// `N<NAME>`: a `needs-count`/`needs-flag` header's device count.
fn is_count(name: &str) -> bool {
    name.len() > 1
        && name.starts_with('N')
        && name[1..]
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
}

/// A C function a row's Rust file lacks.
pub struct Missing<'a> {
    pub row: &'a Entry,
    pub func: CFunc,
    /// What the conditions come to on amd64 and on arm64.
    pub configured: [Option<bool>; 2],
    /// Other Rust files under `sys/` that have a `fn` of that name.
    pub elsewhere: Vec<String>,
}

impl Missing<'_> {
    /// The class `ports missing` prints and `--proposal` writes as the reason.
    pub fn class(&self) -> String {
        let conds = || {
            self.func
                .conds
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" && ")
        };
        match self.configured {
            [Some(false), Some(false)] => {
                format!(
                    "not configured: `{}` is false on amd64 and arm64 GENERIC.MP",
                    conds()
                )
            }
            [Some(true), _] | [_, Some(true)] => {
                if self.elsewhere.is_empty() {
                    if self.func.conds.is_empty() {
                        "missing".to_string()
                    } else {
                        format!("missing (configured: `{}`)", conds())
                    }
                } else {
                    format!("in another file: {}", self.elsewhere.join(", "))
                }
            }
            _ => format!("condition not decided by the tool: `{}`", conds()),
        }
    }
}

/// What a function's conditions come to on amd64 (`[0]`) and arm64 (`[1]`), `eval(i)` being
/// the value under that machine's `GENERIC.MP`. The user's rule of 2026-11-19: a function is
/// required when its `#if` is true on at least one of the two (no other architecture
/// counts), and a file under `sys/arch/amd64/` or `sys/arch/arm64/` is evaluated on its own
/// machine only, its value standing for both.
fn configured_on(c: &str, eval: impl Fn(usize) -> Option<bool>) -> [Option<bool>; 2] {
    if c.starts_with("sys/arch/amd64/") {
        let v = eval(0);
        [v, v]
    } else if c.starts_with("sys/arch/arm64/") {
        let v = eval(1);
        [v, v]
    } else {
        [eval(0), eval(1)]
    }
}

/// The completeness of every `ported` row, into `errors`; with the reference present, the
/// functions of every `ported` and `wip` `.c` row that its Rust file lacks are returned for
/// `ports missing`.
pub struct Report<'a> {
    pub missing: Vec<Missing<'a>>,
    pub stub_rows: usize,
    pub stub_sites: usize,
    pub checked_c_rows: usize,
}

fn load_exceptions(root: &Path) -> Result<Exceptions> {
    let path = root.join(EXCEPTIONS_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Exceptions::default()),
        Err(e) => Err(format!("{}: {e}", path.display()).into()),
    }
}

/// Runs both checks. A `ported` row with a stub site or an unexcepted missing function is an
/// error; an exception that names no row, no function of the C file, a function the Rust
/// file has, or lacks its reason or its approval date, is an error too.
pub fn check<'a>(
    root: &Path,
    rows: &'a [Entry],
    have_reference: bool,
    errors: &mut Vec<String>,
) -> Result<Report<'a>> {
    let exceptions = load_exceptions(root)?;
    let mut excepted: HashMap<(&str, &str), &Exception> = HashMap::new();
    for x in &exceptions.list {
        let tag = format!(
            "{EXCEPTIONS_FILE}: c = \"{}\", function = \"{}\"",
            x.c, x.function
        );
        if x.reason.trim().is_empty() {
            errors.push(format!("{tag}: `reason` is required"));
        }
        if x.approved.len() != 10 || x.approved.as_bytes()[4] != b'-' {
            errors.push(format!(
                "{tag}: `approved` must be the date of the user's approval"
            ));
        }
        if !rows.iter().any(|e| e.c == x.c) {
            errors.push(format!("{tag}: no row in ports.toml"));
        }
        if excepted
            .insert((x.c.as_str(), x.function.as_str()), x)
            .is_some()
        {
            errors.push(format!("{tag}: duplicate"));
        }
    }

    // Every Rust file's fns, for the "in another file" class.
    let mut fns_by_file: BTreeMap<String, HashSet<String>> = BTreeMap::new();
    let mut rs = Vec::new();
    super::walk_rs(&root.join("sys"), &mut rs)?;
    for f in &rs {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let src = fs::read_to_string(f).map_err(|e| format!("{rel}: {e}"))?;
        fns_by_file.insert(rel, rust_fns(&src));
    }

    let mut report = Report {
        missing: Vec::new(),
        stub_rows: 0,
        stub_sites: 0,
        checked_c_rows: 0,
    };
    let reference = root.join(REFERENCE_DIR);
    let pre = Preprocessor::new(&reference);
    let envs = if have_reference {
        Some([
            Env::generic_mp(&reference, "amd64")?,
            Env::generic_mp(&reference, "arm64")?,
        ])
    } else {
        None
    };
    let mut used_exceptions: HashSet<(&str, &str)> = HashSet::new();
    let mut stub_cache: HashMap<&str, Vec<usize>> = HashMap::new();
    for e in rows {
        if !matches!(e.status, Status::Ported | Status::Wip) || e.rust.is_empty() {
            continue;
        }
        let tag = format!("[[file]] c = \"{}\"", e.c);
        if e.status == Status::Ported {
            let sites = stub_cache.entry(e.rust.as_str()).or_insert_with(|| {
                fs::read_to_string(root.join(&e.rust))
                    .map(|s| stub_sites(&s))
                    .unwrap_or_default()
            });
            if !sites.is_empty() {
                report.stub_rows += 1;
                report.stub_sites += sites.len();
                let shown: Vec<String> = sites
                    .iter()
                    .take(5)
                    .map(|l| format!("{}:{l}", e.rust))
                    .collect();
                errors.push(format!(
                    "{tag}: ported, but {} holds {} unported!/db_stub! site(s): {}{}",
                    e.rust,
                    sites.len(),
                    shown.join(", "),
                    if sites.len() > 5 { ", ..." } else { "" }
                ));
            }
        }
        let Some(envs) = &envs else { continue };
        if !e.c.ends_with(".c") {
            continue;
        }
        let Ok(c_src) = fs::read(reference.join(&e.c)) else {
            continue;
        };
        if e.status == Status::Ported {
            report.checked_c_rows += 1;
        }
        let (funcs, _) = c_functions(&c_src);
        let locals = [
            pre.defines(&envs[0], "amd64", &e.c),
            pre.defines(&envs[1], "arm64", &e.c),
        ];
        let have = fns_by_file.get(&e.rust).cloned().unwrap_or_default();
        let mut unexcepted = Vec::new();
        for func in funcs {
            if have.contains(&func.name) {
                if let Some(x) = excepted.get(&(e.c.as_str(), func.name.as_str())) {
                    errors.push(format!(
                        "{EXCEPTIONS_FILE}: c = \"{}\", function = \"{}\": {} has it; drop the \
                         exception",
                        e.c, func.name, e.rust
                    ));
                    used_exceptions.insert((x.c.as_str(), x.function.as_str()));
                }
                continue;
            }
            if let Some(x) = excepted.get(&(e.c.as_str(), func.name.as_str())) {
                used_exceptions.insert((x.c.as_str(), x.function.as_str()));
                continue;
            }
            let configured = configured_on(&e.c, |i| envs[i].eval_all(&func.conds, &locals[i]));
            let elsewhere = fns_by_file
                .iter()
                .filter(|(f, set)| **f != e.rust && set.contains(&func.name))
                .map(|(f, _)| f.clone())
                .collect();
            if e.status == Status::Ported {
                unexcepted.push(func.name.clone());
            }
            report.missing.push(Missing {
                row: e,
                func,
                configured,
                elsewhere,
            });
        }
        if !unexcepted.is_empty() {
            errors.push(format!(
                "{tag}: ported, but {} C function(s) have no fn in {} and no approved exception \
                 ({EXCEPTIONS_FILE}): {}",
                unexcepted.len(),
                e.rust,
                unexcepted.join(", ")
            ));
        }
    }
    if envs.is_some() {
        for x in &exceptions.list {
            if !used_exceptions.contains(&(x.c.as_str(), x.function.as_str())) {
                errors.push(format!(
                    "{EXCEPTIONS_FILE}: c = \"{}\", function = \"{}\": not a function the C file \
                     defines, or its row is not ported/wip",
                    x.c, x.function
                ));
            }
        }
    }
    Ok(report)
}

/// `cargo xtask ports missing [--proposal FILE]`.
pub fn ports_missing(root: &Path, proposal: Option<&Path>) -> Result<()> {
    let ports = super::load_ports(root)?;
    let have_reference = root.join(REFERENCE_DIR).join("sys").is_dir();
    if !have_reference {
        return Err(format!("{REFERENCE_DIR} is not present").into());
    }
    let mut errors = Vec::new();
    let report = check(root, &ports.files, true, &mut errors)?;
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = String::new();
    let mut toml_out = String::from(
        "# Candidate exceptions written by `cargo xtask ports missing --proposal`.\n\
         # NOT APPROVED: an entry goes into ports-exceptions.toml, with `approved` set to the\n\
         # date, only after the user approved it by name.\n",
    );
    for m in &report.missing {
        let class = m.class();
        let key = class
            .split(':')
            .next()
            .unwrap_or("")
            .split(" (")
            .next()
            .unwrap_or("")
            .to_string();
        *classes
            .entry(format!("{} {key}", m.row.status.as_str()))
            .or_default() += 1;
        out.push_str(&format!(
            "{} {}:{} {} -> {}: {class}\n",
            m.row.status.as_str(),
            m.row.c,
            m.func.line,
            m.func.name,
            m.row.rust
        ));
        toml_out.push_str(&format!(
            "\n[[exception]]\nc = \"{}\"\nfunction = \"{}\"\nreason = \"{}\"\napproved = \"\"\n# row status: {}\n",
            m.row.c,
            m.func.name,
            class.replace('\\', "\\\\").replace('"', "\\\""),
            m.row.status.as_str()
        ));
    }
    print!("{out}");
    for (k, v) in &classes {
        println!("total: {v} {k}");
    }
    if let Some(p) = proposal {
        fs::write(p, toml_out).map_err(|e| format!("{}: {e}", p.display()))?;
        println!("proposal written to {}", p.display());
    }
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_definitions_not_prototypes_or_macro_uses() {
        let src = b"/* foo(void) { in a comment } */
int	bar(int);
static int baz(int a) __attribute__((unused));

int
foo(int a, char *s)
{
	return 0;
}

static void qux(void) {
}

LIST_HEAD(, proc) allproc = LIST_HEAD_INITIALIZER(allproc);
TAILQ_HEAD(slpque, proc) slpque[128];
__attribute__((weak)) void hook(void *, int);
__attribute__((weak)) void
weakfn(void *p)
{
}
CTASSERT(sizeof(int) == 4);
RBT_GENERATE(t, n, f, cmp);

#ifdef DDB
void
db_thing(int x)
{
}
#else
void
other(void)
{
}
#endif

int
kr(a, b)
	int a;
	char *b;
{
	return a;
}
";
        let (funcs, _) = c_functions(src);
        let names: Vec<&str> = funcs.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["foo", "qux", "weakfn", "db_thing", "other", "kr"]);
        assert_eq!(
            funcs[3].conds,
            [Cond {
                negated: false,
                expr: "defined(DDB)".into()
            }]
        );
        assert_eq!(
            funcs[4].conds,
            [Cond {
                negated: true,
                expr: "defined(DDB)".into()
            }]
        );
        assert_eq!(funcs[0].line, 6);
    }

    #[test]
    fn elif_negates_the_earlier_branches() {
        let src = b"#if A
#elif B
void
f(void)
{
}
#endif
";
        let (funcs, _) = c_functions(src);
        assert_eq!(
            funcs[0].conds,
            [
                Cond {
                    negated: true,
                    expr: "A".into()
                },
                Cond {
                    negated: false,
                    expr: "B".into()
                }
            ]
        );
    }

    #[test]
    fn rust_fns_and_stubs_ignore_comments_strings_and_tests() {
        // The zone markers are put together here, so that this file's own zones stay the
        // only ones `ports check` sees in it.
        let zone =
            |open: bool, name: &str| format!("/* <{}{name}> */", if open { "" } else { "/" });
        let src = [
            zone(true, "LICENSES"),
            zone(false, "LICENSES"),
            String::new(),
            zone(true, "CODE"),
            "//! fn in_a_doc_comment()".into(),
            "pub fn real(x: &'static str) {}".into(),
            "fn r#type() {}".into(),
            "macro_rules! mac { ($w:literal) => { unported!($w) } }".into(),
            "fn g() { let _ = \"fn in_a_string\"; let _ = unported!(\"thing\"); }".into(),
            "fn after_quote_char<'a>(x: &'a u8) -> bool { *x == b'\"' || 'q' == '\\'' }".into(),
            "fn after_raw() { let _ = r#\"fn not_a_fn() \" \"#; let _ = unported!(\"raw\"); }"
                .into(),
            "/* outer /* fn nested_comment() */ fn still_comment() */ fn after_nested() {}".into(),
            "macro_rules! fam { ($t:ty, $hash:ident, $init:ident) => { pub fn $init(x: $t) { $hash(x) } } }".into(),
            "fam!(u8, called_not_made, made_by_macro);".into(),
            "// let _ = unported!(\"commented\");".into(),
            zone(false, "CODE"),
            String::new(),
            zone(true, "TESTS"),
            "fn test_only() { let _ = unported!(\"t\"); }".into(),
            zone(false, "TESTS"),
        ]
        .join("\n");
        let src = src.as_str();
        let fns = rust_fns(src);
        let mut v: Vec<_> = fns.iter().map(String::as_str).collect();
        v.sort_unstable();
        assert_eq!(
            v,
            [
                "after_nested",
                "after_quote_char",
                "after_raw",
                "fam",
                "g",
                "mac",
                "made_by_macro",
                "real",
                "type"
            ]
        );
        assert_eq!(stub_sites(src), [9, 11]);
    }

    fn env(defs: &[(&str, i64)]) -> Env {
        Env {
            defs: defs
                .iter()
                .map(|(k, v)| ((*k).to_string(), Some(*v)))
                .collect(),
        }
    }

    #[test]
    fn the_preprocessor_follows_branches_undefs_and_headers() {
        let root = std::env::temp_dir().join(format!("emibsd-pre-{}", std::process::id()));
        let dir = root.join("sys/dev");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("feat.h"),
            "#ifndef SMALL\n#define FROM_HEADER\n#endif\n#define BIG 0xffffffffffffffffUL\n",
        )
        .unwrap();
        fs::write(
            dir.join("x.c"),
            "#include <dev/feat.h>\n#ifdef NEVER\n#define IN_DEAD_BRANCH\n#endif\n\
             #define GONE\n#undef GONE\n#if BIG == 0xffffffffffffffff\n#define BIG_OK\n#endif\n",
        )
        .unwrap();
        let env = env(&[]);
        let pre = Preprocessor::new(&root);
        let d = pre.defines(&env, "amd64", "sys/dev/x.c");
        let has = |n: &str| d.iter().any(|(k, _)| k == n);
        assert!(has("FROM_HEADER"));
        assert!(!has("IN_DEAD_BRANCH"));
        assert!(!has("GONE"));
        assert!(has("BIG_OK"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_arch_file_is_evaluated_on_its_own_machine() {
        // amd64 true, arm64 false.
        let split = |i: usize| Some(i == 0);
        assert_eq!(
            configured_on("sys/arch/amd64/amd64/pmap.c", split),
            [Some(true), Some(true)]
        );
        assert_eq!(
            configured_on("sys/arch/arm64/arm64/pmap.c", split),
            [Some(false), Some(false)]
        );
        assert_eq!(
            configured_on("sys/uvm/uvm_km.c", split),
            [Some(true), Some(false)]
        );
    }

    #[test]
    fn evaluates_if_expressions_three_ways() {
        let e = env(&[("DDB", 1), ("NBPFILTER", 1), ("MULTIPROCESSOR", 1)]);
        let c = |s: &str| {
            [Cond {
                negated: false,
                expr: s.into(),
            }]
        };
        assert_eq!(e.eval_all(&c("defined(DDB)"), &[]), Some(true));
        assert_eq!(e.eval_all(&c("defined(WITNESS)"), &[]), Some(false));
        assert_eq!(
            e.eval_all(&c("defined(GPROF) || defined(DDBPROF)"), &[]),
            Some(false)
        );
        assert_eq!(e.eval_all(&c("NBPFILTER > 0"), &[]), Some(true));
        assert_eq!(e.eval_all(&c("NCARP > 0"), &[]), Some(false));
        assert_eq!(e.eval_all(&c("0"), &[]), Some(false));
        assert_eq!(e.eval_all(&c("!defined(SMALL_KERNEL)"), &[]), Some(true));
        assert_eq!(e.eval_all(&c("SOME_HEADER_MACRO > 2"), &[]), None);
        assert_eq!(e.eval_all(&c("__GNUC_PREREQ__(4, 7)"), &[]), None);
        assert_eq!(e.eval_all(&c("defined(X) && FOO(1)"), &[]), Some(false));
        assert_eq!(
            e.eval_all(&c("defined(LOCAL_OPT)"), &[("LOCAL_OPT".into(), 1)]),
            Some(true)
        );
        let neg = [Cond {
            negated: true,
            expr: "defined(DDB)".into(),
        }];
        assert_eq!(e.eval_all(&neg, &[]), Some(false));
        // Defined to an expression: defined, of unknown value.
        let mut e = e;
        e.defs.insert("PMAP_CHECK_COPYIN".into(), None);
        assert_eq!(
            e.eval_all(&c("defined(PMAP_CHECK_COPYIN)"), &[]),
            Some(true)
        );
        assert_eq!(e.eval_all(&c("PMAP_CHECK_COPYIN"), &[]), None);
    }
}
/* </TESTS> */
