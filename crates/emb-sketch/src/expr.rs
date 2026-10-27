//! Expressions of parameters and dimensions (P14 Milestone 6, ADR 0078).
//!
//! An expression is a calculation with numbers, units, the names of
//! parameters and a few functions, for example `80 mm`, `width / 20` or
//! `max(border, 2 mm) + 1`. Each value has a power of length: 1 for a
//! length (in mm), 0 for a plain number (a factor, a count or an angle in
//! degrees), 2 for an area. `*` and `/` add and subtract the powers. In a
//! sum, a plain number and a length are both lengths: the plain number is
//! in the units of the sketch (`Units`). A dimension takes a length (a plain
//! number is in the units of the sketch), an angle takes a plain number.
//!
//! The parser is a usual recursive descent parser, in our own code.

use serde::{Deserialize, Serialize};

/// The units of numbers that have no unit, where a length is necessary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Units {
    #[default]
    Mm,
    Inch,
}

impl Units {
    /// The size of one unit in mm.
    pub fn mm(self) -> f64 {
        match self {
            Units::Mm => 1.0,
            Units::Inch => 25.4,
        }
    }

    pub fn is_mm(&self) -> bool {
        *self == Units::Mm
    }
}

/// A value and its power of length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    /// In mm to the power `length`.
    pub value: f64,
    pub length: i32,
}

impl Quantity {
    pub fn number(value: f64) -> Quantity {
        Quantity { value, length: 0 }
    }

    pub fn mm(value: f64) -> Quantity {
        Quantity { value, length: 1 }
    }
}

/// Why an expression or a parameter is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExprError {
    #[error("type a value, for example 80 mm or width / 2")]
    Empty,
    #[error("the expression is not complete or has a wrong sign: {0}")]
    Syntax(String),
    #[error("there is no parameter with the name {0}")]
    Unknown(String),
    #[error("the parameters make a loop: {0}")]
    Loop(String),
    #[error("a length and an area (or other sizes) cannot be added")]
    Units,
    #[error("the value is not a length")]
    NotLength,
    #[error("the value is not an angle")]
    NotAngle,
    #[error("division by 0")]
    DivideByZero,
    #[error("the value is not a number in its range")]
    BadValue,
    #[error("a name has letters, digits and _ and starts with a letter: {0}")]
    BadName(String),
    #[error("there is a parameter with the name {0} already")]
    Taken(String),
    #[error("other values use the parameter {0}")]
    InUse(String),
    #[error("a driven dimension follows the sketch: it takes no expression")]
    Driven,
}

impl ExprError {
    /// The message as a text with `{}` and its value, for the translations.
    pub fn parts(&self) -> (&'static str, Option<String>) {
        let s = |t: &str| Some(t.to_string());
        match self {
            ExprError::Empty => ("type a value, for example 80 mm or width / 2", None),
            ExprError::Syntax(t) => ("the expression is not complete or has a wrong sign: {}", s(t)),
            ExprError::Unknown(t) => ("there is no parameter with the name {}", s(t)),
            ExprError::Loop(t) => ("the parameters make a loop: {}", s(t)),
            ExprError::Units => ("a length and an area (or other sizes) cannot be added", None),
            ExprError::NotLength => ("the value is not a length", None),
            ExprError::NotAngle => ("the value is not an angle", None),
            ExprError::DivideByZero => ("division by 0", None),
            ExprError::BadValue => ("the value is not a number in its range", None),
            ExprError::BadName(t) => ("a name has letters, digits and _ and starts with a letter: {}", s(t)),
            ExprError::Taken(t) => ("there is a parameter with the name {} already", s(t)),
            ExprError::InUse(t) => ("other values use the parameter {}", s(t)),
            ExprError::Driven => ("a driven dimension follows the sketch: it takes no expression", None),
        }
    }

    /// All message texts (for the translations).
    pub const MESSAGES: [&'static str; 13] = [
        "type a value, for example 80 mm or width / 2",
        "the expression is not complete or has a wrong sign: {}",
        "there is no parameter with the name {}",
        "the parameters make a loop: {}",
        "a length and an area (or other sizes) cannot be added",
        "the value is not a length",
        "the value is not an angle",
        "division by 0",
        "the value is not a number in its range",
        "a name has letters, digits and _ and starts with a letter: {}",
        "there is a parameter with the name {} already",
        "other values use the parameter {}",
        "a driven dimension follows the sketch: it takes no expression",
    ];
}

/// The units after a number: the size in mm for a length, else the factor
/// to degrees for an angle.
const LENGTHS: [(&str, f64); 7] = [
    ("mm", 1.0),
    ("cm", 10.0),
    ("m", 1000.0),
    ("in", 25.4),
    ("\"", 25.4),
    ("ft", 304.8),
    ("pt", 25.4 / 72.0),
];
const ANGLES: [(&str, f64); 3] = [("deg", 1.0), ("°", 1.0), ("rad", 180.0 / std::f64::consts::PI)];
const FUNCTIONS: [&str; 7] = ["sqrt", "abs", "min", "max", "sin", "cos", "tan"];

/// True if `name` can be the name of a parameter.
pub fn valid_name(name: &str) -> bool {
    let mut c = name.chars();
    c.next().is_some_and(|f| f.is_alphabetic())
        && name.chars().all(|x| x.is_alphanumeric() || x == '_')
        && !LENGTHS.iter().chain(&ANGLES).any(|(u, _)| *u == name)
        && !FUNCTIONS.contains(&name)
        && name != "pi"
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Name(String),
    Unit(Quantity),
    Op(char),
}

fn tokens(text: &str) -> Result<Vec<Token>, ExprError> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // An exponent: 1e3, 2.5E-2.
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    i = j;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let s: String = chars[start..i].iter().collect();
            let v: f64 = s.parse().map_err(|_| ExprError::Syntax(s.clone()))?;
            out.push(Token::Num(v));
            // A unit right after the number.
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            let rest: String = chars[j..].iter().collect();
            let word_end = |u: &str| {
                let after = rest[u.len()..].chars().next();
                !after.is_some_and(|a| a.is_alphanumeric() || a == '_')
            };
            let unit = LENGTHS
                .iter()
                .map(|(u, k)| (*u, Quantity::mm(*k)))
                .chain(ANGLES.iter().map(|(u, k)| (*u, Quantity::number(*k))))
                .filter(|(u, _)| rest.starts_with(u) && word_end(u))
                .max_by_key(|(u, _)| u.len());
            if let Some((u, q)) = unit {
                out.push(Token::Unit(q));
                i = j + u.chars().count();
            }
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
        } else if "+-*/^(),".contains(c) {
            out.push(Token::Op(c));
            i += 1;
        } else {
            return Err(ExprError::Syntax(c.to_string()));
        }
    }
    Ok(out)
}

/// A parsed expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Value(Quantity),
    Name(String),
    Neg(Box<Expr>),
    Op(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

struct Parser {
    t: Vec<Token>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.t.get(self.i)
    }

    fn op(&mut self, c: char) -> bool {
        if self.peek() == Some(&Token::Op(c)) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn syntax(&self) -> ExprError {
        ExprError::Syntax(match self.peek() {
            Some(Token::Op(c)) => c.to_string(),
            Some(Token::Name(n)) => n.clone(),
            Some(Token::Num(v)) => v.to_string(),
            Some(Token::Unit(_)) => "unit".into(),
            None => "…".into(),
        })
    }

    fn sum(&mut self) -> Result<Expr, ExprError> {
        let mut a = self.product()?;
        loop {
            let c = if self.op('+') {
                '+'
            } else if self.op('-') {
                '-'
            } else {
                return Ok(a);
            };
            a = Expr::Op(c, Box::new(a), Box::new(self.product()?));
        }
    }

    fn product(&mut self) -> Result<Expr, ExprError> {
        let mut a = self.unary()?;
        loop {
            let c = if self.op('*') {
                '*'
            } else if self.op('/') {
                '/'
            } else {
                return Ok(a);
            };
            a = Expr::Op(c, Box::new(a), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, ExprError> {
        if self.op('-') {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.op('+') {
            return self.unary();
        }
        let a = self.atom()?;
        if self.op('^') {
            return Ok(Expr::Op('^', Box::new(a), Box::new(self.unary()?)));
        }
        Ok(a)
    }

    fn atom(&mut self) -> Result<Expr, ExprError> {
        match self.peek().cloned() {
            Some(Token::Num(v)) => {
                self.i += 1;
                if let Some(Token::Unit(u)) = self.peek().cloned() {
                    self.i += 1;
                    return Ok(Expr::Value(Quantity {
                        value: v * u.value,
                        length: u.length,
                    }));
                }
                Ok(Expr::Value(Quantity::number(v)))
            }
            Some(Token::Name(n)) => {
                self.i += 1;
                if self.op('(') {
                    if !FUNCTIONS.contains(&n.as_str()) {
                        return Err(ExprError::Unknown(n));
                    }
                    let mut args = vec![self.sum()?];
                    while self.op(',') {
                        args.push(self.sum()?);
                    }
                    if !self.op(')') {
                        return Err(self.syntax());
                    }
                    return Ok(Expr::Call(n, args));
                }
                if n == "pi" {
                    return Ok(Expr::Value(Quantity::number(std::f64::consts::PI)));
                }
                Ok(Expr::Name(n))
            }
            Some(Token::Op('(')) => {
                self.i += 1;
                let e = self.sum()?;
                if !self.op(')') {
                    return Err(self.syntax());
                }
                Ok(e)
            }
            _ => Err(self.syntax()),
        }
    }
}

/// Parses `text`.
pub fn parse(text: &str) -> Result<Expr, ExprError> {
    let t = tokens(text)?;
    if t.is_empty() {
        return Err(ExprError::Empty);
    }
    let mut p = Parser { t, i: 0 };
    let e = p.sum()?;
    if p.i < p.t.len() {
        return Err(p.syntax());
    }
    Ok(e)
}

impl Expr {
    /// The names of parameters in the expression.
    pub fn names(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out.sort();
        out.dedup();
        out
    }

    fn collect(&self, out: &mut Vec<String>) {
        match self {
            Expr::Value(_) => {}
            Expr::Name(n) => out.push(n.clone()),
            Expr::Neg(a) => a.collect(out),
            Expr::Op(_, a, b) => {
                a.collect(out);
                b.collect(out);
            }
            Expr::Call(_, args) => args.iter().for_each(|a| a.collect(out)),
        }
    }

    /// The value, with the values of the names from `name` and plain
    /// numbers in sums with lengths in `units`.
    pub fn eval(&self, name: &dyn Fn(&str) -> Option<Quantity>, units: Units) -> Result<Quantity, ExprError> {
        let same = |a: Quantity, b: Quantity| -> Result<(Quantity, Quantity), ExprError> {
            match (a.length, b.length) {
                (x, y) if x == y => Ok((a, b)),
                (0, 1) => Ok((Quantity::mm(a.value * units.mm()), b)),
                (1, 0) => Ok((a, Quantity::mm(b.value * units.mm()))),
                _ => Err(ExprError::Units),
            }
        };
        let q = match self {
            Expr::Value(q) => *q,
            Expr::Name(n) => name(n).ok_or_else(|| ExprError::Unknown(n.clone()))?,
            Expr::Neg(a) => {
                let q = a.eval(name, units)?;
                Quantity {
                    value: -q.value,
                    length: q.length,
                }
            }
            Expr::Op(c, a, b) => {
                let (a, b) = (a.eval(name, units)?, b.eval(name, units)?);
                match c {
                    '+' | '-' => {
                        let (a, b) = same(a, b)?;
                        let v = if *c == '+' {
                            a.value + b.value
                        } else {
                            a.value - b.value
                        };
                        Quantity {
                            value: v,
                            length: a.length,
                        }
                    }
                    '*' => Quantity {
                        value: a.value * b.value,
                        length: a.length + b.length,
                    },
                    '/' => {
                        if b.value == 0.0 {
                            return Err(ExprError::DivideByZero);
                        }
                        Quantity {
                            value: a.value / b.value,
                            length: a.length - b.length,
                        }
                    }
                    _ => {
                        // A power: a plain exponent; a length only to a
                        // whole power.
                        if b.length != 0 || (a.length != 0 && b.value.fract() != 0.0) {
                            return Err(ExprError::Units);
                        }
                        Quantity {
                            value: a.value.powf(b.value),
                            length: a.length * b.value as i32,
                        }
                    }
                }
            }
            Expr::Call(f, args) => {
                let v: Vec<Quantity> = args.iter().map(|a| a.eval(name, units)).collect::<Result<_, _>>()?;
                let one = || -> Result<Quantity, ExprError> {
                    match v.as_slice() {
                        [x] => Ok(*x),
                        _ => Err(ExprError::Syntax(f.clone())),
                    }
                };
                let plain = |x: Quantity| {
                    if x.length == 0 {
                        Ok(x.value)
                    } else {
                        Err(ExprError::NotAngle)
                    }
                };
                match f.as_str() {
                    "sqrt" => {
                        let x = one()?;
                        if x.length % 2 != 0 || x.value < 0.0 {
                            return Err(ExprError::BadValue);
                        }
                        Quantity {
                            value: x.value.sqrt(),
                            length: x.length / 2,
                        }
                    }
                    "abs" => {
                        let x = one()?;
                        Quantity {
                            value: x.value.abs(),
                            length: x.length,
                        }
                    }
                    "min" | "max" => {
                        let mut it = v.iter().copied();
                        let mut best = it.next().ok_or_else(|| ExprError::Syntax(f.clone()))?;
                        for x in it {
                            let (a, b) = same(best, x)?;
                            best = if (f == "min") == (b.value < a.value) { b } else { a };
                        }
                        best
                    }
                    "sin" => Quantity::number(emb_geom::math::sin(plain(one()?)?.to_radians())),
                    "cos" => Quantity::number(emb_geom::math::cos(plain(one()?)?.to_radians())),
                    _ => Quantity::number(emb_geom::math::tan(plain(one()?)?.to_radians())),
                }
            }
        };
        if !q.value.is_finite() {
            return Err(ExprError::BadValue);
        }
        Ok(q)
    }
}

/// The value as a length in mm (a plain number is in `units`).
pub fn to_length(q: Quantity, units: Units) -> Result<f64, ExprError> {
    match q.length {
        1 => Ok(q.value),
        0 => Ok(q.value * units.mm()),
        _ => Err(ExprError::NotLength),
    }
}

/// The value as an angle in degrees.
pub fn to_angle(q: Quantity) -> Result<f64, ExprError> {
    if q.length == 0 {
        Ok(q.value)
    } else {
        Err(ExprError::NotAngle)
    }
}

/// Replaces the name `old` with `new` in `text` (whole names only).
pub fn rename(text: &str, old: &str, new: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let mut after_number = false;
    let flush = |word: &mut String, out: &mut String, after_number: bool| {
        if !after_number && word == old {
            out.push_str(new);
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    let mut prev_digit = false;
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' {
            if word.is_empty() {
                after_number = prev_digit;
            }
            word.push(c);
        } else {
            flush(&mut word, &mut out, after_number);
            out.push(c);
        }
        prev_digit = c.is_ascii_digit() || (prev_digit && c == ' ');
    }
    flush(&mut word, &mut out, after_number);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(text: &str) -> Result<Quantity, ExprError> {
        let names = |n: &str| match n {
            "width" => Some(Quantity::mm(80.0)),
            "n" => Some(Quantity::number(4.0)),
            _ => None,
        };
        parse(text)?.eval(&names, Units::Mm)
    }

    #[test]
    fn numbers_units_names_and_functions() {
        assert_eq!(eval("80 mm").unwrap(), Quantity::mm(80.0));
        assert_eq!(eval("2in").unwrap(), Quantity::mm(50.8));
        assert_eq!(eval("1\"").unwrap(), Quantity::mm(25.4));
        assert_eq!(eval("width / 20").unwrap(), Quantity::mm(4.0));
        assert_eq!(eval("width / n + 2").unwrap(), Quantity::mm(22.0));
        assert_eq!(eval("-(width - 10 mm) * 2").unwrap(), Quantity::mm(-140.0));
        assert_eq!(eval("2 ^ 3").unwrap(), Quantity::number(8.0));
        assert_eq!(eval("sqrt(width * width)").unwrap(), Quantity::mm(80.0));
        assert_eq!(eval("max(width / 40, 3 mm)").unwrap(), Quantity::mm(3.0));
        assert!((eval("cos(60)").unwrap().value - 0.5).abs() < 1e-12);
        assert!((eval("pi").unwrap().value - std::f64::consts::PI).abs() < 1e-12);
        assert_eq!(eval("1.5e1 cm").unwrap(), Quantity::mm(150.0));
        assert!((to_angle(eval("1 rad").unwrap()).unwrap() - 57.29577951308232).abs() < 1e-9);
        // A plain number is a length in the units of the sketch.
        let q = parse("3 + 1 in").unwrap().eval(&|_| None, Units::Inch).unwrap();
        assert!((q.value - 101.6).abs() < 1e-9);
        assert!((to_length(Quantity::number(2.0), Units::Inch).unwrap() - 50.8).abs() < 1e-12);
    }

    #[test]
    fn wrong_expressions_are_refused() {
        assert_eq!(eval(""), Err(ExprError::Empty));
        assert_eq!(eval("width +"), Err(ExprError::Syntax("…".into())));
        assert_eq!(eval("(width"), Err(ExprError::Syntax("…".into())));
        assert_eq!(eval("height"), Err(ExprError::Unknown("height".into())));
        assert_eq!(eval("width * width + width"), Err(ExprError::Units));
        assert_eq!(eval("width / 0"), Err(ExprError::DivideByZero));
        assert_eq!(eval("3 $"), Err(ExprError::Syntax("$".into())));
        assert_eq!(
            to_length(eval("width * width").unwrap(), Units::Mm),
            Err(ExprError::NotLength)
        );
        assert_eq!(to_angle(eval("width").unwrap()), Err(ExprError::NotAngle));
        assert_eq!(
            parse("a + b * a").unwrap().names(),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn names_and_renames() {
        assert!(valid_name("width") && valid_name("border_2") && valid_name("Breite"));
        assert!(!valid_name("2nd") && !valid_name("mm") && !valid_name("sqrt") && !valid_name("a b"));
        assert_eq!(rename("width/2 + widths + 3 in", "width", "w"), "w/2 + widths + 3 in");
        assert_eq!(rename("in + 2 in", "in", "x"), "x + 2 in");
    }
}
