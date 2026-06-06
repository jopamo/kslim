use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum KconfigExpr {
    Symbol(String),
    Literal(TristateLiteral),
    StringLiteral(String),
    Not(Box<KconfigExpr>),
    And(Box<KconfigExpr>, Box<KconfigExpr>),
    Or(Box<KconfigExpr>, Box<KconfigExpr>),
    Eq(Box<KconfigExpr>, Box<KconfigExpr>),
    Ne(Box<KconfigExpr>, Box<KconfigExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TristateLiteral {
    Y,
    M,
    N,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KconfigConst {
    Tristate(TristateLiteral),
    String(String),
}

const KCONFIG_EXPR_EQUIVALENCE_SYMBOL_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExprToken {
    Symbol(String),
    StringLiteral(String),
    Not,
    Eq,
    Ne,
    And,
    Or,
    LParen,
    RParen,
}

pub(super) fn parse_kconfig_expr(input: &str) -> Option<KconfigExpr> {
    let tokens = tokenize_kconfig_expr(input)?;
    let mut idx = 0usize;
    let expr = parse_kconfig_or_expr(&tokens, &mut idx)?;
    if idx != tokens.len() {
        return None;
    }
    if !string_literals_are_comparison_operands(&expr) {
        return None;
    }
    Some(expr)
}

fn tokenize_kconfig_expr(input: &str) -> Option<Vec<ExprToken>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut idx = 0usize;

    while idx < chars.len() {
        match chars[idx] {
            ' ' | '\t' => idx += 1,
            '!' => {
                if chars.get(idx + 1) == Some(&'=') {
                    tokens.push(ExprToken::Ne);
                    idx += 2;
                } else {
                    tokens.push(ExprToken::Not);
                    idx += 1;
                }
            }
            '=' => {
                tokens.push(ExprToken::Eq);
                idx += 1;
            }
            '&' => {
                if chars.get(idx + 1) != Some(&'&') {
                    return None;
                }
                tokens.push(ExprToken::And);
                idx += 2;
            }
            '|' => {
                if chars.get(idx + 1) != Some(&'|') {
                    return None;
                }
                tokens.push(ExprToken::Or);
                idx += 2;
            }
            '(' => {
                tokens.push(ExprToken::LParen);
                idx += 1;
            }
            ')' => {
                tokens.push(ExprToken::RParen);
                idx += 1;
            }
            '"' => {
                idx += 1;
                let mut value = String::new();
                let mut closed = false;
                while idx < chars.len() {
                    match chars[idx] {
                        '\\' if idx + 1 < chars.len() => {
                            idx += 1;
                            value.push(chars[idx]);
                            idx += 1;
                        }
                        '"' => {
                            idx += 1;
                            closed = true;
                            break;
                        }
                        ch => {
                            value.push(ch);
                            idx += 1;
                        }
                    }
                }
                if !closed {
                    return None;
                }
                tokens.push(ExprToken::StringLiteral(value));
            }
            ch if is_kconfig_symbol_char(ch) => {
                let start = idx;
                idx += 1;
                while idx < chars.len() && is_kconfig_symbol_char(chars[idx]) {
                    idx += 1;
                }
                tokens.push(ExprToken::Symbol(chars[start..idx].iter().collect()));
            }
            _ => return None,
        }
    }

    (!tokens.is_empty()).then_some(tokens)
}

fn parse_kconfig_or_expr(tokens: &[ExprToken], idx: &mut usize) -> Option<KconfigExpr> {
    let mut expr = parse_kconfig_and_expr(tokens, idx)?;
    while matches!(tokens.get(*idx), Some(ExprToken::Or)) {
        *idx += 1;
        let rhs = parse_kconfig_and_expr(tokens, idx)?;
        expr = KconfigExpr::Or(Box::new(expr), Box::new(rhs));
    }
    Some(expr)
}

fn parse_kconfig_and_expr(tokens: &[ExprToken], idx: &mut usize) -> Option<KconfigExpr> {
    let mut expr = parse_kconfig_cmp_expr(tokens, idx)?;
    while matches!(tokens.get(*idx), Some(ExprToken::And)) {
        *idx += 1;
        let rhs = parse_kconfig_cmp_expr(tokens, idx)?;
        expr = KconfigExpr::And(Box::new(expr), Box::new(rhs));
    }
    Some(expr)
}

fn parse_kconfig_cmp_expr(tokens: &[ExprToken], idx: &mut usize) -> Option<KconfigExpr> {
    let mut expr = parse_kconfig_unary_expr(tokens, idx)?;
    loop {
        match tokens.get(*idx) {
            Some(ExprToken::Eq) => {
                *idx += 1;
                let rhs = parse_kconfig_unary_expr(tokens, idx)?;
                expr = KconfigExpr::Eq(Box::new(expr), Box::new(rhs));
            }
            Some(ExprToken::Ne) => {
                *idx += 1;
                let rhs = parse_kconfig_unary_expr(tokens, idx)?;
                expr = KconfigExpr::Ne(Box::new(expr), Box::new(rhs));
            }
            _ => return Some(expr),
        }
    }
}

fn parse_kconfig_unary_expr(tokens: &[ExprToken], idx: &mut usize) -> Option<KconfigExpr> {
    if matches!(tokens.get(*idx), Some(ExprToken::Not)) {
        *idx += 1;
        return Some(KconfigExpr::Not(Box::new(parse_kconfig_unary_expr(
            tokens, idx,
        )?)));
    }
    parse_kconfig_primary_expr(tokens, idx)
}

fn parse_kconfig_primary_expr(tokens: &[ExprToken], idx: &mut usize) -> Option<KconfigExpr> {
    match tokens.get(*idx)? {
        ExprToken::Symbol(symbol) => {
            *idx += 1;
            Some(match symbol.as_str() {
                "y" => KconfigExpr::Literal(TristateLiteral::Y),
                "m" => KconfigExpr::Literal(TristateLiteral::M),
                "n" => KconfigExpr::Literal(TristateLiteral::N),
                _ => KconfigExpr::Symbol(symbol.clone()),
            })
        }
        ExprToken::StringLiteral(value) => {
            *idx += 1;
            Some(KconfigExpr::StringLiteral(value.clone()))
        }
        ExprToken::LParen => {
            *idx += 1;
            let expr = parse_kconfig_or_expr(tokens, idx)?;
            match tokens.get(*idx) {
                Some(ExprToken::RParen) => {
                    *idx += 1;
                    Some(expr)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn string_literals_are_comparison_operands(expr: &KconfigExpr) -> bool {
    match expr {
        KconfigExpr::Symbol(_) | KconfigExpr::Literal(_) => true,
        KconfigExpr::StringLiteral(_) => false,
        KconfigExpr::Not(inner) => string_literals_are_comparison_operands(inner),
        KconfigExpr::And(lhs, rhs) | KconfigExpr::Or(lhs, rhs) => {
            string_literals_are_comparison_operands(lhs)
                && string_literals_are_comparison_operands(rhs)
        }
        KconfigExpr::Eq(lhs, rhs) | KconfigExpr::Ne(lhs, rhs) => {
            comparison_operand_allows_string_literal(lhs)
                && comparison_operand_allows_string_literal(rhs)
        }
    }
}

fn comparison_operand_allows_string_literal(expr: &KconfigExpr) -> bool {
    match expr {
        KconfigExpr::Symbol(_) | KconfigExpr::Literal(_) | KconfigExpr::StringLiteral(_) => true,
        other => {
            !expr_contains_string_literal(other) && string_literals_are_comparison_operands(other)
        }
    }
}

fn expr_contains_string_literal(expr: &KconfigExpr) -> bool {
    match expr {
        KconfigExpr::StringLiteral(_) => true,
        KconfigExpr::Symbol(_) | KconfigExpr::Literal(_) => false,
        KconfigExpr::Not(inner) => expr_contains_string_literal(inner),
        KconfigExpr::And(lhs, rhs)
        | KconfigExpr::Or(lhs, rhs)
        | KconfigExpr::Eq(lhs, rhs)
        | KconfigExpr::Ne(lhs, rhs) => {
            expr_contains_string_literal(lhs) || expr_contains_string_literal(rhs)
        }
    }
}

pub(super) fn simplify_kconfig_expr(expr: &KconfigExpr, removed: &HashSet<&str>) -> KconfigExpr {
    match expr {
        KconfigExpr::Symbol(symbol) => {
            if removed.contains(symbol.as_str()) {
                KconfigExpr::Literal(TristateLiteral::N)
            } else {
                KconfigExpr::Symbol(symbol.clone())
            }
        }
        KconfigExpr::Literal(lit) => KconfigExpr::Literal(*lit),
        KconfigExpr::StringLiteral(value) => KconfigExpr::StringLiteral(value.clone()),
        KconfigExpr::Not(inner) => match simplify_kconfig_expr(inner, removed) {
            KconfigExpr::Literal(TristateLiteral::Y) => KconfigExpr::Literal(TristateLiteral::N),
            KconfigExpr::Literal(TristateLiteral::N) => KconfigExpr::Literal(TristateLiteral::Y),
            KconfigExpr::Literal(TristateLiteral::M) => KconfigExpr::Literal(TristateLiteral::M),
            other => KconfigExpr::Not(Box::new(other)),
        },
        KconfigExpr::And(lhs, rhs) => {
            let lhs = simplify_kconfig_expr(lhs, removed);
            let rhs = simplify_kconfig_expr(rhs, removed);
            match (&lhs, &rhs) {
                (KconfigExpr::Literal(TristateLiteral::N), _)
                | (_, KconfigExpr::Literal(TristateLiteral::N)) => {
                    KconfigExpr::Literal(TristateLiteral::N)
                }
                (KconfigExpr::Literal(lhs), KconfigExpr::Literal(rhs)) => {
                    KconfigExpr::Literal(tristate_and(*lhs, *rhs))
                }
                (KconfigExpr::Literal(TristateLiteral::Y), _) => rhs,
                (_, KconfigExpr::Literal(TristateLiteral::Y)) => lhs,
                _ => KconfigExpr::And(Box::new(lhs), Box::new(rhs)),
            }
        }
        KconfigExpr::Or(lhs, rhs) => {
            let lhs = simplify_kconfig_expr(lhs, removed);
            let rhs = simplify_kconfig_expr(rhs, removed);
            match (&lhs, &rhs) {
                (KconfigExpr::Literal(TristateLiteral::Y), _)
                | (_, KconfigExpr::Literal(TristateLiteral::Y)) => {
                    KconfigExpr::Literal(TristateLiteral::Y)
                }
                (KconfigExpr::Literal(lhs), KconfigExpr::Literal(rhs)) => {
                    KconfigExpr::Literal(tristate_or(*lhs, *rhs))
                }
                (KconfigExpr::Literal(TristateLiteral::N), _) => rhs,
                (_, KconfigExpr::Literal(TristateLiteral::N)) => lhs,
                _ => KconfigExpr::Or(Box::new(lhs), Box::new(rhs)),
            }
        }
        KconfigExpr::Eq(lhs, rhs) => {
            let lhs = simplify_kconfig_expr(lhs, removed);
            let rhs = simplify_kconfig_expr(rhs, removed);
            match (kconfig_const_value(&lhs), kconfig_const_value(&rhs)) {
                (Some(lhs), Some(rhs)) => KconfigExpr::Literal(if kconfig_const_eq(&lhs, &rhs) {
                    TristateLiteral::Y
                } else {
                    TristateLiteral::N
                }),
                _ if lhs == rhs => KconfigExpr::Literal(TristateLiteral::Y),
                _ => KconfigExpr::Eq(Box::new(lhs), Box::new(rhs)),
            }
        }
        KconfigExpr::Ne(lhs, rhs) => {
            let lhs = simplify_kconfig_expr(lhs, removed);
            let rhs = simplify_kconfig_expr(rhs, removed);
            match (kconfig_const_value(&lhs), kconfig_const_value(&rhs)) {
                (Some(lhs), Some(rhs)) => KconfigExpr::Literal(if !kconfig_const_eq(&lhs, &rhs) {
                    TristateLiteral::Y
                } else {
                    TristateLiteral::N
                }),
                _ if lhs == rhs => KconfigExpr::Literal(TristateLiteral::N),
                _ => KconfigExpr::Ne(Box::new(lhs), Box::new(rhs)),
            }
        }
    }
}

pub(super) fn equivalent_kconfig_expr_simplification(
    expr: &KconfigExpr,
    removed: &HashSet<&str>,
) -> Option<KconfigExpr> {
    let simplified = simplify_kconfig_expr(expr, removed);
    kconfig_expr_rewrite_is_tristate_equivalent(expr, &simplified, removed).then_some(simplified)
}

pub(super) fn kconfig_expr_rewrite_is_tristate_equivalent(
    original: &KconfigExpr,
    rewritten: &KconfigExpr,
    removed: &HashSet<&str>,
) -> bool {
    let symbols = kconfig_expr_live_symbols(original, rewritten, removed);
    if symbols.len() > KCONFIG_EXPR_EQUIVALENCE_SYMBOL_LIMIT {
        return false;
    }

    let mut values = BTreeMap::new();
    kconfig_expr_rewrite_is_tristate_equivalent_for_assignments(
        original,
        rewritten,
        removed,
        &symbols,
        0,
        &mut values,
    )
}

fn kconfig_expr_rewrite_is_tristate_equivalent_for_assignments(
    original: &KconfigExpr,
    rewritten: &KconfigExpr,
    removed: &HashSet<&str>,
    symbols: &[String],
    idx: usize,
    values: &mut BTreeMap<String, TristateLiteral>,
) -> bool {
    if idx == symbols.len() {
        let original_value =
            evaluate_kconfig_expr_under_removed_tristate_semantics(original, values, removed);
        let rewritten_value =
            evaluate_kconfig_expr_under_removed_tristate_semantics(rewritten, values, removed);
        return original_value.is_some() && original_value == rewritten_value;
    }

    let symbol = symbols[idx].clone();
    for value in [
        TristateLiteral::N,
        TristateLiteral::M,
        TristateLiteral::Y,
    ] {
        values.insert(symbol.clone(), value);
        if !kconfig_expr_rewrite_is_tristate_equivalent_for_assignments(
            original,
            rewritten,
            removed,
            symbols,
            idx + 1,
            values,
        ) {
            values.remove(&symbol);
            return false;
        }
    }
    values.remove(&symbol);
    true
}

fn kconfig_expr_live_symbols(
    original: &KconfigExpr,
    rewritten: &KconfigExpr,
    removed: &HashSet<&str>,
) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    collect_kconfig_expr_live_symbols(original, removed, &mut symbols);
    collect_kconfig_expr_live_symbols(rewritten, removed, &mut symbols);
    symbols.into_iter().collect()
}

fn collect_kconfig_expr_live_symbols(
    expr: &KconfigExpr,
    removed: &HashSet<&str>,
    symbols: &mut BTreeSet<String>,
) {
    match expr {
        KconfigExpr::Symbol(symbol) => {
            if !removed.contains(symbol.as_str()) {
                symbols.insert(symbol.clone());
            }
        }
        KconfigExpr::Literal(_) | KconfigExpr::StringLiteral(_) => {}
        KconfigExpr::Not(inner) => collect_kconfig_expr_live_symbols(inner, removed, symbols),
        KconfigExpr::And(lhs, rhs)
        | KconfigExpr::Or(lhs, rhs)
        | KconfigExpr::Eq(lhs, rhs)
        | KconfigExpr::Ne(lhs, rhs) => {
            collect_kconfig_expr_live_symbols(lhs, removed, symbols);
            collect_kconfig_expr_live_symbols(rhs, removed, symbols);
        }
    }
}

pub(super) fn evaluate_kconfig_expr_under_removed_tristate_semantics(
    expr: &KconfigExpr,
    symbol_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<TristateLiteral> {
    match expr {
        KconfigExpr::Symbol(symbol) if removed_symbols.contains(symbol.as_str()) => {
            Some(TristateLiteral::N)
        }
        KconfigExpr::Symbol(symbol) => symbol_values.get(symbol).copied(),
        KconfigExpr::Literal(value) => Some(*value),
        KconfigExpr::StringLiteral(_) => None,
        KconfigExpr::Not(inner) => {
            evaluate_kconfig_expr_under_removed_tristate_semantics(
                inner,
                symbol_values,
                removed_symbols,
            )
            .map(tristate_not)
        }
        KconfigExpr::And(lhs, rhs) => Some(tristate_and(
            evaluate_kconfig_expr_under_removed_tristate_semantics(
                lhs,
                symbol_values,
                removed_symbols,
            )?,
            evaluate_kconfig_expr_under_removed_tristate_semantics(
                rhs,
                symbol_values,
                removed_symbols,
            )?,
        )),
        KconfigExpr::Or(lhs, rhs) => Some(tristate_or(
            evaluate_kconfig_expr_under_removed_tristate_semantics(
                lhs,
                symbol_values,
                removed_symbols,
            )?,
            evaluate_kconfig_expr_under_removed_tristate_semantics(
                rhs,
                symbol_values,
                removed_symbols,
            )?,
        )),
        KconfigExpr::Eq(lhs, rhs) => Some(if kconfig_const_eq(
            &evaluate_kconfig_const_under_removed_tristate_semantics(
                lhs,
                symbol_values,
                removed_symbols,
            )?,
            &evaluate_kconfig_const_under_removed_tristate_semantics(
                rhs,
                symbol_values,
                removed_symbols,
            )?,
        ) {
            TristateLiteral::Y
        } else {
            TristateLiteral::N
        }),
        KconfigExpr::Ne(lhs, rhs) => Some(if !kconfig_const_eq(
            &evaluate_kconfig_const_under_removed_tristate_semantics(
                lhs,
                symbol_values,
                removed_symbols,
            )?,
            &evaluate_kconfig_const_under_removed_tristate_semantics(
                rhs,
                symbol_values,
                removed_symbols,
            )?,
        ) {
            TristateLiteral::Y
        } else {
            TristateLiteral::N
        }),
    }
}

fn evaluate_kconfig_const_under_removed_tristate_semantics(
    expr: &KconfigExpr,
    symbol_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<KconfigConst> {
    match expr {
        KconfigExpr::Literal(value) => Some(KconfigConst::Tristate(*value)),
        KconfigExpr::StringLiteral(value) => Some(KconfigConst::String(value.clone())),
        KconfigExpr::Symbol(symbol) if removed_symbols.contains(symbol.as_str()) => {
            Some(KconfigConst::Tristate(TristateLiteral::N))
        }
        KconfigExpr::Symbol(symbol) => symbol_values
            .get(symbol)
            .copied()
            .map(KconfigConst::Tristate),
        other => evaluate_kconfig_expr_under_removed_tristate_semantics(
            other,
            symbol_values,
            removed_symbols,
        )
        .map(KconfigConst::Tristate),
    }
}

#[allow(dead_code)]
pub(super) fn evaluate_kconfig_expr_after_removed_symbols(
    expr: &KconfigExpr,
    symbol_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<TristateLiteral> {
    let simplified = simplify_kconfig_expr(expr, removed_symbols);
    evaluate_kconfig_expr(&simplified, symbol_values)
}

#[allow(dead_code)]
pub(super) fn evaluate_kconfig_expr(
    expr: &KconfigExpr,
    symbol_values: &BTreeMap<String, TristateLiteral>,
) -> Option<TristateLiteral> {
    match expr {
        KconfigExpr::Symbol(symbol) => symbol_values.get(symbol).copied(),
        KconfigExpr::Literal(value) => Some(*value),
        KconfigExpr::StringLiteral(_) => None,
        KconfigExpr::Not(inner) => evaluate_kconfig_expr(inner, symbol_values).map(tristate_not),
        KconfigExpr::And(lhs, rhs) => Some(tristate_and(
            evaluate_kconfig_expr(lhs, symbol_values)?,
            evaluate_kconfig_expr(rhs, symbol_values)?,
        )),
        KconfigExpr::Or(lhs, rhs) => Some(tristate_or(
            evaluate_kconfig_expr(lhs, symbol_values)?,
            evaluate_kconfig_expr(rhs, symbol_values)?,
        )),
        KconfigExpr::Eq(lhs, rhs) => Some(if kconfig_const_eq(
            &evaluate_kconfig_const(lhs, symbol_values)?,
            &evaluate_kconfig_const(rhs, symbol_values)?,
        ) {
            TristateLiteral::Y
        } else {
            TristateLiteral::N
        }),
        KconfigExpr::Ne(lhs, rhs) => Some(if !kconfig_const_eq(
            &evaluate_kconfig_const(lhs, symbol_values)?,
            &evaluate_kconfig_const(rhs, symbol_values)?,
        ) {
            TristateLiteral::Y
        } else {
            TristateLiteral::N
        }),
    }
}

#[allow(dead_code)]
fn evaluate_kconfig_const(
    expr: &KconfigExpr,
    symbol_values: &BTreeMap<String, TristateLiteral>,
) -> Option<KconfigConst> {
    match expr {
        KconfigExpr::Literal(value) => Some(KconfigConst::Tristate(*value)),
        KconfigExpr::StringLiteral(value) => Some(KconfigConst::String(value.clone())),
        KconfigExpr::Symbol(symbol) => symbol_values
            .get(symbol)
            .copied()
            .map(KconfigConst::Tristate),
        other => evaluate_kconfig_expr(other, symbol_values).map(KconfigConst::Tristate),
    }
}

fn kconfig_const_value(expr: &KconfigExpr) -> Option<KconfigConst> {
    match expr {
        KconfigExpr::Literal(value) => Some(KconfigConst::Tristate(*value)),
        KconfigExpr::StringLiteral(value) => Some(KconfigConst::String(value.clone())),
        _ => None,
    }
}

fn kconfig_const_eq(lhs: &KconfigConst, rhs: &KconfigConst) -> bool {
    match (lhs, rhs) {
        (KconfigConst::Tristate(lhs), KconfigConst::Tristate(rhs)) => lhs == rhs,
        (KconfigConst::String(lhs), KconfigConst::String(rhs)) => lhs == rhs,
        (KconfigConst::Tristate(lhs), KconfigConst::String(rhs))
        | (KconfigConst::String(rhs), KconfigConst::Tristate(lhs)) => {
            string_as_tristate(rhs).is_some_and(|rhs| rhs == *lhs)
        }
    }
}

fn string_as_tristate(value: &str) -> Option<TristateLiteral> {
    match value {
        "y" => Some(TristateLiteral::Y),
        "m" => Some(TristateLiteral::M),
        "n" => Some(TristateLiteral::N),
        _ => None,
    }
}

#[allow(dead_code)]
fn tristate_not(value: TristateLiteral) -> TristateLiteral {
    match value {
        TristateLiteral::Y => TristateLiteral::N,
        TristateLiteral::M => TristateLiteral::M,
        TristateLiteral::N => TristateLiteral::Y,
    }
}

pub(super) fn tristate_and(lhs: TristateLiteral, rhs: TristateLiteral) -> TristateLiteral {
    std::cmp::min_by_key(lhs, rhs, |value| tristate_rank(*value))
}

pub(super) fn tristate_or(lhs: TristateLiteral, rhs: TristateLiteral) -> TristateLiteral {
    std::cmp::max_by_key(lhs, rhs, |value| tristate_rank(*value))
}

fn tristate_rank(value: TristateLiteral) -> u8 {
    match value {
        TristateLiteral::N => 0,
        TristateLiteral::M => 1,
        TristateLiteral::Y => 2,
    }
}

pub(super) fn first_removed_symbol(expr: &KconfigExpr, removed: &HashSet<&str>) -> Option<String> {
    let mut symbols = BTreeSet::new();
    collect_removed_symbols(expr, removed, &mut symbols);
    symbols.into_iter().next()
}

fn collect_removed_symbols(
    expr: &KconfigExpr,
    removed: &HashSet<&str>,
    symbols: &mut BTreeSet<String>,
) {
    match expr {
        KconfigExpr::Symbol(symbol) => {
            if removed.contains(symbol.as_str()) {
                symbols.insert(symbol.clone());
            }
        }
        KconfigExpr::Literal(_) | KconfigExpr::StringLiteral(_) => {}
        KconfigExpr::Not(inner) => collect_removed_symbols(inner, removed, symbols),
        KconfigExpr::And(lhs, rhs)
        | KconfigExpr::Or(lhs, rhs)
        | KconfigExpr::Eq(lhs, rhs)
        | KconfigExpr::Ne(lhs, rhs) => {
            collect_removed_symbols(lhs, removed, symbols);
            collect_removed_symbols(rhs, removed, symbols);
        }
    }
}

pub(super) fn render_kconfig_expr(expr: &KconfigExpr) -> String {
    render_kconfig_expr_with_prec(expr, 0)
}

fn render_kconfig_expr_with_prec(expr: &KconfigExpr, parent_prec: u8) -> String {
    let current_prec = kconfig_expr_precedence(expr);
    let rendered = match expr {
        KconfigExpr::Symbol(symbol) => symbol.clone(),
        KconfigExpr::Literal(TristateLiteral::Y) => String::from("y"),
        KconfigExpr::Literal(TristateLiteral::M) => String::from("m"),
        KconfigExpr::Literal(TristateLiteral::N) => String::from("n"),
        KconfigExpr::StringLiteral(value) => quote_kconfig_string(value),
        KconfigExpr::Not(inner) => {
            format!("!{}", render_kconfig_expr_with_prec(inner, current_prec))
        }
        KconfigExpr::And(lhs, rhs) => format!(
            "{} && {}",
            render_kconfig_expr_with_prec(lhs, current_prec),
            render_kconfig_expr_with_prec(rhs, current_prec)
        ),
        KconfigExpr::Or(lhs, rhs) => format!(
            "{} || {}",
            render_kconfig_expr_with_prec(lhs, current_prec),
            render_kconfig_expr_with_prec(rhs, current_prec)
        ),
        KconfigExpr::Eq(lhs, rhs) => format!(
            "{} = {}",
            render_kconfig_expr_with_prec(lhs, current_prec),
            render_kconfig_expr_with_prec(rhs, current_prec)
        ),
        KconfigExpr::Ne(lhs, rhs) => format!(
            "{} != {}",
            render_kconfig_expr_with_prec(lhs, current_prec),
            render_kconfig_expr_with_prec(rhs, current_prec)
        ),
    };

    if current_prec < parent_prec {
        format!("({rendered})")
    } else {
        rendered
    }
}

fn kconfig_expr_precedence(expr: &KconfigExpr) -> u8 {
    match expr {
        KconfigExpr::Or(_, _) => 1,
        KconfigExpr::And(_, _) => 2,
        KconfigExpr::Eq(_, _) | KconfigExpr::Ne(_, _) => 3,
        KconfigExpr::Not(_) => 4,
        KconfigExpr::Symbol(_) | KconfigExpr::Literal(_) | KconfigExpr::StringLiteral(_) => 5,
    }
}

fn quote_kconfig_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

pub(super) fn is_kconfig_symbol_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
