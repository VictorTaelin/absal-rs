#![allow(dead_code)]

use net::*;
use std;

// λ-Terms are either lambdas, variables or applications.
#[derive(Clone, Debug)]
pub enum Term {
    App {fun: Box<Term>, arg: Box<Term>},
    Lam {bod: Box<Term>},
    Var {idx: u32}
}
use self::Term::{*};

// Source code is Ascii-encoded.
pub type Str = [u8];
pub type Chr = u8;

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
pub fn parse_term<'a>(code : &'a Str, ctx : &mut Context<'a>) -> (&'a Str, Term) {
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

pub fn from_net(net : &Net) -> Term {
    fn go(net : &Net, node_depth : &mut Vec<u32>, next : Port, exit : &mut Vec<Port>, depth : u32) -> Term {
        let prev_port = enter(net, next);
        let prev_slot = slot(prev_port);
        let prev_node = node(prev_port);
        if kind(net, prev_node) == 1 {
            match prev_slot {
                0 => {
                    node_depth[prev_node as usize] = depth;
                    Lam{bod: Box::new(go(net, node_depth, port(prev_node, 2), exit, depth + 1))}
                },
                1 => {
                    Var{idx: depth - node_depth[prev_node as usize] - 1}
                },
                _ => {
                    let fun = go(net, node_depth, port(prev_node, 0), exit, depth);
                    let arg = go(net, node_depth, port(prev_node, 1), exit, depth);
                    App{fun: Box::new(fun), arg: Box::new(arg)}
                }
            }
        } else if prev_slot > 0 {
            exit.push(prev_slot);
            let term = go(net, node_depth, port(prev_node, 0), exit, depth);
            exit.pop();
            term
        } else {
            let e = exit.pop().unwrap();
            let term = go(net, node_depth, port(prev_node, e), exit, depth);
            exit.push(e);
            term
        }
    }
    let mut node_depth : Vec<u32> = Vec::with_capacity(net.nodes.len() / 4);
    let mut exit : Vec<u32> = Vec::new();
    node_depth.resize(net.nodes.len() / 4, 0);
    go(net, &mut node_depth, 0, &mut exit, 0)
}

pub fn to_net(term : &Term) -> Net {
    fn encode(net : &mut Net, _kind : &mut u32, scope : &mut Vec<u32>, term : &Term) -> Port {
        match term {
            &App{ref fun, ref arg} => {
                let app = new_node(net, 1);
                let fun = encode(net, _kind, scope, fun);
                link(net, port(app, 0), fun);
                let arg = encode(net, _kind, scope, arg);
                link(net, port(app, 1), arg);
                port(app, 2)
            },
            &Lam{ref bod} => {
                let fun = new_node(net, 1);
                let era = new_node(net, 0);
                link(net, port(fun, 1), port(era, 0));
                link(net, port(era, 1), port(era, 2));
                scope.push(fun);
                let bod = encode(net, _kind, scope, bod);
                scope.pop();
                link(net, port(fun, 2), bod);
                port(fun, 0)
            },
            &Var{ref idx} => {
                let lam = scope[scope.len() - 1 - (*idx as usize)];
                let arg = enter(net, port(lam, 1));
                if kind(net, node(arg)) == 0 {
                    net.reuse.push(node(arg));
                    port(lam, 1)
                } else {
                    *_kind += 1;
                    let dup = new_node(net, *_kind);
                    link(net, port(dup, 2), arg);
                    link(net, port(dup, 0), port(lam, 1));
                    port(dup, 1)
                }
            }
        }
    }
    let mut net : Net = Net { nodes: vec![0,2,1,4], reuse: vec![] };
    let mut kind : u32 = 1;
    let mut scope : Vec<u32> = Vec::new();
    let ptr : Port = encode(&mut net, &mut kind, &mut scope, term);
    link(&mut net, 0, ptr);
    net
}
