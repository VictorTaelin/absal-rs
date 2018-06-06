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

pub type Port = u32;

pub fn new_node(net : &mut Net, kind : u32) -> u32 {
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
    net.nodes[port(node, 3) as usize] = kind;
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
    net.nodes[port(node, 3) as usize]
}

pub fn meta(net : &Net, node : u32) -> u32 {
    net.nodes[port(node, 3) as usize] & 3
}

pub fn set_meta(net : &mut Net, node : u32, meta : u32) {
    let ptr = port(node, 3) as usize;
    net.nodes[ptr] = net.nodes[ptr] & 0xFFFFFFFC | meta;
}

pub fn link(net : &mut Net, ptr_a : u32, ptr_b : u32) {
    net.nodes[ptr_a as usize] = ptr_b;
    net.nodes[ptr_b as usize] = ptr_a;
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

pub fn rewrite(net : &mut Net, x : Port, y : Port) {
    if kind(net, y) == 0xFFFFFFFF {
        panic!();
        //let a = new_node(net, 0xFFFFFFFF);
        //net.nodes[a * 4 + 1] = net.nodes[
    } else if kind(net, x) == kind(net, y) {
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
