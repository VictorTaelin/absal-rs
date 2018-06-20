#![allow(dead_code)]

#[derive(Clone, Debug)]
pub struct Stats {
    pub loops: u32,
    pub rules: u32,
    pub betas: u32,
    pub dupls: u32,
    pub annis: u32
}

#[derive(Clone, Debug)]
pub struct Net {
    pub nodes: Vec<u32>,
    pub reuse: Vec<u32>
}

pub const CON : u32 = 0;
pub const NUM : u32 = 1;
pub const FN2 : u32 = 2;
pub const FN1 : u32 = 3;

pub const ADD : u32 = 0;
pub const MUL : u32 = 1;
pub const SUB : u32 = 2;
pub const DIV : u32 = 3;

pub type Port = u32;

pub fn new_node(net : &mut Net, tipo : u32, kind : u32) -> u32 {
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
    net.nodes[port(node, 3) as usize] = (tipo << 30) | kind;
    return node;
}

pub fn port(node : u32, slot : u32) -> Port {
    (node << 2) | slot
}

pub fn node(port : Port) -> u32 {
    port >> 2
}

pub fn slot(port : Port) -> u32 {
    port & 3
}

pub fn enter(net : &Net, port : Port) -> Port {
    net.nodes[port as usize]
}

pub fn kind(net : &Net, node : u32) -> u32 {
    net.nodes[port(node, 3) as usize] & 0x3FFFFFFF
}

pub fn tipo(net : &Net, node : u32) -> u32 {
    net.nodes[port(node, 3) as usize] >> 30
}

pub fn link(net : &mut Net, ptr_a : u32, ptr_b : u32) {
    net.nodes[ptr_a as usize] = ptr_b;
    net.nodes[ptr_b as usize] = ptr_a;
}

pub fn reduce(net : &mut Net) -> Stats {
    let mut stats = Stats { loops: 0, rules: 0, betas: 0, dupls: 0, annis: 0 };
    let mut back : Vec<u32> = Vec::new();
    let mut warp : Vec<u32> = vec![0];
    while warp.len() > 0 {
        let prev : Port = warp.pop().unwrap();
        let next : Port = enter(net, prev);
        if next == 0 {
            continue;
        } else if slot(next) == 0 && slot(prev) == 0 && node(prev) != 0 {
            stats.rules += 1;
            rewrite(net, node(prev), node(next));
            warp.push(back.pop().unwrap());
        } else if slot(next) == 0 && tipo(net, node(next)) == NUM {
            continue;
        } else if slot(next) == 0 {
            warp.push(port(node(next), 2));
            warp.push(port(node(next), 1));
        } else {
            back.push(prev);
            warp.push(port(node(next), 0));
        }
        stats.loops += 1;
    }
    stats
}

pub fn ani_two(net : &mut Net, x : Port, y : Port) {
    let p0 = enter(net, port(x, 1));
    let p1 = enter(net, port(y, 1));
    link(net, p0, p1);
    let p0 = enter(net, port(x, 2));
    let p1 = enter(net, port(y, 2));
    link(net, p0, p1);
    net.reuse.push(x);
    net.reuse.push(y);
}

pub fn dup_two(net : &mut Net, x : Port, y : Port) {
    let t = tipo(net, x); let u = kind(net, x); let a = new_node(net, t, u);
    let t = tipo(net, y); let u = kind(net, y); let b = new_node(net, t, u);
    let t = enter(net, port(x, 1)); link(net, port(b, 0), t);
    let t = enter(net, port(x, 2)); link(net, port(y, 0), t);
    let t = enter(net, port(y, 1)); link(net, port(a, 0), t);
    let t = enter(net, port(y, 2)); link(net, port(x, 0), t);
    link(net, port(a, 1), port(b, 1));
    link(net, port(a, 2), port(y, 1));
    link(net, port(x, 1), port(b, 2));
    link(net, port(x, 2), port(y, 2));
}

pub fn dup_fn1(net : &mut Net, x : Port, y : Port) {
    let z = new_node(net, FN1, 0);
    let n = node(enter(net, port(y, 1)));
    let m = new_node(net, NUM, 0);
    net.nodes[(m * 4 + 1) as usize] = net.nodes[(n * 4 + 1) as usize];
    net.nodes[(m * 4 + 2) as usize] = net.nodes[(n * 4 + 2) as usize];
    link(net, port(z, 1), port(m, 0));
    let t = enter(net, port(x, 1)); link(net, t, port(y, 0));
    let t = enter(net, port(x, 2)); link(net, t, port(z, 0));
    let t = enter(net, port(y, 2)); link(net, t, port(x, 0));
    link(net, port(x, 1), port(y, 2));
    link(net, port(x, 2), port(z, 2));
}

pub fn word(net : &mut Net, x : Port) -> u64 {
    ((net.nodes[(x * 4 + 1) as usize] as u64) << 32) | net.nodes[(x * 4 + 2) as usize] as u64
}

pub fn dup_num(net : &mut Net, x : Port, y : Port) {
    let z = new_node(net, NUM, 0);
    net.nodes[(z * 4 + 1) as usize] = net.nodes[(y * 4 + 1) as usize];
    net.nodes[(z * 4 + 2) as usize] = net.nodes[(y * 4 + 2) as usize];
    let t = enter(net, port(x, 1)); link(net, t, port(y, 0));
    let t = enter(net, port(x, 2)); link(net, t, port(z, 0));
    net.reuse.push(x);
}

pub fn use_fn2(net : &mut Net, x : Port, y : Port) {
    let t = enter(net, port(x, 1));
    link(net, t, port(x, 0));
    link(net, port(x, 1), port(y, 0));
    net.nodes[(x * 4 + 3) as usize] = (FN1 << 30) | kind(net, x);
}

pub fn use_fn1(net : &mut Net, x : Port, y : Port) {
    let m = node(enter(net, port(x, 1)));
    let a = ((net.nodes[(m * 4 + 1) as usize] as u64) << 32) | net.nodes[(m * 4 + 2) as usize] as u64;
    let b = ((net.nodes[(y * 4 + 1) as usize] as u64) << 32) | net.nodes[(y * 4 + 2) as usize] as u64;
    let c = match kind(net, x) {
        ADD => a + b,
        MUL => a * b,
        SUB => a - b,
        DIV => a / b,
        _   => b
    };
    let t = enter(net, port(x, 2));
    link(net, t, port(y, 0));
    net.nodes[(y * 4 + 1) as usize] = (c >> 32) as u32;
    net.nodes[(y * 4 + 2) as usize] = c as u32;
    net.reuse.push(x);
    net.reuse.push(m);
}

pub fn rewrite(net : &mut Net, x : Port, y : Port) {
    if      tipo(net, x) == FN2 && tipo(net, y) == NUM { use_fn2(net, x, y); }
    else if tipo(net, x) == NUM && tipo(net, y) == FN2 { use_fn2(net, y, x); }
    else if tipo(net, x) == FN1 && tipo(net, y) == NUM { use_fn1(net, x, y); }
    else if tipo(net, x) == NUM && tipo(net, y) == FN1 { use_fn1(net, y, x); }
    else if tipo(net, x) == FN1 && tipo(net, y) == CON { dup_fn1(net, y, x); }
    else if tipo(net, x) == CON && tipo(net, y) == FN1 { dup_fn1(net, x, y); }
    else if tipo(net, x) == NUM && tipo(net, y) == CON { dup_num(net, y, x); }
    else if tipo(net, x) == CON && tipo(net, y) == NUM { dup_num(net, x, y); }
    else if kind(net, x) == kind(net, y)               { ani_two(net, x, y); }
    else                                               { dup_two(net, x, y); }
}
