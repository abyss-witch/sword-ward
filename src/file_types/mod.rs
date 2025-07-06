pub mod script;
pub mod scenes;
use std::{str::FromStr, num::ParseFloatError};
use crate::PlayerData;
use thin_engine::prelude::*;
#[derive(Debug)]
pub enum ParseErr {
    EarlyCloseBracket,
    NoCloseBracket,
    NoOpenBracket,
    NotEnoughArgs,
    ToManyArgs,
    InvalidReqOps,
    InvalidPrefix(String),
    InvalidColliderType(String),
    InvalidNumber(ParseFloatError),
    IoError(std::io::Error),
    InvalidComparison,
    InvalidColour,
}
impl From<std::io::Error>  for ParseErr { fn from(e: std::io::Error)  -> Self {  Self::IoError(e)       } }
impl From<ParseFloatError> for ParseErr { fn from(e: ParseFloatError) -> Self {  Self::InvalidNumber(e) } }
fn split_bracket(s: &str) -> Result<(String, usize), ParseErr> {
    if debug_parse() { println!("splitting bracket: {s}") }
    let mut indent = 1;
    for (e, c) in s.chars().enumerate() {
        indent += match c { '[' =>  1, ']' => -1, _ => continue };
        if indent == 0 { return Ok((s[..e].to_string(), e + 1)) }
        if indent <  0 { return Err(ParseErr::EarlyCloseBracket) }
    }
    Err(ParseErr::NoCloseBracket)
}
fn split_args(s: &str) -> Result<Vec<String>, ParseErr> {
    if debug_parse() { println!("splitting args: {s}") }
    let mut result = Vec::new();
    let mut current = String::new();
    let mut indent = 0;
    let mut control_char = false;
    for c in s.chars() {
        if !control_char && c == '[' { indent += 1 }
        if !control_char && c == ']' { indent -= 1 }
        if indent < 0 { return Err(ParseErr::EarlyCloseBracket) }

        if indent == 0 && (c == '\n' || c == ',') && !control_char && !current.trim().is_empty()  {
            result.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(c);
        }
        control_char = !control_char && c == '\\';
    }
    if !current.trim().is_empty() { result.push(current.trim().to_string()) }
    Ok(result)
}
fn parse_colour(s: &str) -> Result<Vec3, ParseErr> {
    if debug_parse() { println!("parsing colour: {s}") }
    let (prefix, brackets) = s.split_once('[').ok_or(ParseErr::NoOpenBracket)?;
    if prefix != "colour" { return Err(ParseErr::InvalidPrefix(prefix.to_string())) }
    let (inner, end) = split_bracket(brackets)?;
    if !brackets[end..].trim().is_empty() { return Err(ParseErr::EarlyCloseBracket) }
    match split_args(&inner)?.as_slice() {
        [r, g, b] => Ok(vec3(
            r.parse::<f32>()? / 255.0,
            g.parse::<f32>()? / 255.0,
            b.parse::<f32>()? / 255.0
        )),
        [_, _, _, _, ..] => Err(ParseErr::ToManyArgs),
        _ => Err(ParseErr::NotEnoughArgs)
    }
}
use std::env::vars;
fn is_true(val: &str)    -> bool { matches!(val.to_lowercase().trim(), "true" | "t" | "1" | "yes" | "y")  }
pub fn debug_parse()     -> bool { vars().any(|(key, val)| { key == "DEBUG_PARSING"   && is_true(&val) }) }
pub fn debug_colliders() -> bool { vars().any(|(key, val)| { key == "DEBUG_COLLIDERS" && is_true(&val) }) }
pub fn debug_lights()    -> bool { vars().any(|(key, val)| { key == "DEBUG_LIGHTS"    && is_true(&val) }) }


#[derive(Debug, Clone, PartialEq)]
pub enum CompVal {
    Recovery,
        Focus,
        Reasoning,
    Fitness,
        Strength,
        Speed,
    Charisma,
        Expression,
        Deception,
    Const(f32),
}
impl CompVal {
    fn evaluate(&self, data: &PlayerData) -> f32 {
        match self {
            Self::Recovery   => data.recovery()   as f32,
            Self::Focus      => data.focus()      as f32,
            Self::Reasoning  => data.reasoning()  as f32,
            Self::Fitness    => data.fitness()    as f32,
            Self::Strength   => data.strength()   as f32,
            Self::Speed      => data.speed()      as f32,
            Self::Charisma   => data.charisma()   as f32,
            Self::Expression => data.expression() as f32,
            Self::Deception  => data.deception()  as f32,
            Self::Const(v)   => *v,
        }
    }
}
impl FromStr for CompVal {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if debug_parse() { println!("parsing comparison value: {s}") }
        match s {
            "recovery" => Ok(Self::Recovery),
                "focus"     => Ok(Self::Focus),
                "reasoning" => Ok(Self::Reasoning),
            "fitness" => Ok(Self::Fitness),
                "speed" => Ok(Self::Speed),
                "strength" => Ok(Self::Strength),
            "charisma" => Ok(Self::Charisma),
                "expression" => Ok(Self::Expression),
                "deception" => Ok(Self::Deception),
            _ => Ok(Self::Const(s.parse()?))
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
    NotEqual,
}
impl FromStr for Comparison {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if debug_parse() { println!("parsing comparison: {s}") }
        match s {
            "<"  => Ok(Self::LessThan),
            "<=" => Ok(Self::LessThanOrEqual),
            ">"  => Ok(Self::GreaterThan),
            ">=" => Ok(Self::GreaterThanOrEqual),
            "="  => Ok(Self::Equal),
            "!=" => Ok(Self::NotEqual),
            _ => Err(Self::Err::InvalidComparison)
        }
    }
}
impl Comparison {
    pub fn evaluate(self, v1: &CompVal, v2: &CompVal, data: &PlayerData) -> bool {
        match self {
            Self::LessThan           => v1.evaluate(data) <  v2.evaluate(data),
            Self::LessThanOrEqual    => v1.evaluate(data) <= v2.evaluate(data),
            Self::GreaterThan        => v1.evaluate(data) >  v2.evaluate(data),
            Self::GreaterThanOrEqual => v1.evaluate(data) >= v2.evaluate(data),
            Self::Equal              => (v1.evaluate(data) - v2.evaluate(data)).abs() <= f32::EPSILON,
            Self::NotEqual           => (v1.evaluate(data) - v2.evaluate(data)).abs() >  f32::EPSILON,
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum ReqVal {
    Not(Box<ReqVal>),
    Req(Box<Requirements>),
    Tag(String),
    Comparison(CompVal, Comparison, CompVal),
}
impl ReqVal {
    fn evaluate(&self, data: &PlayerData) -> bool {
        match self {
            Self::Not(i) => !i.evaluate(data),
            Self::Req(i) => i.evaluate(data),
            Self::Tag(s) => data.acquired_tags.contains(s),
            Self::Comparison(v1, c, v2) => c.evaluate(v1, v2, data),
        }
    }
}
impl FromStr for ReqVal {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if debug_parse() { println!("parsing requirement value: {s}") }
        match &s[0..1] {
            "[" => {
                let (req, skip) = split_bracket(&s[1..])?;
                if !s[(skip+1)..].trim().is_empty() { todo!() }
                Ok(Self::Req(Box::new(req.parse()?)))
            },
            "!" => Ok(Self::Not(Box::new(s[1..].parse()?))),
             _  => {
                if let Ok((s, c, e)) = split_comparison(s) {
                    Ok(Self::Comparison(s.parse()?, c.parse()?, e.parse()?))
                } else {
                    Ok(Self::Tag(s.trim().to_string()))
                }
             }
        }
    }
}
fn split_comparison(s: &str) -> Result<(String, String, String), ParseErr> {
    if debug_parse() { println!("splitting comparison: {s}") }
    let mut start      = String::new();
    let mut comparison = String::new();
    let mut ending     = false;
    let mut end        = String::new();
    for c in s.chars() {
        match c {
            '<' | '=' | '>' | '!' => { comparison.push(c); ending = true },
            c => if ending { end.push(c) } else { start.push(c) }
        }
    }
    if start.is_empty() | comparison.is_empty() | end.is_empty() { Err(ParseErr::InvalidComparison) }
    else { Ok((start, comparison, end)) }
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReqOps {
    And,
    Or,
}
#[derive(Debug, PartialEq, Clone)]
pub struct Requirements {
    head: ReqVal,
    body: Vec<(ReqOps, ReqVal)>
}
impl Requirements {
    fn evaluate(&self, data: &PlayerData) -> bool {
        let mut result = self.head.evaluate(data);
        for (ops, val) in &self.body {
            match ops {
                ReqOps::And => result &= val.evaluate(data),
                ReqOps::Or  => result |= val.evaluate(data),
            }
        }
        result
    }
}
impl FromStr for Requirements {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if debug_parse() { println!("parsing requirements: {s}") }
        let s = s.replace(char::is_whitespace, "");
        let (head, s) = s.split_at(s.find(['&', '|']).unwrap_or(s.len()));
        let mut ops = None;

        let head = head.parse()?;
        let mut body = Vec::new();
        let mut last = 1;
        for (i, c) in s.chars().enumerate() {
            match c {
                '&' => {
                    if let Some(ops) = ops { body.push((ops, s[last..i].parse()?))}
                    last = i + 1;
                    ops = Some(ReqOps::And);
                },
                '|' => {
                    if let Some(ops) = ops { body.push((ops, s[last..i].parse()?)) }
                    last = i + 1;
                    ops = Some(ReqOps::Or);

                },
                _ => if ops.is_none() { return Err(ParseErr::InvalidReqOps) }
            }
        }

        if let Some(ops) = ops { body.push((ops, s[last..].parse()?)) }
        Ok(Self { head, body })
    }
}
