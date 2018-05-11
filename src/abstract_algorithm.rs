#![allow(dead_code)]

use lambda_calculus::{*};
use self::Term::{*};

#[derive(Clone, Debug)]
pub struct Stats {
    loops: u32,
    rules: u32,
    betas: u32,
    dupls: u32,
    annis: u32
}

#[derive(Clone, Debug)]
pub struct Net {
    pub nodes: Vec<u32>,
    pub reuse: Vec<u32>
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
    net.nodes[port(node, 0) as usize] = port(node, 0);
    net.nodes[port(node, 1) as usize] = port(node, 1);
    net.nodes[port(node, 2) as usize] = port(node, 2);
    net.nodes[port(node, 3) as usize] = kind << 2;
    return node;
}

fn port(node : u32, slot : u32) -> Port {
    (node << 2) | slot
}

fn node(port : Port) -> u32 {
    port >> 2
}

fn slot(port : Port) -> u32 {
    port & 3
}

fn enter(net : &Net, port : Port) -> Port {
    net.nodes[port as usize]
}

fn kind(net : &Net, node : u32) -> u32 {
    net.nodes[port(node, 3) as usize] >> 2
}

fn meta(net : &Net, node : u32) -> u32 {
    net.nodes[port(node, 3) as usize] & 3
}

fn set_meta(net : &mut Net, node : u32, meta : u32) {
    let ptr = port(node, 3) as usize;
    net.nodes[ptr] = net.nodes[ptr] & 0xFFFFFFFC | meta;
}

fn link(net : &mut Net, ptr_a : u32, ptr_b : u32) {
    net.nodes[ptr_a as usize] = ptr_b;
    net.nodes[ptr_b as usize] = ptr_a;
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
                if kind(net, node(enter(net, port(lam, 1)))) == 0 {
                    port(lam, 1)
                } else {
                    *_kind += 1;
                    let dup = new_node(net, *_kind);
                    let arg = enter(net, port(lam, 1));
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

pub fn from_net(net : &Net) -> Term {
    fn go(net : &Net, node_depth : &mut Vec<u32>, next : Port, exit : &mut Vec<Port>, depth : u32) -> Term {
        let prev_port = enter(net, next);
        let prev_slot = slot(prev_port);
        let prev_node = node(prev_port);
        //println!("{} {:?} {} {} {} {}", next, exit, depth, prev_port, prev_slot, prev_node);
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

pub fn reduce(net : &mut Net) -> Stats {
    let mut stats = Stats { loops: 0, rules: 0, betas: 0, dupls: 0, annis: 0 };
    let mut warp : Vec<u32> = Vec::new();
    let mut next : Port = net.nodes[0];
    let mut prev : Port;
    let mut back : Port;
    while (next > 0) || (warp.len() > 0) {
        next = if next == 0 { enter(net, port(warp.pop().unwrap(), 2)) } else { next };
        prev = enter(net, next);
        if slot(next) == 0 && slot(prev) == 0 && node(prev) != 0 {
            stats.rules += 1;
            back = enter(net, port(node(prev), meta(net, node(prev))));
            rewrite(net, node(prev), node(next));
            next = enter(net, back);
        } else if slot(next) == 0 {
            warp.push(node(next));
            next = enter(net, port(node(next), 1));
        } else {
            set_meta(net, node(next), slot(next));
            next = enter(net, port(node(next), 0));
        }
        stats.loops += 1;
    }
    stats
}

fn rewrite(net : &mut Net, x : Port, y : Port) {
    if kind(net, x) == kind(net, y) {
        let p0 = enter(net, port(x, 1));
        let p1 = enter(net, port(y, 1));
        link(net, p0, p1);
        let p0 = enter(net, port(x, 2));
        let p1 = enter(net, port(y, 2));
        link(net, p0, p1);
        net.reuse.push(x);
        net.reuse.push(y);
    } else {
        let t = kind(net, x);
        let a = new_node(net, t);
        let t = kind(net, y);
        let b = new_node(net, t);
        let t = enter(net, port(x, 1));
        link(net, port(b, 0), t);
        let t = enter(net, port(x, 2));
        link(net, port(y, 0), t);
        let t = enter(net, port(y, 1));
        link(net, port(a, 0), t);
        let t = enter(net, port(y, 2));
        link(net, port(x, 0), t);
        link(net, port(a, 1), port(b, 1));
        link(net, port(a, 2), port(y, 1));
        link(net, port(x, 1), port(b, 2));
        link(net, port(x, 2), port(y, 2));
        set_meta(net, x, 0);
        set_meta(net, y, 0);
    }
}
