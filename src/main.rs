#![allow(dead_code)]

// λ-Terms are either lambdas, variables or applications.
#[derive(Clone, Debug)]
enum Term {
    App {fun: Box<Term>, arg: Box<Term>},
    Lam {bod: Box<Term>},
    Var {idx: u32}
}
use Term::{*};

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
fn from_string<'a>(code : &'a Str) -> Term {
    let mut ctx = Vec::new();
    let (_code, term) = parse_term(code, &mut ctx);
    term
}

// Builds a var name from an index (0="a", 1="b", 26="aa"...).
fn var_name(idx : u32) -> Vec<Chr> {
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
fn to_string(term : &Term) -> Vec<Chr> {
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

#[derive(Clone, Debug)]
struct Stats {
    loops: u32,
    rules: u32,
    betas: u32,
    dupls: u32,
    annis: u32
}

#[derive(Clone, Debug)]
struct Net {
    nodes: Vec<u32>,
    reuse: Vec<u32>
}


type Port = u32;

fn new_node(net : &mut Net, kind : u32) -> u32 {
    let node : u32 = match net.reuse.pop() {
        Some(index) => index,
        None        => {
            let len = net.nodes.len();
            net.nodes.resize(len + 4, 0);
            (len as u32) / 4
        }
    };
    net.nodes[(node * 4 + 0) as usize] = node * 4 + 0;
    net.nodes[(node * 4 + 1) as usize] = node * 4 + 1;
    net.nodes[(node * 4 + 2) as usize] = node * 4 + 2;
    net.nodes[(node * 4 + 3) as usize] = kind << 2;
    return node;
}

fn port(node : u32, slot : u32) -> Port {
    (node << 2) | slot
}

fn get_port_node(port : Port) -> u32 {
    port >> 2
}

fn get_port_slot(port : Port) -> u32 {
    port & 3
}

fn enter_port(net : &Net, port : Port) -> Port {
    net.nodes[port as usize]
}

fn get_node_kind(net : &Net, node_index : u32) -> u32 {
    net.nodes[(node_index * 4 + 3) as usize] >> 2
}

fn get_node_meta(net : &Net, node_index : u32) -> u32 {
    net.nodes[(node_index * 4 + 3) as usize] & 3
}

fn set_node_meta(net : &mut Net, node_index : u32, meta : u32) {
    let ptr = (node_index * 4 + 3) as usize;
    net.nodes[ptr] = net.nodes[ptr] & 0xFFFFFFFC | meta;
}

fn link(net : &mut Net, ptr_a : u32, ptr_b : u32) {
    net.nodes[ptr_a as usize] = ptr_b;
    net.nodes[ptr_b as usize] = ptr_a;
}

fn to_net(term : &Term) -> Net {
    fn encode(net : &mut Net, kind : &mut u32, scope : &mut Vec<u32>, term : &Term) -> Port {
        match term {
            &App{ref fun, ref arg} => {
                let app = new_node(net, 1);
                let fun = encode(net, kind, scope, fun);
                link(net, port(app, 0), fun);
                let arg = encode(net, kind, scope, arg);
                link(net, port(app, 1), arg);
                port(app, 2)
            },
            &Lam{ref bod} => {
                let fun = new_node(net, 1);
                let era = new_node(net, 0);
                link(net, port(fun, 1), port(era, 0));
                link(net, port(era, 1), port(era, 2));
                scope.push(fun);
                let bod = encode(net, kind, scope, bod);
                scope.pop();
                link(net, port(fun, 2), bod);
                port(fun, 0)
            },
            &Var{ref idx} => {
                let lam = scope[scope.len() - 1 - (*idx as usize)];
                if get_node_kind(net, get_port_node(enter_port(net, port(lam, 1)))) == 0 {
                    port(lam, 1)
                } else {
                    *kind = *kind + 1;
                    let dup = new_node(net, *kind);
                    let arg = enter_port(net, port(lam, 1));
                    link(net, port(dup, 1), arg);
                    link(net, port(dup, 0), port(lam, 1));
                    port(dup, 2)
                }
            }
        }
    }
    let mut net : Net = Net { nodes: vec![0,1,2,0], reuse: vec![] };
    let mut kind : u32 = 1;
    let mut scope : Vec<u32> = Vec::new();
    let ptr : Port = encode(&mut net, &mut kind, &mut scope, term);
    link(&mut net, 0, ptr);
    net
}

fn from_net(net : &Net) -> Term {
    fn go(net : &Net, node_depth : &mut Vec<u32>, next : Port, exit : &mut Vec<Port>, depth : u32) -> Term {
        let prev_port = enter_port(net, next);
        let prev_slot = get_port_slot(prev_port);
        let prev_node = get_port_node(prev_port);
        //println!("{} {:?} {} {} {} {}", next, exit, depth, prev_port, prev_slot, prev_node);
        if get_node_kind(net, prev_node) == 1 {
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

fn reduce(net : &mut Net) -> Stats {
    let mut stats = Stats { loops: 0, rules: 0, betas: 0, dupls: 0, annis: 0 };
    let mut next : Port = net.nodes[0];
    let mut prev : Port;
    let mut back : Port;
    while next > 0 {
        prev = enter_port(net, next);
        next = enter_port(net, prev);
        if get_port_slot(next) == 0 {
            if get_port_slot(prev) == 0 && get_port_node(prev) != 0 {
                stats.rules = stats.rules + 1;
                back = enter_port(net, port(get_port_node(prev), get_node_meta(net, get_port_node(prev))));
                rewrite(net, get_port_node(prev), get_port_node(next));
                next = enter_port(net, back);
            } else {
                set_node_meta(net, get_port_node(next), 1);
                next = enter_port(net, port(get_port_node(next), 1));
            }
        } else {
            let meta = get_node_meta(net, get_port_node(next));
            set_node_meta(net, get_port_node(next), if meta == 0 { get_port_slot(next) } else { meta + 1});
            next = enter_port(net, port(get_port_node(next), if meta == 1 { 2 } else { 0 }));
        }
        stats.loops = stats.loops + 1;
    }
    stats
}

fn rewrite(net : &mut Net, x : Port, y : Port) {
    if get_node_kind(net, x) == get_node_kind(net, y) {
        let p0 = enter_port(net, port(x, 1));
        let p1 = enter_port(net, port(y, 1));
        link(net, p0, p1);
        let p0 = enter_port(net, port(x, 2));
        let p1 = enter_port(net, port(y, 2));
        link(net, p0, p1);
        net.reuse.push(x);
        net.reuse.push(y);
    } else {
        let t = get_node_kind(net, x);
        let a = new_node(net, t);
        let t = get_node_kind(net, y);
        let b = new_node(net, t);
        let t = enter_port(net, port(x, 1));
        link(net, port(b, 0), t);
        let t = enter_port(net, port(x, 2));
        link(net, port(y, 0), t);
        let t = enter_port(net, port(y, 1));
        link(net, port(a, 0), t);
        let t = enter_port(net, port(y, 2));
        link(net, port(x, 0), t);
        link(net, port(a, 1), port(b, 1));
        link(net, port(a, 2), port(y, 1));
        link(net, port(x, 1), port(b, 2));
        link(net, port(x, 2), port(y, 2));
        set_node_meta(net, x, 0);
        set_node_meta(net, y, 0);
    }
}

fn main() {
    // Parses the following λ-program:
    //   two = λf. λx. f (f x)
    //   exp = λn. λm. m n
    //   exp two two
    let code = b"/ #f #x /f /f x #f #x /f /f x";
    let term = from_string(code);
    let mut net = to_net(&term);
    let stats = reduce(&mut net);
    println!("Stats     : {:?}", stats);
    println!("{}", net.nodes.len());
}
