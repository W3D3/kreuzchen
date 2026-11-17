use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Value),
    Name(String),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Or,
    And,
    Eq,
    Ne,
    Ge,
    Le,
    Gt,
    Lt,
    Add,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Name(String),
    Op(Op),
    Not,
    True,
    False,
    LParen,
    RParen,
}

const INCOMPLETE: &str = "Ausdruck unvollständig";

fn unexpected(i: usize) -> String {
    format!("Unerwartetes Zeichen an Position {i}")
}

fn tokenize(src: &str) -> Result<Vec<(usize, Tok)>, String> {
    let c: Vec<char> = src.chars().collect();
    let ident = |i: &mut usize| {
        let s = *i;
        while *i < c.len() && (c[*i].is_ascii_alphanumeric() || c[*i] == '_') {
            *i += 1;
        }
        c[s..*i].iter().collect::<String>()
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let start = i;
        let ch = c[i];
        let tok = if ch.is_whitespace() {
            i += 1;
            continue;
        } else if ch.is_ascii_digit() {
            while i < c.len() && c[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < c.len() && c[i] == '.' && c[i + 1].is_ascii_digit() {
                i += 1;
                while i < c.len() && c[i].is_ascii_digit() {
                    i += 1;
                }
            }
            Tok::Num(c[start..i].iter().collect::<String>().parse().unwrap())
        } else if ch == '"' {
            i += 1;
            let s = i;
            while i < c.len() && c[i] != '"' {
                i += 1;
            }
            if i >= c.len() {
                return Err(INCOMPLETE.into());
            }
            i += 1;
            Tok::Str(c[s..i - 1].iter().collect())
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let mut w = ident(&mut i);
            if i + 1 < c.len() && c[i] == '.' && (c[i + 1].is_ascii_alphabetic() || c[i + 1] == '_')
            {
                i += 1;
                w = format!("{w}.{}", ident(&mut i));
            }
            match w.as_str() {
                "and" => Tok::Op(Op::And),
                "or" => Tok::Op(Op::Or),
                "not" => Tok::Not,
                "true" => Tok::True,
                "false" => Tok::False,
                _ => Tok::Name(w),
            }
        } else {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            let (op, n) = match (two.as_str(), ch) {
                ("==", _) => (Op::Eq, 2),
                ("!=", _) => (Op::Ne, 2),
                (">=", _) => (Op::Ge, 2),
                ("<=", _) => (Op::Le, 2),
                (_, '>') => (Op::Gt, 1),
                (_, '<') => (Op::Lt, 1),
                (_, '+') => (Op::Add, 1),
                (_, '/') => (Op::Div, 1),
                (_, '(') => {
                    i += 1;
                    out.push((start, Tok::LParen));
                    continue;
                }
                (_, ')') => {
                    i += 1;
                    out.push((start, Tok::RParen));
                    continue;
                }
                _ => return Err(unexpected(start)),
            };
            i += n;
            Tok::Op(op)
        };
        out.push((start, tok));
    }
    Ok(out)
}

struct Parser {
    toks: Vec<(usize, Tok)>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.1)
    }

    fn eat_op(&mut self, ops: &[Op]) -> Option<Op> {
        if let Some(Tok::Op(o)) = self.peek() {
            let o = *o;
            if ops.contains(&o) {
                self.pos += 1;
                return Some(o);
            }
        }
        None
    }

    fn binary(
        &mut self,
        op: Op,
        next: fn(&mut Self) -> Result<Expr, String>,
    ) -> Result<Expr, String> {
        let mut l = next(self)?;
        while self.eat_op(&[op]).is_some() {
            l = Expr::Bin(op, Box::new(l), Box::new(next(self)?));
        }
        Ok(l)
    }

    fn or(&mut self) -> Result<Expr, String> {
        self.binary(Op::Or, Self::and)
    }

    fn and(&mut self) -> Result<Expr, String> {
        self.binary(Op::And, Self::not)
    }

    fn not(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Tok::Not) {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<Expr, String> {
        let l = self.sum()?;
        match self.eat_op(&[Op::Eq, Op::Ne, Op::Ge, Op::Le, Op::Gt, Op::Lt]) {
            Some(op) => Ok(Expr::Bin(op, Box::new(l), Box::new(self.sum()?))),
            None => Ok(l),
        }
    }

    fn sum(&mut self) -> Result<Expr, String> {
        self.binary(Op::Add, Self::quotient)
    }

    fn quotient(&mut self) -> Result<Expr, String> {
        self.binary(Op::Div, Self::atom)
    }

    fn atom(&mut self) -> Result<Expr, String> {
        let Some((at, tok)) = self.toks.get(self.pos).cloned() else {
            return Err(INCOMPLETE.into());
        };
        self.pos += 1;
        Ok(match tok {
            Tok::Num(n) => Expr::Lit(Value::Num(n)),
            Tok::Str(s) => Expr::Lit(Value::Str(s)),
            Tok::True => Expr::Lit(Value::Bool(true)),
            Tok::False => Expr::Lit(Value::Bool(false)),
            Tok::Name(n) => Expr::Name(n),
            Tok::LParen => {
                let e = self.or()?;
                match self.toks.get(self.pos) {
                    Some((_, Tok::RParen)) => self.pos += 1,
                    Some((p, _)) => return Err(unexpected(*p)),
                    None => return Err(INCOMPLETE.into()),
                }
                e
            }
            _ => return Err(unexpected(at)),
        })
    }
}

pub fn parse(src: &str) -> Result<Expr, String> {
    let mut p = Parser {
        toks: tokenize(src)?,
        pos: 0,
    };
    let e = p.or()?;
    match p.toks.get(p.pos) {
        Some((at, _)) => Err(unexpected(*at)),
        None => Ok(e),
    }
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Num(_) => "Zahl",
        Value::Bool(_) => "Wahrheitswert",
        Value::Str(_) => "Text",
    }
}

pub(crate) fn want_bool(v: Value) -> Result<bool, String> {
    match v {
        Value::Bool(b) => Ok(b),
        v => Err(format!(
            "Typfehler: Wahrheitswert erwartet, {} gefunden",
            kind(&v)
        )),
    }
}

fn want_num(v: Value) -> Result<f64, String> {
    match v {
        Value::Num(n) => Ok(n),
        v => Err(format!("Typfehler: Zahl erwartet, {} gefunden", kind(&v))),
    }
}

/// Evaluates both sides of `and`/`or`, so validation sees every name and type.
pub fn eval(e: &Expr, env: &BTreeMap<String, Value>) -> Result<Value, String> {
    run(e, env, false)
}

/// Skips the right side of `and`/`or` once the left decides, so `alter.gesetzt and alter > 24`
/// grades when no age is entered.
pub fn eval_short(e: &Expr, env: &BTreeMap<String, Value>) -> Result<Value, String> {
    run(e, env, true)
}

fn run(e: &Expr, env: &BTreeMap<String, Value>, short: bool) -> Result<Value, String> {
    let eval = |e: &Expr| run(e, env, short);
    match e {
        Expr::Lit(v) => Ok(v.clone()),
        Expr::Name(n) => env
            .get(n)
            .cloned()
            .ok_or_else(|| format!("„{n}“ hat keinen Wert")),
        Expr::Not(x) => Ok(Value::Bool(!want_bool(eval(x)?)?)),
        Expr::Bin(op @ (Op::And | Op::Or), l, r) if short => {
            let l = want_bool(eval(l)?)?;
            if l == (*op == Op::Or) {
                return Ok(Value::Bool(l));
            }
            Ok(Value::Bool(want_bool(eval(r)?)?))
        }
        Expr::Bin(op, l, r) => {
            let (l, r) = (eval(l)?, eval(r)?);
            Ok(match op {
                Op::And => {
                    let (a, b) = (want_bool(l)?, want_bool(r)?);
                    Value::Bool(a && b)
                }
                Op::Or => {
                    let (a, b) = (want_bool(l)?, want_bool(r)?);
                    Value::Bool(a || b)
                }
                Op::Add => Value::Num(want_num(l)? + want_num(r)?),
                // x / 0 stays infinite here; grading rejects it, validation's zero stand-ins must not.
                Op::Div => Value::Num(want_num(l)? / want_num(r)?),
                Op::Eq | Op::Ne => {
                    if std::mem::discriminant(&l) != std::mem::discriminant(&r) {
                        return Err(format!(
                            "Typfehler: {} und {} sind nicht vergleichbar",
                            kind(&l),
                            kind(&r)
                        ));
                    }
                    Value::Bool((l == r) == (*op == Op::Eq))
                }
                _ => {
                    let (a, b) = (want_num(l)?, want_num(r)?);
                    Value::Bool(match op {
                        Op::Ge => a >= b,
                        Op::Le => a <= b,
                        Op::Gt => a > b,
                        _ => a < b,
                    })
                }
            })
        }
    }
}

pub fn names(e: &Expr) -> BTreeSet<String> {
    match e {
        Expr::Lit(_) => BTreeSet::new(),
        Expr::Name(n) => BTreeSet::from([n.clone()]),
        Expr::Not(x) => names(x),
        Expr::Bin(_, l, r) => &names(l) | &names(r),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str, env: &[(&str, Value)]) -> Result<Value, String> {
        let env: BTreeMap<String, Value> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        eval(&parse(src)?, &env)
    }

    #[test]
    fn short_circuit_skips_an_unset_name() {
        let env: BTreeMap<String, Value> = BTreeMap::new();
        let e = parse("false and x > 1").unwrap();
        assert_eq!(eval_short(&e, &env), Ok(Value::Bool(false)));
        assert_eq!(
            eval_short(&parse("true or x").unwrap(), &env),
            Ok(Value::Bool(true))
        );
        // Validation evaluates both sides, so a wrong name is still reported.
        assert!(eval(&e, &env).is_err());
    }

    #[test]
    fn precedence() {
        assert_eq!(run("1 + 2 >= 3 and not false", &[]), Ok(Value::Bool(true)));
    }

    #[test]
    fn division() {
        assert_eq!(run("7 / 2", &[]), Ok(Value::Num(3.5)));
        assert_eq!(run("1 + 6 / 3 / 2", &[]), Ok(Value::Num(2.0)));
    }

    #[test]
    fn dotted_names() {
        let env = [("A.over", Value::Bool(true)), ("A", Value::Num(40.0))];
        assert_eq!(run("A.over and A > 39", &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn string_equality() {
        let env = [("sev", Value::Str("Ja".into()))];
        assert_eq!(run("sev == \"Ja\"", &env), Ok(Value::Bool(true)));
        assert_eq!(run("sev != \"Ja\"", &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn missing_name() {
        assert!(run("x > 1", &[])
            .unwrap_err()
            .contains("„x“ hat keinen Wert"));
    }

    #[test]
    fn type_error() {
        assert!(run("1 and true", &[]).unwrap_err().contains("Typfehler"));
    }

    #[test]
    fn parse_errors() {
        for s in ["r1 and and", "(1 + 2", "1 +"] {
            assert!(parse(s).is_err(), "{s}");
        }
    }

    #[test]
    fn names_collects() {
        let n = names(&parse("gesamt.over and not k1 or fehlend <= 1").unwrap());
        let want: BTreeSet<String> = ["gesamt.over", "k1", "fehlend"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(n, want);
    }

    #[test]
    fn both_operands_type_checked() {
        for s in ["false and 1", "true or 1", "1 == \"a\""] {
            assert!(run(s, &[]).unwrap_err().contains("Typfehler"), "{s}");
        }
    }

    #[test]
    fn keyword_prefixed_names_and_bad_input() {
        let env = [("android", Value::Bool(true)), ("notiz", Value::Bool(true))];
        assert_eq!(run("android and notiz", &env), Ok(Value::Bool(true)));
        assert!(parse("ä").is_err());
        assert!(parse("a.b.c").is_err());
    }
}
