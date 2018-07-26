//#![allow(dead_code)]

use std;

// Source code is Ascii-encoded.
pub type Str = [u8];
pub type Chr = u8;

//#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
//pub enum Term {
    //App {fun: Box<Term>, arg: Box<Term>},
    //Lam {bod: Box<Proof>},
    //All {typ: Box<Proof>, bod: Box<Proof>},
    //Var {idx: u32},
    //Dup {nam: Vec<u8>, val: Box<Proof>, bod: Box<Proof>},
    //Inc {val: Box<Proof>}, 
    //Lvl {val: Box<Proof>},
    //Set
//}
//use self::Proof::{*};

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum Proof {
    App {era: bool, fun: Box<Proof>, arg: Box<Proof>},
    Lam {era: bool, nam: Vec<u8>, typ: Box<Proof>, bod: Box<Proof>},
    All {era: bool, nam: Vec<u8>, typ: Box<Proof>, bod: Box<Proof>},
    Var {idx: u32},
    Dup {nam: Vec<u8>, val: Box<Proof>, bod: Box<Proof>},
    Inc {val: Box<Proof>}, 
    Lvl {val: Box<Proof>},
    Set
}
use self::Proof::{*};

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum VarType {
    Affine,
    Exponential,
    Polymorphic
}
use self::VarType::{*};

// A Scope is a vector of (name, value) assignments.
type Scope<'a> = Vec<(&'a Str, Option<Proof>)>;

// Extends a scope with a (name, value) assignments.
fn extend_scope<'a,'b>(nam : &'a Str, val : Option<Proof>, ctx : &'b mut Scope<'a>) -> &'b mut Scope<'a> {
    ctx.push((nam,val));
    ctx
}

// Removes an assignment from a Scope.
fn narrow_scope<'a,'b>(ctx : &'b mut Scope<'a>) -> &'b mut Scope<'a> {
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
pub fn parse_term<'a>(code : &'a Str, ctx : &mut Scope<'a>) -> (&'a Str, Proof) {
    match code[0] {
        // Whitespace
        b' ' => parse_term(&code[1..], ctx),
        // Newline
        b'\n' => parse_term(&code[1..], ctx),
        // Polymorphic specialization
        b'.' => {
            let (code, fun) = parse_term(&code[1..], ctx);
            let (code, arg) = parse_term(code, ctx);
            let fun = Box::new(fun);
            let arg = Box::new(arg);
            (code, App{era:true,fun,arg})
        },
        // Polymorphic value
        b'^' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, typ) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend_scope(nam, None, ctx));
            let nam = nam.to_vec();
            let typ = Box::new(typ);
            let bod = Box::new(bod);
            narrow_scope(ctx);
            (code, Lam{era:true,nam,typ,bod})
        },
        // Polymorphic type
        b'&' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, typ) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend_scope(nam, None, ctx));
            let nam = nam.to_vec();
            let typ = Box::new(typ);
            let bod = Box::new(bod);
            narrow_scope(ctx);
            (code, All{era:true,nam,typ,bod})
        },
        // Application
        b':' => {
            let (code, fun) = parse_term(&code[1..], ctx);
            let (code, arg) = parse_term(code, ctx);
            let fun = Box::new(fun);
            let arg = Box::new(arg);
            (code, App{era:false,fun,arg})
        },
        // Lambda
        b'#' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, typ) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend_scope(nam, None, ctx));
            let nam = nam.to_vec();
            let typ = Box::new(typ);
            let bod = Box::new(bod);
            narrow_scope(ctx);
            (code, Lam{era:false,nam,typ,bod})
        },
        // Forall
        b'@' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, typ) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend_scope(nam, None, ctx));
            let nam = nam.to_vec();
            let typ = Box::new(typ);
            let bod = Box::new(bod);
            narrow_scope(ctx);
            (code, All{era:false,nam,typ,bod})
        },
        // Definition
        b'$' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, val) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend_scope(nam, Some(val), ctx));
            narrow_scope(ctx);
            (code, bod)
        },
        // Dup
        b'=' => {
            let (code, nam) = parse_name(&code[1..]);
            let (code, val) = parse_term(code, ctx);
            let (code, bod) = parse_term(code, extend_scope(nam, None, ctx));
            let nam = nam.to_vec();
            let val = Box::new(val);
            let bod = Box::new(bod);
            narrow_scope(ctx);
            (code, Dup{nam,val,bod})
        },
        // Inc
        b'|' => {
            let (code, val) = parse_term(&code[1..], ctx);
            let val = Box::new(val);
            (code, Inc{val})
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
            &App{era, ref fun, ref arg} => {
                code.extend_from_slice(if era { b"." } else { b":" });
                build(code, &fun, dph);
                code.extend_from_slice(b" ");
                build(code, &arg, dph);
            },
            &Lam{era, nam: ref _nam, ref typ, ref bod} => {
                code.extend_from_slice(if era { b"^" } else { b"#" });
                code.append(&mut var_name(dph + 1));
                code.extend_from_slice(b" ");
                build(code, &typ, dph);
                code.extend_from_slice(b" ");
                build(code, &bod, dph + 1);
            },
            &All{era, nam: ref _nam, ref typ, ref bod} => {
                code.extend_from_slice(if era { b"&" } else { b"@" });
                code.append(&mut var_name(dph + 1));
                code.extend_from_slice(b" ");
                build(code, &typ, dph);
                code.extend_from_slice(b" ");
                build(code, &bod, dph + 1);
            },
            &Var{idx} => {
                code.append(&mut var_name(dph - idx));
            },
            &Dup{nam: ref _nam, ref val, ref bod} => {
                code.extend_from_slice(b"=");
                code.append(&mut var_name(dph + 1));
                code.extend_from_slice(b" ");
                build(code, &val, dph);
                code.extend_from_slice(b" ");
                build(code, &bod, dph + 1);
            },
            &Inc{ref val} => {
                code.extend_from_slice(b"|");
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
        &mut App{era: _era, ref mut fun, ref mut arg} => {
            shift(fun, d, c);
            shift(arg, d, c);
        },
        &mut Lam{era: _era, nam: ref mut _nam, ref mut typ, ref mut bod} => {
            shift(typ, d, c);
            shift(bod, d, c+1);
        },
        &mut All{era: _era, nam: ref mut _nam, ref mut typ, ref mut bod} => {
            shift(typ, d, c);
            shift(bod, d, c+1);
        },
        &mut Var{ref mut idx} => {
            *idx = if *idx < c { *idx } else { *idx + d };
        },
        &mut Dup{nam: ref mut _nam, ref mut val, ref mut bod} => {
            shift(val, d, c);
            shift(bod, d, c+1);
        },
        &mut Inc{ref mut val} => {
            shift(val, d, c);
        },
        &mut Lvl{ref mut val} => {
            shift(val, d, c);
        },
        &mut Set => {}
    }
}

pub fn equals(a : &Proof, b : &Proof) -> bool {
    match (a,b) {
        (&App{era: ref _ax, fun: ref ay, arg: ref az},
         &App{era: ref _bx, fun: ref by, arg: ref bz})
         => equals(ay,by) && equals(az,bz),
        (&Lam{era: ref _ax, nam: ref _ay, typ: ref az, bod: ref aw},
         &Lam{era: ref _bx, nam: ref _by, typ: ref bz, bod: ref bw})
         => equals(az,bz) && equals(aw,bw),
        (&All{era: ref _ax, nam: ref _ay, typ: ref az, bod: ref aw},
         &All{era: ref _bx, nam: ref _by, typ: ref bz, bod: ref bw})
         => equals(az,bz) && equals(aw,bw),
        (&Var{idx: ref ax},
         &Var{idx: ref bx})
         => ax == bx,
        (&Dup{nam: ref _ax, val: ref ay, bod: ref az},
         &Dup{nam: ref _bx, val: ref by, bod: ref bz})
         => equals(ay, by) && equals(az, bz),
        (&Inc{val: ref ax}, &Inc{val: ref bx})
         => equals(ax, bx),
        (&Lvl{val: ref ax}, &Lvl{val: ref bx})
         => equals(ax, bx),
        (Set, Set)
         => true,
        _ => false
    }
}

pub fn subs(proof : &mut Proof, value : &Proof) {
    fn subs(proof : &mut Proof, value : &Proof, dph : u32) {
        let var_idx = match proof {
            &mut App{era: _era, ref mut fun, ref mut arg} => {
                subs(fun, value, dph);
                subs(arg, value, dph);
                None
            },
            &mut Lam{era: _era, nam: ref mut _nam, ref mut typ, ref mut bod} => {
                subs(typ, value, dph);
                subs(bod, value, dph+1);
                None
            },
            &mut All{era: _era, nam: ref mut _nam, ref mut typ, ref mut bod} => {
                subs(typ, value, dph);
                subs(bod, value, dph+1);
                None
            },
            &mut Var{idx} => {
                Some(idx)
            },
            &mut Dup{nam: ref mut _nam, ref mut val, ref mut bod} => {
                subs(val, value, dph);
                subs(bod, value, dph+1);
                None
            },
            &mut Inc{ref mut val} => {
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
    subs(proof, value, 0);
}

pub fn reduce(proof : &Proof) -> Proof {
    match proof {
        &App{era, ref fun, ref arg} => {
            let fun = reduce(fun);
            match fun {
                Lam{era:_era, nam:_nam, typ:_typ, bod} => {
                    let mut new_bod = *bod.clone();
                    subs(&mut new_bod, arg);
                    reduce(&new_bod)
                },
                Dup{nam: vnam, val: vval, bod: vbod} => {
                    let new_bod = Box::new(reduce(&App{era, fun: vbod, arg: Box::new(*arg.clone())})); 
                    Dup{nam: vnam, val: vval, bod: new_bod}
                },
                _ => App{era, fun: Box::new(fun), arg: Box::new(reduce(&arg))}
            }
        },
        &Lam{era, ref nam, ref typ, ref bod} => {
            let typ = Box::new(reduce(typ));
            let bod = Box::new(reduce(bod));
            Lam{era,nam:nam.to_vec(),typ,bod}
        },
        &All{era, ref nam, ref typ, ref bod} => {
            let typ = Box::new(reduce(typ));
            let bod = Box::new(reduce(bod));
            All{era,nam:nam.to_vec(),typ,bod}
        },
        &Var{idx} => {
            Var{idx}
        },
        &Dup{ref nam, ref val, ref bod} => {
            let val = reduce(val);
            match val {
                Inc{ref val} => {
                    let mut new_bod = *bod.clone();
                    subs(&mut new_bod, val);
                    reduce(&new_bod)
                },
                Dup{nam: vnam, val: vval, bod: vbod} => {
                    let new_bod = Box::new(reduce(&Dup{nam:nam.to_vec(), val: vbod, bod: Box::new(*bod.clone())})); 
                    Dup{nam: vnam, val: vval, bod: new_bod}
                },
                _ => {
                    let val = Box::new(val);
                    let bod = Box::new(reduce(bod));
                    Dup{nam:nam.to_vec(),val,bod}
                }
            }
        },
        &Inc{ref val} => {
            let val = Box::new(reduce(val));
            Inc{val}
        },
        &Lvl{ref val} => {
            let val = Box::new(reduce(val));
            Lvl{val}
        },
        Set => {
            Set
        }
    }
}

// TODO: return Result
pub fn is_stratified(proof : &Proof) -> bool {
    pub fn check<'a>(proof : &'a Proof, lvl : u32, ctx : &'a mut Vec<(Vec<u8>,VarType,bool,u32)>) { // name, is_exponential, was_used, level
        match proof {
            &App{era: _era, ref fun, ref arg} => {
                check(fun, lvl, ctx);
                check(arg, lvl, ctx);
            },
            &Lam{era, ref nam, ref typ, ref bod} => {
                check(typ, lvl, ctx);
                ctx.push((nam.to_vec(), if era { Polymorphic } else { Affine }, false, lvl));
                check(bod, lvl, ctx);
                ctx.pop();
            },
            &All{era, ref nam, ref typ, ref bod} => {
                check(typ, lvl, ctx);
                ctx.push((nam.to_vec(), if era { Polymorphic } else { Affine }, false, lvl));
                check(bod, lvl, ctx);
                ctx.pop();
            },
            &Var{idx} => {
                let pos = ctx.len() - idx as usize - 1;
                let var_nam = ctx[pos].0.clone();
                let var_typ = ctx[pos].1.clone();
                let var_use = ctx[pos].2;
                let var_lvl = ctx[pos].3;
                match var_typ {
                    Affine => {
                        if var_use {
                            panic!("Affine variable '{}' used more than once.", std::str::from_utf8(&var_nam).unwrap());
                        }
                        if lvl > var_lvl {
                            panic!("Affine variable '{}' lost its scope.", std::str::from_utf8(&var_nam).unwrap());
                        }
                    },
                    Exponential => {
                        if lvl - var_lvl != 1 {
                            panic!("Exponential variables should have surrounding box, but '{}' has {}.", std::str::from_utf8(&var_nam).unwrap(), lvl - var_lvl);
                        }
                    },
                    Polymorphic => {}
                }
                ctx[pos].2 = true;
            },
            &Dup{ref nam, ref val, ref bod} => {
                check(val, lvl, ctx);
                ctx.push((nam.to_vec(), Exponential, false, lvl));
                check(bod, lvl, ctx);
                ctx.pop();
            },
            &Inc{ref val} => {
                check(val, lvl + 1, ctx);
            },
            &Lvl{ref val} => {
                check(val, lvl, ctx);
            },
            &Set => {
            }
        }
    };
    let mut ctx : Vec<(Vec<u8>,VarType,bool,u32)> = Vec::new();
    check(proof, 0, &mut ctx);
    true
}

// A Context is a vector of (name, value) assignments.
type Context<'a> = Vec<Box<Proof>>;

// Extends a context.
fn extend_context<'a>(val : Box<Proof>, ctx : &'a mut Context<'a>) -> &'a mut Context<'a> {
    ctx.push(val);
    for i in 0..ctx.len() {
        shift(&mut ctx[i], 1, 0);
    }
    ctx
}

// Narrows a context.
fn narrow_context<'a>(ctx : &'a mut Context<'a>) -> &'a mut Context<'a> {
    ctx.pop();
    ctx
}

// TODO: return Result
pub fn infer(proof : &Proof) -> Proof {
    pub fn infer<'a>(proof : &'a Proof, ctx : &'a mut Context) -> Proof {
        match proof {
            &App{era: _era, ref fun, ref arg} => {
                let fun_t = infer(fun, ctx);
                let arg_t = infer(arg, ctx);
                let arg_n = reduce(arg);
                match fun_t {
                    All{era:_era, nam: ref _nam, ref typ, ref bod} => {
                        let mut new_bod = bod.clone();
                        let a : &Proof = &arg_t;
                        let b : &Proof = typ;
                        if !equals(a, b) {
                            panic!("Type mismatch.");
                        }
                        subs(&mut new_bod, &arg_n);
                        *new_bod
                    },
                    _ => {
                        panic!("Non-function application.");
                    }
                }
            },
            &Lam{era, ref nam, ref typ, ref bod} => {
                let typ_n = Box::new(reduce(typ));
                extend_context(typ_n.clone(), ctx);
                let bod_t = Box::new(infer(bod, ctx));
                narrow_context(ctx);
                All{era, nam: nam.to_vec(), typ: typ_n, bod: bod_t}
            },
            &All{era: _era, nam: ref _nam, ref typ, bod: ref _bod} => {
                let typ_n = Box::new(reduce(typ));
                extend_context(typ_n, ctx);
                // TODO: valid forall check
                narrow_context(ctx);
                Set
            },
            &Var{idx} => {
                *ctx[ctx.len() - (idx as usize) - 1].clone()
            },
            &Dup{nam: ref _nam, ref val, ref bod} => {
                let val_t = infer(val, ctx);
                let val_n = reduce(val);
                match val_t {
                    Lvl{val: val_t} => {
                        extend_context(val_t, ctx);
                        let mut bod_t = infer(bod, ctx);
                        narrow_context(ctx);
                        subs(&mut bod_t, &val_n);
                        bod_t
                    },
                    _ => {
                        panic!("Unboxed duplication.");
                    }
                }
            },
            &Inc{ref val} => {
                let val_t = infer(val, ctx);
                Lvl{val: Box::new(val_t)}
            },
            &Lvl{ref val} => {
                let val_t = infer(val, ctx);
                match val_t {
                    Set => Set,
                    _ => panic!("What")
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
        Lam{era:_era,nam:_nam,typ:_typ,bod} => bod,
        _ => panic!()
    }
}
