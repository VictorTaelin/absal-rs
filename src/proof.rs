#![allow(dead_code)]

use std;

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum Proof {
    App {fun: Box<Proof>, arg: Box<Proof>},
    Lam {typ: Box<Proof>, bod: Box<Proof>},
    All {typ: Box<Proof>, bod: Box<Proof>},
    Var {idx: u32},
    Inc {val: Box<Proof>}, 
    Dec {val: Box<Proof>},
    Lvl {val: Box<Proof>},
    Set
}
use self::Proof::{*};

// Source code is Ascii-encoded.
pub type Str = [u8];
pub type Chr = u8;

// A context is a vector of (name, value) assignments.
type Context<'a> = Vec<(&'a Str, Option<Proof>)>;

// Extends a context with a (name, value) assignments.
fn extend<'a,'b>(nam : &'a Str, val : Option<Proof>, ctx : &'b mut Context<'a>) -> &'b mut Context<'a> {
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
pub fn parse_term<'a>(code : &'a Str, ctx : &mut Context<'a>) -> (&'a Str, Proof) {
    match code[0] {
        // Whitespace
        b' ' => parse_term(&code[1..], ctx),
        // Newline
        b'\n' => parse_term(&code[1..], ctx),
        // Applicationn
        b':' => {
            let (code, fun) = parse_term(&code[1..], ctx);
            let (code, arg) = parse_term(code, ctx);
            let fun = Box::new(fun);
            let arg = Box::new(arg);
            (code, App{fun,arg})
        },
        // Lambda
        b'#' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, typ) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend(nam, None, ctx));
            let typ = Box::new(typ);
            let bod = Box::new(bod);
            narrow(ctx);
            (code, Lam{typ,bod})
        },
        // Forall
        b'@' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, typ) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend(nam, None, ctx));
            let typ = Box::new(typ);
            let bod = Box::new(bod);
            narrow(ctx);
            (code, All{typ,bod})
        },
        // Definition
        b'$' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, val) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend(nam, Some(val), ctx));
            narrow(ctx);
            (code, bod)

        },
        // Inc
        b'|' => {
            let (code, val) = parse_term(&code[1..], ctx);
            let val = Box::new(val);
            (code, Inc{val})
        },
        // Dec
        b'~' => {
            let (code, val) = parse_term(&code[1..], ctx);
            let val = Box::new(val);
            (code, Dec{val})
        },
        // Lvl
        b'!' => {
            let (code, val) = parse_term(&code[1..], ctx);
            let val = Box::new(val);
            (code, Lvl{val})
        },
        // Set
        b'*' => {
            (&code[1..], Set)
        },
        // Variable
        _ => {
            let (code, nam) = parse_name(code);
            let mut idx : u32 = 0;
            let mut val : Option<Proof> = None;
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
pub fn from_string<'a>(code : &'a Str) -> Proof {
    let mut ctx = Vec::new();
    let (_code, term) = parse_term(code, &mut ctx);
    term
}

// Builds a var name from an index (0="a", 1="b", 26="aa"...).
pub fn var_name(idx : u32) -> Vec<Chr> {
    let mut name = Vec::new();
    let mut idx  = idx;
    if idx == 0 {
        name.push(63);
    }
    while idx > 0 {
        idx = idx - 1;
        name.push((97 + idx % 26) as u8);
        idx = idx / 26;
    }
    return name;
}

// Converts a λ-term back to a source-code.
pub fn to_string(term : &Proof) -> Vec<Chr> {
    fn build(code : &mut Vec<u8>, term : &Proof, dph : u32) {
        match term {
            &App{ref fun, ref arg} => {
                code.extend_from_slice(b":");
                build(code, &fun, dph);
                code.extend_from_slice(b" ");
                build(code, &arg, dph);
            },
            &Lam{ref typ, ref bod} => {
                code.extend_from_slice(b"#");
                code.append(&mut var_name(dph + 1));
                code.extend_from_slice(b" ");
                build(code, &typ, dph);
                code.extend_from_slice(b" ");
                build(code, &bod, dph + 1);
            },
            &All{ref typ, ref bod} => {
                code.extend_from_slice(b"@");
                code.append(&mut var_name(dph + 1));
                code.extend_from_slice(b" ");
                build(code, &typ, dph);
                code.extend_from_slice(b" ");
                build(code, &bod, dph + 1);
            },
            &Var{idx} => {
                code.append(&mut var_name(dph - idx));
            },
            &Inc{ref val} => {
                code.extend_from_slice(b"|");
                build(code, &val, dph);
            },
            &Dec{ref val} => {
                code.extend_from_slice(b"~");
                build(code, &val, dph);
            },
            &Lvl{ref val} => {
                code.extend_from_slice(b"!");
                build(code, &val, dph);
            },
            &Set => {
                code.extend_from_slice(b"*");
            }
        }
    }
    let mut code = Vec::new();
    build(&mut code, term, 0);
    return code;
}

pub fn shift(proof : &mut Proof, d : u32, c : u32) {
    match proof {
        &mut App{ref mut fun, ref mut arg} => {
            shift(fun, d, c);
            shift(arg, d, c);
        },
        &mut Lam{ref mut typ, ref mut bod} => {
            shift(typ, d, c);
            shift(bod, d, c+1);
        },
        &mut All{ref mut typ, ref mut bod} => {
            shift(typ, d, c);
            shift(bod, d, c+1);
        },
        &mut Var{ref mut idx} => {
            *idx = if *idx < c { *idx } else { *idx + d };
        },
        &mut Inc{ref mut val} => {
            shift(val, d, c);
        },
        &mut Dec{ref mut val} => {
            shift(val, d, c);
        },
        &mut Lvl{ref mut val} => {
            shift(val, d, c);
        },
        &mut Set => {}
    }
}

pub fn subs(proof : &mut Proof, value : &Proof, dph : u32) {
    let var_idx = match proof {
        &mut App{ref mut fun, ref mut arg} => {
            subs(fun, value, dph);
            subs(arg, value, dph);
            None
        },
        &mut Lam{ref mut typ, ref mut bod} => {
            subs(typ, value, dph);
            subs(bod, value, dph+1);
            None
        },
        &mut All{ref mut typ, ref mut bod} => {
            subs(typ, value, dph);
            subs(bod, value, dph+1);
            None
        },
        &mut Var{idx} => {
            Some(idx)
        },
        &mut Inc{ref mut val} => {
            subs(val, value, dph);
            None
        },
        &mut Dec{ref mut val} => {
            subs(val, value, dph);
            None
        },
        &mut Lvl{ref mut val} => {
            subs(val, value, dph);
            None
        },
        &mut Set => {
            None
        }
    };
    match var_idx {
        Some(idx) => {
            if dph == idx {
                let mut val = value.clone();
                shift(&mut val, dph, 0);
                *proof = val
            } else if dph < idx {
                *proof = Var{idx: idx - 1}
            }
        },
        None => {}
    }
}

pub fn reduce(proof : &Proof) -> Proof {
    match proof {
        &App{ref fun, ref arg} => {
            let fun = reduce(fun);
            match fun {
                Lam{typ:_typ, bod} => {
                    let mut new_bod = *bod.clone();
                    subs(&mut new_bod, arg, 0);
                    reduce(&new_bod)
                },
                _ => App{fun: Box::new(fun), arg: Box::new(reduce(&arg))}
            }
        },
        &Lam{ref typ, ref bod} => {
            let typ = Box::new(reduce(typ));
            let bod = Box::new(reduce(bod));
            Lam{typ,bod}
        },
        &All{ref typ, ref bod} => {
            let typ = Box::new(reduce(typ));
            let bod = Box::new(reduce(bod));
            All{typ,bod}
        },
        Var{idx} => {
            Var{idx: *idx}
        },
        Inc{ref val} => {
            let val = Box::new(reduce(val));
            Inc{val}
        },
        Dec{ref val} => {
            let val = Box::new(reduce(val));
            Dec{val}
        },
        Lvl{ref val} => {
            let val = Box::new(reduce(val));
            Lvl{val}
        },
        Set => {
            Set
        }
    }
}

pub fn infer(proof : &Proof) -> Proof {
    //println!("| {:?} |", proof);
    pub fn infer<'a>(proof : &'a Proof, ctx : &'a mut Vec<Box<Proof>>) -> Proof {
        match proof {
            &App{ref fun, ref arg} => {
                let fun_t = infer(fun, ctx);
                let arg_t = infer(arg, ctx);
                let arg_n = reduce(arg);
                match fun_t {
                    All{ref typ, ref bod} => {
                        let mut new_bod = bod.clone();
                        let a : &Proof = &arg_t;
                        let b : &Proof = typ;
                        if a != b {
                            panic!("Type mismatch.");
                        }
                        //println!("app {:?} {:?}", fun_t, new_bod);
                        subs(&mut new_bod, &arg_n, 0);
                        *new_bod
                    },
                    _ => panic!("Non-function application.")
                }
            },
            &Lam{ref typ, ref bod} => {
                let typ_n = Box::new(reduce(typ));
                ctx.push(typ_n.clone());
                for i in 0..ctx.len() {
                    shift(&mut ctx[i], 1, 0);
                }
                //println!("> {:?} <", ctx);
                let bod_t = Box::new(infer(bod, ctx));
                ctx.pop();
                All{typ: typ_n, bod: bod_t}
            },
            &All{ref typ, bod: ref _bod} => {
                let typ_n = Box::new(reduce(typ));
                //let typ_t = Box::new(infer(typ, ctx));
                ctx.push(typ_n);
                for i in 0..ctx.len() {
                    shift(&mut ctx[i], 1, 0);
                }
                //let bod_t = Box::new(infer(bod, ctx));
                ctx.pop();
                Set
            },
            &Var{idx} => {
                *ctx[ctx.len() - (idx as usize) - 1].clone()
            },
            &Inc{ref val} => {
                let val_t = infer(val, ctx);
                Lvl{val: Box::new(val_t)}
            },
            &Dec{ref val} => {
                let val_t = infer(val, ctx);
                match val_t {
                    Lvl{val} => *val,
                    _ => panic!("type error dec")
                }
            },
            &Lvl{ref val} => {
                let val_t = infer(val, ctx);
                match val_t {
                    Set => Set,
                    _ => panic!("type error lvl")
                }
            },
            &Set => {
                Set
            }
        }
    }
    let mut ctx : Vec<Box<Proof>> = Vec::new();
    infer(proof, &mut ctx)
}

impl std::fmt::Display for Proof {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&to_string(&self)))
    }
}

pub fn bod(proof : &Proof) -> &Proof {
    match proof {
        Lam{typ:_typ,bod} => bod,
        _ => panic!()
    }
}
