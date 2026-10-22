//! Bounded Pratt parser. No scripting, dynamic code, I/O, or implicit coercions.
use anyhow::{Result, bail, ensure};

/// Host lookup for named values and geometry functions.
pub trait Context {
    /// Resolve `m.name`, `v.name`, or a host-defined constant.
    fn resolve(&mut self, name: &str) -> Result<f64>;
    /// Resolve geometry functions with raw object IDs as arguments.
    fn geometry(&mut self, name: &str, ids: &[String]) -> Result<f64>;
}
#[derive(Debug)]
enum Expr {
    Number(f64),
    Name(String),
    Unary(bool, Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}
struct Parser<'a> {
    source: &'a [u8],
    pos: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self
            .source
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.space();
        self.source.get(self.pos).copied()
    }
    fn take(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expr(&mut self, min: u8, depth: usize) -> Result<Expr> {
        self.nodes += 1;
        ensure!(
            depth <= 64 && self.nodes <= 2048,
            "expression exceeds complexity limit"
        );
        let mut lhs = if self.take(b'-') {
            Expr::Unary(true, Box::new(self.expr(5, depth + 1)?))
        } else if self.take(b'+') {
            Expr::Unary(false, Box::new(self.expr(5, depth + 1)?))
        } else if self.take(b'(') {
            let v = self.expr(0, depth + 1)?;
            ensure!(self.take(b')'), "expected ')' at byte {}", self.pos);
            v
        } else if self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'.') {
            let start = self.pos;
            while self
                .source
                .get(self.pos)
                .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
            {
                self.pos += 1;
            }
            if self
                .source
                .get(self.pos)
                .is_some_and(|c| *c == b'e' || *c == b'E')
            {
                self.pos += 1;
                if self
                    .source
                    .get(self.pos)
                    .is_some_and(|c| *c == b'+' || *c == b'-')
                {
                    self.pos += 1;
                }
                while self.source.get(self.pos).is_some_and(u8::is_ascii_digit) {
                    self.pos += 1;
                }
            }
            let s = std::str::from_utf8(&self.source[start..self.pos])?;
            Expr::Number(
                s.parse()
                    .map_err(|_| anyhow::anyhow!("invalid number '{s}'"))?,
            )
        } else if self
            .peek()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        {
            let start = self.pos;
            while self
                .source
                .get(self.pos)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'.')
            {
                self.pos += 1;
            }
            let name = std::str::from_utf8(&self.source[start..self.pos])?.to_owned();
            if self.take(b'(') {
                let mut args = vec![];
                if !self.take(b')') {
                    loop {
                        args.push(self.expr(0, depth + 1)?);
                        if self.take(b')') {
                            break;
                        }
                        ensure!(self.take(b','), "expected ',' at byte {}", self.pos);
                    }
                }
                Expr::Call(name, args)
            } else {
                Expr::Name(name)
            }
        } else {
            bail!("expected value at byte {}", self.pos)
        };
        while let Some(op) = self.peek() {
            let (l, r) = match op {
                b'+' | b'-' => (1, 2),
                b'*' | b'/' => (3, 4),
                b'^' => (7, 6),
                _ => break,
            };
            if l < min {
                break;
            }
            self.pos += 1;
            lhs = Expr::Binary(
                op as char,
                Box::new(lhs),
                Box::new(self.expr(r, depth + 1)?),
            );
        }
        Ok(lhs)
    }
}
fn run(e: &Expr, ctx: &mut impl Context) -> Result<f64> {
    let v = match e {
        Expr::Number(v) => *v,
        Expr::Name(n) if n == "pi" => std::f64::consts::PI,
        Expr::Name(n) => ctx.resolve(n)?,
        Expr::Unary(neg, a) => run(a, ctx)? * if *neg { -1.0 } else { 1.0 },
        Expr::Binary(op, a, b) => {
            let (a, b) = (run(a, ctx)?, run(b, ctx)?);
            match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => {
                    ensure!(b != 0.0, "division by zero");
                    a / b
                }
                '^' => a.powf(b),
                _ => bail!("unsupported operator '{op}'"),
            }
        }
        Expr::Call(n, args)
            if matches!(n.as_str(), "length" | "distance" | "angle" | "x" | "y") =>
        {
            let ids = args
                .iter()
                .map(|a| {
                    if let Expr::Name(n) = a {
                        Ok(n.clone())
                    } else {
                        bail!("{n} expects object IDs")
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            ctx.geometry(n, &ids)?
        }
        Expr::Call(n, args) => {
            let a = args
                .iter()
                .map(|a| run(a, ctx))
                .collect::<Result<Vec<_>>>()?;
            match (n.as_str(), a.as_slice()) {
                ("sqrt", [x]) => x.sqrt(),
                ("cap_height", [width, half_length, max_height]) => {
                    crate::block_solvers::cap_height(*width, *half_length, *max_height)?
                }
                ("seam_height", [dx, target]) => crate::block_solvers::seam_height(*dx, *target)?,
                ("sin", [x]) => x.to_radians().sin(),
                ("cos", [x]) => x.to_radians().cos(),
                ("tan", [x]) => {
                    ensure!(
                        x.to_radians().cos().abs() > 1e-14,
                        "tan undefined at {x} degrees"
                    );
                    x.to_radians().tan()
                }
                ("asin", [x]) => x.asin().to_degrees(),
                ("acos", [x]) => x.acos().to_degrees(),
                ("atan2", [y, x]) => {
                    ensure!(*x != 0.0 || *y != 0.0, "atan2 undefined at origin");
                    y.atan2(*x).to_degrees()
                }
                ("abs", [x]) => x.abs(),
                ("round", [x]) => x.round(),
                ("min", [_, _, ..]) => a.iter().copied().fold(f64::INFINITY, f64::min),
                ("max", [_, _, ..]) => a.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                _ => bail!("unknown function or wrong argument count: {n}"),
            }
        }
    };
    ensure!(v.is_finite(), "expression produced a non-finite value");
    Ok(v)
}
/// Parse and evaluate an expression. Input is limited to 16 KiB and 64 nesting levels.
pub fn evaluate(source: &str, context: &mut impl Context) -> Result<f64> {
    ensure!(source.len() <= 16384, "expression exceeds 16 KiB");
    let mut p = Parser {
        source: source.as_bytes(),
        pos: 0,
        nodes: 0,
    };
    let e = p.expr(0, 0)?;
    ensure!(p.peek().is_none(), "unexpected token at byte {}", p.pos);
    run(&e, context)
}
/// Object and variable dependencies in a validated formula.
pub fn references(source: &str) -> Result<(Vec<String>, Vec<String>)> {
    let e = parse(source)?;
    let mut objects = Vec::new();
    let mut variables = Vec::new();
    fn walk(e: &Expr, objects: &mut Vec<String>, variables: &mut Vec<String>) {
        match e {
            Expr::Name(n) => {
                if let Some(v) = n.strip_prefix("v.") {
                    variables.push(v.into());
                }
            }
            Expr::Unary(_, a) => walk(a, objects, variables),
            Expr::Binary(_, a, b) => {
                walk(a, objects, variables);
                walk(b, objects, variables);
            }
            Expr::Call(n, args)
                if matches!(n.as_str(), "length" | "distance" | "angle" | "x" | "y") =>
            {
                for a in args {
                    if let Expr::Name(id) = a {
                        objects.push(id.clone());
                    }
                }
            }
            Expr::Call(_, args) => {
                for a in args {
                    walk(a, objects, variables);
                }
            }
            _ => {}
        }
    }
    walk(&e, &mut objects, &mut variables);
    objects.sort();
    objects.dedup();
    variables.sort();
    variables.dedup();
    Ok((objects, variables))
}
fn parse(source: &str) -> Result<Expr> {
    ensure!(source.len() <= 16384, "expression exceeds 16 KiB");
    let mut p = Parser {
        source: source.as_bytes(),
        pos: 0,
        nodes: 0,
    };
    let e = p.expr(0, 0)?;
    ensure!(p.peek().is_none(), "unexpected token at byte {}", p.pos);
    Ok(e)
}
/// Rename only object IDs used as geometry-function arguments.
pub fn rename_reference(source: &str, old: &str, new: &str) -> Result<String> {
    fn render(e: &Expr, old: &str, new: &str, geometry_arg: bool) -> String {
        match e {
            Expr::Number(n) => n.to_string(),
            Expr::Name(n) => {
                if geometry_arg && n == old {
                    new.into()
                } else {
                    n.clone()
                }
            }
            Expr::Unary(neg, a) => format!(
                "({}{})",
                if *neg { "-" } else { "+" },
                render(a, old, new, false)
            ),
            Expr::Binary(op, a, b) => format!(
                "({}{}{})",
                render(a, old, new, false),
                op,
                render(b, old, new, false)
            ),
            Expr::Call(n, args) => format!(
                "{}({})",
                n,
                args.iter()
                    .map(|a| render(
                        a,
                        old,
                        new,
                        matches!(n.as_str(), "length" | "distance" | "angle" | "x" | "y")
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
    Ok(render(&parse(source)?, old, new, false))
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Empty;
    impl Context for Empty {
        fn resolve(&mut self, n: &str) -> Result<f64> {
            bail!("unknown {n}")
        }
        fn geometry(&mut self, _: &str, _: &[String]) -> Result<f64> {
            bail!("no geometry")
        }
    }
    #[test]
    fn arithmetic_and_functions() {
        for (s, v) in [
            ("-2^2", -4.),
            ("2^3^2", 512.),
            ("(2+3)*4/2", 10.),
            ("sqrt(9)+sin(30)+cos(60)", 4.),
            ("atan2(1,0)", 90.),
            ("min(4,2,3)+max(3,9)+abs(-2)+round(1.6)", 15.),
            ("asin(1)+acos(0)+tan(45)", 181.),
            ("1e-3", 0.001),
        ] {
            assert!((evaluate(s, &mut Empty).unwrap() - v).abs() < 1e-9, "{s}");
        }
    }
    #[test]
    fn rejects_unsafe_or_invalid() {
        for s in [
            "1/0",
            "sqrt(-1)",
            "1;2",
            "foo(2)",
            "sin()",
            "2 3",
            "m.missing",
            "asin(2)",
            "tan(90)",
            "atan2(0,0)",
        ] {
            assert!(evaluate(s, &mut Empty).is_err(), "{s}");
        }
        assert!(evaluate(&"(".repeat(100), &mut Empty).is_err());
    }
}
