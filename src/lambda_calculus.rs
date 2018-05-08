#![allow(dead_code)]

// λ-Terms are either lambdas, variables or applications.
#[derive(Clone, Debug)]
pub enum Term {
    App {fun: Box<Term>, arg: Box<Term>},
    Lam {bod: Box<Term>},
    Var {idx: u32}
}
use self::Term::{*};
use std;

// Source code is Ascii-encoded.
type Str = [u8];
type Chr = u8;

// A context is a vector of (name, value) assignments.
type Context<'a> = Vec<(&'a Str, Option<Term>)>;

// Extends a context with a (name, value) assignments.
fn extend<'a,'b>(nam : &'a Str, val : Option<Term>, ctx : &'b mut Context<'a>) -> &'b mut Context<'a> {
    ctx.push((nam,val));
    ctx
}

// Removes an assignment from a context.
fn narrow<'a,'b>(ctx : &'b mut Context<'a>) -> &'b mut Context<'a> {
    ctx.pop();
    ctx
}

// Parses a name, returns the remaining code and the name.
fn parse_name(code : &Str) -> (&Str, &Str) {
    let mut i : usize = 0;
    while i < code.len() && !(code[i] == b' ' || code[i] == b'\n') {
        i += 1;
    }
    (&code[i..], &code[0..i])
}

// Parses a term, returns the remaining code and the term. Syntax:
// - lam:   #var body      -- same as: λvar. body
// - app:   /f x           -- same as: f(x)
// - def:   @var val bod   -- same as: bod[val/var]
// - let:   :var val bod   -- same as: (λvar. bod)(val)
fn parse_term<'a>(code : &'a Str, ctx : &mut Context<'a>) -> (&'a Str, Term) {
    match code[0] {
        // Whitespace
        b' ' => parse_term(&code[1..], ctx),
        // Newline
        b'\n' => parse_term(&code[1..], ctx),
        // Applicationn
        b'/' => {
            let (code, fun) = parse_term(&code[1..], ctx);
            let (code, arg) = parse_term(code, ctx);
            let fun = Box::new(fun);
            let arg = Box::new(arg);
            (code, App{fun,arg})
        },
        // Lambda
        b'#' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, bod) = parse_term(code, extend(nam, None, ctx));
            let bod = Box::new(bod);
            narrow(ctx);
            (code, Lam{bod})
        },
        // Definition
        b'@' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, val) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend(nam, Some(val), ctx));
            narrow(ctx);
            (code, bod)

        },
        // Let
        b':' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, val) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend(nam, None, ctx));
            let bod = Box::new(bod);
            let fun = Box::new(Lam{bod});
            let arg = Box::new(val);
            narrow(ctx);
            (code, App{fun,arg})
        },
        // Variable
        _ => {
            let (code, nam) = parse_name(code);
            let mut idx : u32 = 0;
            let mut val : Option<Term> = None;
            for i in (0..ctx.len()).rev() {
                if ctx[i].0 == nam {
                    val = ctx[i].1.clone();
                    break;
                }
                idx = idx + (match &ctx[i].1 { &Some(ref _t) => 0, &None => 1});
            }
            (code, match val { Some(term) => term, None => Var{idx} })
        }
    }
}

// Converts a source-code to a λ-term.
pub fn from_string<'a>(code : &'a Str) -> Term {
    let mut ctx = Vec::new();
    let (_code, term) = parse_term(code, &mut ctx);
    term
}

// Builds a var name from an index (0="a", 1="b", 26="aa"...).
pub fn var_name(idx : u32) -> Vec<Chr> {
    let mut name = Vec::new();
    let mut idx  = idx;
    while idx > 0 {
        idx = idx - 1;
        name.push((97 + idx % 26) as u8);
        idx = idx / 26;
    }
    return name;
}

// Converts a λ-term back to a source-code.
pub fn to_string(term : &Term) -> Vec<Chr> {
    fn build(code : &mut Vec<u8>, term : &Term, dph : u32) {
        match term {
            &App{ref fun, ref arg} => {
                code.extend_from_slice(b"/");
                build(code, &fun, dph);
                code.extend_from_slice(b" ");
                build(code, &arg, dph);
            },
            &Lam{ref bod} => {
                code.extend_from_slice(b"#");
                code.append(&mut var_name(dph + 1));
                code.extend_from_slice(b" ");
                build(code, &bod, dph + 1);
            }
            &Var{idx} => {
                code.append(&mut var_name(dph - idx));
            },
        }
    }
    let mut code = Vec::new();
    build(&mut code, term, 0);
    return code;
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&to_string(&self)))
    }
}
