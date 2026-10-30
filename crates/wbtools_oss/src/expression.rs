//! The single entry point for parsing user expressions with `evalexpr`.
//!
//! The raster calculator and conditional evaluation, the attribute query and
//! field calculator tools, and the LiDAR filter and modify tools all parse
//! through [`build_operator_tree`].
//!
//! This crate pins `evalexpr` 11.3.1, the last MIT-licensed release; 12.0.0 and
//! later are AGPL-3.0-only. Upstream `whitebox_next_gen` depends on 13.1.0, so
//! re-vendoring `wbtools_oss` must keep this file and the 11.3.1 requirement
//! (see MAINTAINING.md).

use evalexpr::{EvalexprResult, Node};
use std::borrow::Cow;

/// Parses `expression` into an evaluator tree.
///
/// Binary (`0b101`) and octal (`0o17`) integer literals are rewritten to
/// decimal first. The tools accepted them while built on `evalexpr` 13.1.0,
/// but 11.3.1 reads them as variable names.
pub(crate) fn build_operator_tree(expression: &str) -> EvalexprResult<Node> {
    evalexpr::build_operator_tree(&rewrite_radix_literals(expression))
}

/// Rewrites each `0b`/`0o` literal whose digits fit an `i64` as a decimal
/// literal. Literals are split where `evalexpr`'s tokenizer splits them;
/// string literals and comments are copied unchanged.
fn rewrite_radix_literals(expression: &str) -> Cow<'_, str> {
    if !expression.contains("0b") && !expression.contains("0o") {
        return Cow::Borrowed(expression);
    }
    let mut out = String::with_capacity(expression.len());
    let mut rest = expression;
    while let Some(c) = rest.chars().next() {
        let len = if c == '"' {
            string_literal_len(rest)
        } else if rest.starts_with("//") {
            rest.find('\n').unwrap_or(rest.len())
        } else if rest.starts_with("/*") {
            rest[2..].find("*/").map_or(rest.len(), |end| end + 4)
        } else if ends_literal(c) {
            c.len_utf8()
        } else {
            let len = rest.find(ends_literal).unwrap_or(rest.len());
            match radix_literal_value(&rest[..len]) {
                Some(value) => out.push_str(&value.to_string()),
                None => out.push_str(&rest[..len]),
            }
            rest = &rest[len..];
            continue;
        };
        out.push_str(&rest[..len]);
        rest = &rest[len..];
    }
    Cow::Owned(out)
}

/// Whether `c` ends a number or identifier in `evalexpr`'s tokenizer.
fn ends_literal(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '+' | '-' | '*' | '/' | '%' | '^' | '(' | ')' | ',' | ';' | '=' | '!' | '>' | '<'
                | '&' | '|' | '"'
        )
}

/// Byte length of the string literal at the start of `s` (which starts with
/// `"`), including its closing quote; the rest of `s` if it is unterminated.
fn string_literal_len(s: &str) -> usize {
    let mut escaped = false;
    for (idx, c) in s.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return idx + 1;
        }
    }
    s.len()
}

/// The value of a `0b`/`0o` literal token, if it is one and fits an `i64`.
fn radix_literal_value(token: &str) -> Option<i64> {
    let (digits, radix) = match token.strip_prefix("0b") {
        Some(digits) => (digits, 2),
        None => (token.strip_prefix("0o")?, 8),
    };
    i64::from_str_radix(digits, radix).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use evalexpr::Value;

    fn eval(expression: &str) -> EvalexprResult<Value> {
        build_operator_tree(expression)?.eval()
    }

    #[test]
    fn radix_literals_evaluate_as_integers() {
        assert_eq!(eval("0b101"), Ok(Value::Int(5)));
        assert_eq!(eval("0o17"), Ok(Value::Int(15)));
        assert_eq!(eval("-0b101 + 0o10*2"), Ok(Value::Int(11)));
        assert_eq!(eval("0x1F"), Ok(Value::Int(31)));
        assert_eq!(
            eval("0b111111111111111111111111111111111111111111111111111111111111111"),
            Ok(Value::Int(i64::MAX))
        );
    }

    #[test]
    fn non_radix_tokens_are_untouched() {
        for text in [
            "0B101", "0b", "0b2", "0o8", "0b1_0", "0b101.5", "x0b1", "00b1",
            // One bit too many for an i64.
            "0b1111111111111111111111111111111111111111111111111111111111111111",
            "\"0b101\"", "\"a\\\"0b1\"", "/* 0b1 */ 1", "/*/ 0b1 */", "1 // 0b1",
        ] {
            assert_eq!(rewrite_radix_literals(text), text, "{text}");
        }
    }
}
