use anyhow::{Context, Result};

use crate::model::KconfigSymbol;

use super::super::{indentation, split_kconfig_if_clause, split_kconfig_trailing_comment};
use super::{is_kconfig_help_block_directive, parse_quoted_string_literal, KconfigRawLine};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KconfigSymbolType {
    Bool,
    Tristate,
    String,
    Int,
    Hex,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigTypeDefinition {
    kind: KconfigSymbolType,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigTypeDefinition {
    pub(crate) fn kind(&self) -> KconfigSymbolType {
        self.kind
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigPromptDefinition {
    prompt: String,
    condition: Option<String>,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigPromptDefinition {
    pub(crate) fn prompt(&self) -> &str {
        &self.prompt
    }

    pub(crate) fn condition(&self) -> Option<&str> {
        self.condition.as_deref()
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigDefaultDefinition {
    value: String,
    condition: Option<String>,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigDefaultDefinition {
    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    pub(crate) fn condition(&self) -> Option<&str> {
        self.condition.as_deref()
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigRangeDefinition {
    minimum: String,
    maximum: String,
    condition: Option<String>,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigRangeDefinition {
    pub(crate) fn minimum(&self) -> &str {
        &self.minimum
    }

    pub(crate) fn maximum(&self) -> &str {
        &self.maximum
    }

    pub(crate) fn condition(&self) -> Option<&str> {
        self.condition.as_deref()
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigDependencyDefinition {
    expression: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigDependencyDefinition {
    pub(crate) fn expression(&self) -> &str {
        &self.expression
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigSelectDefinition {
    target: KconfigSymbol,
    condition: Option<String>,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigSelectDefinition {
    pub(crate) fn target(&self) -> &KconfigSymbol {
        &self.target
    }

    pub(crate) fn condition(&self) -> Option<&str> {
        self.condition.as_deref()
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigImplyDefinition {
    target: KconfigSymbol,
    condition: Option<String>,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigImplyDefinition {
    pub(crate) fn target(&self) -> &KconfigSymbol {
        &self.target
    }

    pub(crate) fn condition(&self) -> Option<&str> {
        self.condition.as_deref()
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigOptionDefinition {
    name: String,
    value: Option<String>,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigOptionDefinition {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigModulesDefinition {
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigModulesDefinition {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

pub(super) fn parse_type_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigTypeDefinition>> {
    parse_body_definitions(body, |line| Ok(parse_type_definition(line)))
}

fn parse_type_definition(line: &KconfigRawLine) -> Option<KconfigTypeDefinition> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    let kind = if is_kconfig_bool_type_line(trimmed) {
        KconfigSymbolType::Bool
    } else if is_kconfig_tristate_type_line(trimmed) {
        KconfigSymbolType::Tristate
    } else if is_kconfig_string_type_line(trimmed) {
        KconfigSymbolType::String
    } else if is_kconfig_int_type_line(trimmed) {
        KconfigSymbolType::Int
    } else if is_kconfig_hex_type_line(trimmed) {
        KconfigSymbolType::Hex
    } else {
        return None;
    };

    Some(KconfigTypeDefinition {
        kind,
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    })
}

pub(super) fn parse_prompt_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigPromptDefinition>> {
    parse_body_definitions(body, parse_prompt_definition)
}

fn parse_prompt_definition(line: &KconfigRawLine) -> Result<Option<KconfigPromptDefinition>> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    let Some((prompt, condition)) = parse_prompt_payload(trimmed, line.line())? else {
        return Ok(None);
    };

    Ok(Some(KconfigPromptDefinition {
        prompt,
        condition,
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    }))
}

pub(super) fn parse_default_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigDefaultDefinition>> {
    parse_body_definitions(body, parse_default_definition)
}

fn parse_default_definition(line: &KconfigRawLine) -> Result<Option<KconfigDefaultDefinition>> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    let Some(rest) = trimmed.strip_prefix("default") else {
        return Ok(None);
    };
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let (value, condition) = split_kconfig_if_clause(rest.trim_start());
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    Ok(Some(KconfigDefaultDefinition {
        value: value.to_string(),
        condition: condition
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .map(str::to_string),
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    }))
}

pub(super) fn parse_range_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigRangeDefinition>> {
    parse_body_definitions(body, parse_range_definition)
}

fn parse_range_definition(line: &KconfigRawLine) -> Result<Option<KconfigRangeDefinition>> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    let Some(rest) = trimmed.strip_prefix("range") else {
        return Ok(None);
    };
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let (bounds, condition) = split_kconfig_if_clause(rest.trim_start());
    let mut bounds = bounds.split_whitespace();
    let Some(minimum) = bounds.next() else {
        return Ok(None);
    };
    let Some(maximum) = bounds.next() else {
        return Ok(None);
    };
    if bounds.next().is_some() {
        return Ok(None);
    }

    Ok(Some(KconfigRangeDefinition {
        minimum: minimum.to_string(),
        maximum: maximum.to_string(),
        condition: condition
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .map(str::to_string),
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    }))
}

pub(super) fn parse_dependency_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigDependencyDefinition>> {
    parse_body_definitions(body, parse_dependency_definition)
}

fn parse_dependency_definition(
    line: &KconfigRawLine,
) -> Result<Option<KconfigDependencyDefinition>> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    let Some(rest) = trimmed.strip_prefix("depends") else {
        return Ok(None);
    };
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix("on") else {
        return Ok(None);
    };
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let expression = rest.trim();
    if expression.is_empty() {
        return Ok(None);
    }

    Ok(Some(KconfigDependencyDefinition {
        expression: expression.to_string(),
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    }))
}

pub(super) fn parse_select_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigSelectDefinition>> {
    parse_body_definitions(body, parse_select_definition)
}

fn parse_select_definition(line: &KconfigRawLine) -> Result<Option<KconfigSelectDefinition>> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    let Some(rest) = trimmed.strip_prefix("select") else {
        return Ok(None);
    };
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let (target, condition) = split_kconfig_if_clause(rest.trim_start());
    let mut target_parts = target.split_whitespace();
    let Some(target) = target_parts.next() else {
        return Ok(None);
    };
    if target_parts.next().is_some() {
        return Ok(None);
    }

    Ok(Some(KconfigSelectDefinition {
        target: KconfigSymbol::new(target)
            .with_context(|| format!("invalid Kconfig select target on line {}", line.line()))?,
        condition: condition
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .map(str::to_string),
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    }))
}

pub(super) fn parse_imply_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigImplyDefinition>> {
    parse_body_definitions(body, parse_imply_definition)
}

fn parse_imply_definition(line: &KconfigRawLine) -> Result<Option<KconfigImplyDefinition>> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    let Some(rest) = trimmed.strip_prefix("imply") else {
        return Ok(None);
    };
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let (target, condition) = split_kconfig_if_clause(rest.trim_start());
    let mut target_parts = target.split_whitespace();
    let Some(target) = target_parts.next() else {
        return Ok(None);
    };
    if target_parts.next().is_some() {
        return Ok(None);
    }

    Ok(Some(KconfigImplyDefinition {
        target: KconfigSymbol::new(target)
            .with_context(|| format!("invalid Kconfig imply target on line {}", line.line()))?,
        condition: condition
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .map(str::to_string),
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    }))
}

pub(super) fn parse_option_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigOptionDefinition>> {
    parse_body_definitions(body, |line| Ok(parse_option_definition(line)))
}

fn parse_option_definition(line: &KconfigRawLine) -> Option<KconfigOptionDefinition> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    let Some(rest) = trimmed.strip_prefix("option") else {
        return None;
    };
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }

    let payload = rest.trim_start();
    let mut payload_parts = payload.split_whitespace();
    let Some(token) = payload_parts.next() else {
        return None;
    };
    if payload_parts.next().is_some() {
        return None;
    }

    let (name, value) = match token.split_once('=') {
        Some((name, value)) if !name.is_empty() && !value.is_empty() => {
            (name, Some(value.to_string()))
        }
        Some(_) => return None,
        None if !token.is_empty() => (token, None),
        None => return None,
    };

    Some(KconfigOptionDefinition {
        name: name.to_string(),
        value,
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    })
}

pub(super) fn parse_modules_definitions(
    body: &[KconfigRawLine],
) -> Result<Vec<KconfigModulesDefinition>> {
    parse_body_definitions(body, |line| Ok(parse_modules_definition(line)))
}

fn parse_modules_definition(line: &KconfigRawLine) -> Option<KconfigModulesDefinition> {
    let (directive_text, _) = split_kconfig_trailing_comment(line.text());
    let trimmed = directive_text.trim_start();
    if trimmed != "modules" {
        return None;
    }

    Some(KconfigModulesDefinition {
        line: line.line(),
        end_line: line.line(),
        directive: line.clone(),
    })
}

fn parse_body_definitions<T>(
    body: &[KconfigRawLine],
    mut parse: impl FnMut(&KconfigRawLine) -> Result<Option<T>>,
) -> Result<Vec<T>> {
    let mut definitions = Vec::new();
    let mut idx = 0usize;

    while idx < body.len() {
        let line = &body[idx];
        let trimmed = line.text().trim_start();
        if is_kconfig_help_block_directive(trimmed) {
            idx = next_body_line_after_help(body, idx);
            continue;
        }

        if let Some(definition) = parse(line)? {
            definitions.push(definition);
        }
        idx += 1;
    }

    Ok(definitions)
}

fn next_body_line_after_help(body: &[KconfigRawLine], mut idx: usize) -> usize {
    let help_indent = indentation(body[idx].text());
    idx += 1;
    while idx < body.len() {
        let help_line = body[idx].text();
        if help_line.trim().is_empty() || indentation(help_line) > help_indent {
            idx += 1;
            continue;
        }
        break;
    }
    idx
}

fn parse_prompt_payload(
    trimmed: &str,
    line_number: usize,
) -> Result<Option<(String, Option<String>)>> {
    if let Some(rest) = trimmed.strip_prefix("prompt") {
        if rest.starts_with(char::is_whitespace) {
            return parse_quoted_prompt_payload(rest.trim_start(), "prompt", line_number);
        }
        return Ok(None);
    }

    for keyword in ["bool", "tristate", "string", "int", "hex"] {
        let Some(rest) = trimmed.strip_prefix(keyword) else {
            continue;
        };
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            return parse_quoted_prompt_payload(rest.trim_start(), keyword, line_number);
        }
    }
    Ok(None)
}

fn parse_quoted_prompt_payload(
    input: &str,
    keyword: &str,
    line_number: usize,
) -> Result<Option<(String, Option<String>)>> {
    let Some((prompt, trailing)) =
        parse_quoted_string_literal(input, keyword, "prompt", line_number)?
    else {
        return Ok(None);
    };
    let (before_if, condition) = split_kconfig_if_clause(trailing);
    if !before_if.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some((
        prompt,
        condition
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .map(str::to_string),
    )))
}

fn is_kconfig_bool_type_line(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix("bool") else {
        return false;
    };
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

fn is_kconfig_tristate_type_line(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix("tristate") else {
        return false;
    };
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

fn is_kconfig_string_type_line(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix("string") else {
        return false;
    };
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

fn is_kconfig_int_type_line(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix("int") else {
        return false;
    };
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

fn is_kconfig_hex_type_line(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix("hex") else {
        return false;
    };
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}
