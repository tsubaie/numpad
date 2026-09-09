use crate::math::{self, Number as D};
use num_traits::{Signed, Zero};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Format {
    pub comma: bool,
    pub digits: u8,
}
impl Default for Format {
    fn default() -> Self {
        Self {
            comma: false,
            digits: 2,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Blank,
    Comment,
    Assignment,
    Value,
    Rule,
    Total,
}
#[derive(Debug, Clone)]
pub struct Line {
    pub raw: String,
    pub kind: Kind,
    pub op: char,
    pub operand: String,
    pub comment: String,
    pub name: String,
    pub percent: bool,
    pub value: Option<D>,
    pub amount: Option<D>,
    pub result: D,
    pub error: Option<String>,
    pub block: usize,
    pub count: usize,
}
impl Line {
    fn new(raw: &str) -> Self {
        Self {
            raw: raw.into(),
            kind: Kind::Comment,
            op: '+',
            operand: String::new(),
            comment: String::new(),
            name: String::new(),
            percent: false,
            value: None,
            amount: None,
            result: D::zero(),
            error: None,
            block: 0,
            count: 0,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Tape {
    pub lines: Vec<Line>,
    pub grand: D,
    pub variables: BTreeMap<String, (String, D)>,
}
pub fn valid_name(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}
pub fn is_rule(s: &str) -> bool {
    let s = s.trim();
    s.chars().count() >= 3 && s.chars().all(|c| matches!(c, '-' | '─' | '━' | '='))
}
pub fn parse(raw: &str, f: &Format, total: bool) -> Line {
    let mut l = Line::new(raw);
    let s = raw.trim();
    if s.is_empty() && !total {
        l.kind = Kind::Blank;
        return l;
    }
    if is_rule(s) {
        l.kind = Kind::Rule;
        return l;
    }
    if total || s.starts_with('=') {
        l.kind = Kind::Total;
        if let Some((_, rest)) = s.split_once('=') {
            let mut pieces = rest.trim().splitn(2, char::is_whitespace);
            l.name = pieces.next().unwrap_or("").into();
            l.comment = pieces.next().unwrap_or("").trim().into();
            if !l.name.is_empty() && !valid_name(&l.name) {
                l.error = Some("Invalid variable name".into());
            }
        } else {
            let (_, _, rest) = read_operand(s, f);
            l.comment = rest.trim().into();
        }
        return l;
    }
    let start = s.chars().next().unwrap();
    if let Some((left, right)) = s.split_once('=')
        && (start.is_alphabetic() || start == '_')
    {
        l.kind = Kind::Assignment;
        l.name = left.trim().into();
        l.operand = right.trim().into();
        if !valid_name(&l.name) {
            l.error = Some("Invalid variable name".into());
        }
        return l;
    }
    if !start.is_ascii_digit()
        && !matches!(
            start,
            '+' | '-' | '−' | '*' | '×' | '/' | '÷' | '^' | '.' | ','
        )
    {
        l.comment = raw.into();
        return l;
    }
    l.kind = Kind::Value;
    let (op, operand, rest) = read_operand(s, f);
    l.op = op;
    l.operand = operand;
    let mut tail = rest.trim_start();
    if tail.starts_with('%') {
        l.percent = true;
        tail = tail[1..].trim_start();
    }
    if tail.starts_with('|') {
        tail = tail[1..].trim_start();
        let end = tail.find(char::is_whitespace).unwrap_or(tail.len());
        tail = tail[end..].trim_start();
    }
    l.comment = tail.into();
    if l.operand.is_empty() {
        l.error = Some(
            if tail.starts_with(['+', '-', '*', '/', '^']) {
                "Too many operators"
            } else {
                "Value undefined"
            }
            .into(),
        );
    }
    if l.comment.starts_with('=') {
        l.error = Some("Variable definitions only in sum lines".into());
    }
    l
}
fn read_operand<'a>(s: &'a str, f: &Format) -> (char, String, &'a str) {
    let mut body = s.trim_start();
    let mut op = '+';
    if let Some(c) = body.chars().next()
        && matches!(c, '+' | '-' | '−' | '*' | '×' | '/' | '÷' | '^')
    {
        op = match c {
            '−' => '-',
            '×' => '*',
            '÷' => '/',
            _ => c,
        };
        body = body[c.len_utf8()..].trim_start();
    }
    let mut end = 0;
    let mut seen_decimal = false;
    let numeric = body.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '.' | ',' | '-'));
    for (i, c) in body.char_indices() {
        let allowed = if numeric {
            if c == if f.comma { ',' } else { '.' } {
                if seen_decimal {
                    false
                } else {
                    seen_decimal = true;
                    true
                }
            } else {
                c.is_ascii_digit()
                    || (c == if f.comma { '.' } else { ',' } && !seen_decimal)
                    || (i == 0 && c == '-')
            }
        } else {
            c.is_ascii_alphanumeric()
        };
        if !allowed {
            break;
        }
        end = i + c.len_utf8();
    }
    (op, body[..end].into(), &body[end..])
}
pub fn number(s: &str, f: &Format) -> Result<D, String> {
    if s.len() > 128 {
        return Err("Number is too long".into());
    }
    let normalized = if f.comma {
        s.replace('.', "").replace(',', ".")
    } else {
        s.replace(',', "")
    };
    D::from_str(&normalized)
        .map_err(|_| "Invalid number".into())
        .and_then(math::checked)
}
fn resolve(s: &str, vars: &BTreeMap<String, (String, D)>, f: &Format) -> Result<D, String> {
    if s.starts_with(|c: char| c.is_ascii_alphabetic()) {
        vars.get(&s.to_ascii_lowercase())
            .map(|(_, v)| v.clone())
            .ok_or_else(|| format!("Variable not defined: {s}"))
    } else {
        number(s, f)
    }
}
fn priority(c: char) -> u8 {
    match c {
        '+' | '-' => 1,
        '*' | '/' => 2,
        '^' => 3,
        _ => 0,
    }
}
pub fn calculate(text: &str, f: &Format) -> Tape {
    let rows: Vec<&str> = text.split('\n').collect();
    let mut lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .map(|(i, s)| parse(s, f, i > 0 && is_rule(rows[i - 1])))
        .collect();
    let mut vars: BTreeMap<String, (String, D)> = BTreeMap::new();
    let mut finals: BTreeMap<usize, D> = BTreeMap::new();
    let mut block = 0;
    let mut running = D::zero();
    let mut count = 0;
    let mut failed = false;
    let mut reset = false;
    for i in 0..lines.len() {
        if matches!(lines[i].kind, Kind::Blank | Kind::Comment) {
            lines[i].result =
                if lines[i].kind == Kind::Blank && i > 0 && lines[i - 1].kind == Kind::Total {
                    running.clone()
                } else {
                    D::zero()
                };
            lines[i].block = block;
            lines[i].count = 0;
            reset = true;
            continue;
        }
        if lines[i].kind == Kind::Rule {
            lines[i].result = running.clone();
            lines[i].block = block;
            lines[i].count = count;
            continue;
        }
        if reset {
            block += 1;
            running = D::zero();
            count = 0;
            failed = false;
            reset = false;
        }
        let mut l = lines[i].clone();
        l.block = block;
        let outcome = (|| -> Result<(), String> {
            if let Some(e) = &l.error {
                return Err(e.clone());
            }
            match l.kind {
                Kind::Assignment => {
                    let key = l.name.to_ascii_lowercase();
                    if vars.contains_key(&key) {
                        return Err(format!("Variable name already in use: {}", l.name));
                    }
                    let value = expression(&l.operand, &vars, f)?;
                    l.value = Some(value.clone());
                    vars.insert(key, (l.name.clone(), value));
                }
                Kind::Total => {
                    if count == 0 {
                        return Err("No calculation above this total".into());
                    }
                    if failed {
                        return Err("Invalid calculation".into());
                    }
                    l.value = Some(running.clone());
                    if !l.name.is_empty() {
                        let key = l.name.to_ascii_lowercase();
                        if vars.contains_key(&key) {
                            return Err(format!("Variable name already in use: {}", l.name));
                        }
                        vars.insert(key, (l.name.clone(), running.clone()));
                    }
                    finals.insert(block, running.clone());
                    count = 1;
                }
                Kind::Value => {
                    if count == 0 && !matches!(l.op, '+' | '-') {
                        return Err("Calculations must start with a number or a sign".into());
                    }
                    let v = resolve(&l.operand, &vars, f)?;
                    if let Some((name, _)) = vars.get(&l.operand.to_ascii_lowercase()) {
                        l.operand = name.clone();
                    }
                    l.value = Some(v.clone());
                    let actual = if l.percent {
                        if count == 0 {
                            return Err("Percentage not at the start".into());
                        }
                        let factor = math::div(&v, &D::from(100))?;
                        let amount = if priority(l.op) == 1 {
                            math::rounded(&running * &factor)
                        } else {
                            factor
                        };
                        l.amount = Some(if l.op == '-' {
                            -&amount
                        } else {
                            amount.clone()
                        });
                        amount
                    } else {
                        v
                    };
                    count += 1;
                    running = math::binary(&running, &actual, l.op)?;
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(e) = outcome {
            l.error = Some(e);
            if l.kind != Kind::Assignment {
                failed = true;
            }
            if l.kind == Kind::Total {
                l.value = Some(D::zero());
            }
        }
        l.result = if failed { D::zero() } else { running.clone() };
        l.count = count;
        lines[i] = l;
    }
    // Display the completed block result even while the caret is on an earlier operand.
    let mut block_results = BTreeMap::new();
    for l in &lines {
        if matches!(l.kind, Kind::Value | Kind::Total) {
            block_results.insert(l.block, l.result.clone());
        }
    }
    for l in &mut lines {
        if matches!(l.kind, Kind::Value | Kind::Total | Kind::Rule)
            && let Some(v) = block_results.get(&l.block)
        {
            l.result = v.clone();
        }
    }
    Tape {
        lines,
        grand: finals.values().fold(D::zero(), |s, v| s + v),
        variables: vars,
    }
}
pub fn format(v: &D, f: &Format) -> String {
    let s = v
        .with_scale_round(
            i64::from(f.digits.min(12)),
            bigdecimal::RoundingMode::HalfUp,
        )
        .to_plain_string();
    let (integer, fraction) = s.split_once('.').unwrap_or((&s, ""));
    let negative = integer.starts_with('-');
    let digits = integer.trim_start_matches('-');
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        let remaining = digits.len() - i;
        if i > 0 && remaining % 3 == 0 {
            out.push(if f.comma { '.' } else { ',' });
        }
        out.push(c);
    }
    if negative && !v.is_zero() {
        out.insert(0, '-');
    }
    if f.digits > 0 {
        out.push(if f.comma { ',' } else { '.' });
        out.push_str(fraction);
    }
    out
}
pub fn render(l: &Line, f: &Format) -> String {
    match l.kind {
        Kind::Total => {
            let value = l.value.clone().unwrap_or_default();
            let sign = if value.is_negative() { '-' } else { '+' };
            format!(
                " {sign} {:>14}{}{}",
                format(&value.abs(), f),
                if l.name.is_empty() && !l.raw.contains('=') {
                    String::new()
                } else {
                    format!(" = {}", l.name)
                },
                if l.comment.is_empty() {
                    String::new()
                } else {
                    format!(" {}", l.comment)
                }
            )
        }
        Kind::Rule => " -----------------".into(),
        Kind::Value if l.value.is_some() && l.error.is_none() => {
            let token = if l.operand.starts_with(|c: char| c.is_ascii_alphabetic()) {
                l.operand.clone()
            } else {
                let value = l.value.as_ref().unwrap();
                let precision = value.normalized().as_bigint_and_exponent().1.clamp(0, 128) as u8;
                let operand_format = Format {
                    digits: f.digits.max(precision),
                    ..f.clone()
                };
                format(value, &operand_format)
            };
            let annotation = if let Some(a) = &l.amount {
                format!(
                    " | {}{}",
                    if matches!(l.op, '*' | '/') {
                        l.op.to_string()
                    } else {
                        String::new()
                    },
                    format(a, f)
                )
            } else {
                String::new()
            };
            format!(
                " {} {:>14}{}{}{}",
                l.op,
                token,
                if l.percent { "%" } else { " " },
                annotation,
                if l.comment.is_empty() {
                    String::new()
                } else {
                    format!("  {}", l.comment)
                }
            )
        }
        _ => l.raw.clone(),
    }
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
    depth: usize,
    vars: &'a BTreeMap<String, (String, D)>,
    f: &'a Format,
}
pub fn expression(s: &str, vars: &BTreeMap<String, (String, D)>, f: &Format) -> Result<D, String> {
    if s.len() > 2048 {
        return Err("Expression is too long".into());
    }
    let mut p = Parser {
        s,
        pos: 0,
        depth: 0,
        vars,
        f,
    };
    let v = p.expr(1)?;
    p.space();
    if p.pos < s.len() {
        return Err("Syntax error in assignment".into());
    }
    math::checked(v)
}
impl Parser<'_> {
    fn space(&mut self) {
        while self.s[self.pos..].starts_with(char::is_whitespace) {
            self.pos += self.s[self.pos..].chars().next().unwrap().len_utf8();
        }
    }
    fn expr(&mut self, min: u8) -> Result<D, String> {
        self.depth += 1;
        if self.depth > 64 {
            return Err("Expression nesting is too deep".into());
        }
        let mut lhs = self.atom()?;
        loop {
            self.space();
            let Some(op) = self.s[self.pos..].chars().next() else {
                break;
            };
            let precedence = priority(op);
            if precedence < min {
                break;
            }
            self.pos += op.len_utf8();
            let rhs = self.expr(precedence + 1)?;
            lhs = math::binary(&lhs, &rhs, op)?;
        }
        self.depth -= 1;
        Ok(lhs)
    }
    fn atom(&mut self) -> Result<D, String> {
        self.space();
        let Some(c) = self.s[self.pos..].chars().next() else {
            return Err("Value undefined".into());
        };
        if c == '+' || c == '-' {
            self.pos += 1;
            let v = self.atom()?;
            return Ok(if c == '-' { -v } else { v });
        }
        if c == '(' {
            self.pos += 1;
            let v = self.expr(1)?;
            self.space();
            if !self.s[self.pos..].starts_with(')') {
                return Err("Unclosed parenthesis".into());
            }
            self.pos += 1;
            return Ok(v);
        }
        let start = self.pos;
        if c.is_ascii_alphabetic() {
            while self.s[self.pos..].starts_with(|x: char| x.is_ascii_alphanumeric()) {
                self.pos += 1;
            }
            let name = &self.s[start..self.pos];
            self.space();
            if self.s[self.pos..].starts_with('(') {
                self.pos += 1;
                let v = self.expr(1)?;
                self.space();
                if !self.s[self.pos..].starts_with(')') {
                    return Err("Unclosed function argument".into());
                }
                self.pos += 1;
                return math::function(name, &v);
            }
            return resolve(name, self.vars, self.f);
        }
        while self.s[self.pos..].starts_with(|x: char| x.is_ascii_digit() || matches!(x, '.' | ','))
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err("Syntax error in assignment".into());
        }
        number(&self.s[start..self.pos], self.f)
    }
}
