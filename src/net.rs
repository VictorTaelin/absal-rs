#![allow(dead_code)]

extern crate rand;

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
    pub index: u32,
    pub alloc: Vec<u64>,
    pub nodes: Vec<u32>
}

pub type Port = u32;

pub fn new_net(allocs : u32) -> Net {
    return Net {
        index: 0,
        alloc: vec![0xFFFFFFFFFFFFFFFF; allocs as usize],
        nodes: vec![0; (allocs*4*4) as usize]
    };
}


pub fn hash(k: u64) -> u64 {
    const C1: u64 = 0xff51afd7ed558ccd;
    const C2: u64 = 0xc4ceb9fe1a85ec53;
    const R: u64 = 33;
    let mut tmp = k;
    tmp ^= tmp >> R;
    tmp = tmp.wrapping_mul(C1);
    tmp ^= tmp >> R;
    tmp = tmp.wrapping_mul(C2);
    tmp ^= tmp >> R;
    tmp
}

fn cmpxchg(vec : &mut Vec<u64>, i : u32, v0 : u64, v1 : u64) -> u64 {
    let old = vec[i as usize];
    if old == v0 { vec[i as usize] = v1; }
    return old;
}

pub fn alloc(net : &mut Net, seed : u64) -> u32 {
    let l = net.alloc.len() as u32;
    let h = hash(seed);
    let mut i = (h as u32) % l;
    loop {
        let k = cmpxchg(&mut net.alloc, i, 0xFFFFFFFFFFFFFFFF, h);
        if k == 0xFFFFFFFFFFFFFFFF || k == h {
            return i * 4;
        }
        i = (i + 1) % l;
    }
}

pub fn new_node(net : &mut Net, kind : u32) -> u32 {
    net.index += 1;
    let seed = net.index;
    let node = alloc(net, seed as u64);
    net.nodes[port(node, 0) as usize] = port(node, 0);
    net.nodes[port(node, 1) as usize] = port(node, 1);
    net.nodes[port(node, 2) as usize] = port(node, 2);
    net.nodes[port(node, 3) as usize] = kind << 2;
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

pub fn is_wire(net : &Net, node : Port) -> bool {
    return net.nodes[(node*4+3) as usize] & 1 == 1;
}

pub fn enter(net : &Net, next_ : Port) -> Port {
    let mut next = net.nodes[next_ as usize];
    while is_wire(net, node(next)) {
        next = net.nodes[next as usize];
    }
    return next;
}

pub fn kind(net : &Net, node : u32) -> u32 {
    net.nodes[port(node, 3) as usize] >> 2
}

pub fn link(net : &mut Net, ptr_a : u32, ptr_b : u32) {
    net.nodes[ptr_a as usize] = ptr_b;
    net.nodes[ptr_b as usize] = ptr_a;
}

pub fn rewrite(net : &mut Net, a : Port, b : Port) {
    let p : u32 = net.nodes[port(a,1) as usize];
    let q : u32 = net.nodes[port(a,2) as usize];
    let r : u32 = net.nodes[port(b,1) as usize];
    let s : u32 = net.nodes[port(b,2) as usize];
    if kind(net,a) == kind(net,b) {
        //  Q[ A2]   [ A1]P         Q[ A2]<,  ,>[ A1]P  
        //     |       |               v   |  |   v     
        // A2[ Q ]---[ P ]A1       A2[ S ]  \/  [ R ]A1 
        //      \  A  /                |    /\    |     
        //       [ B0]A0                \  |  |  /      
        //         |                     \/    \/       
        //         |                     /\    /\       
        //       [ A0]B0                /  |  |  \      
        //      /  B  \                |    \/    |     
        // B1[ R ]---[ S ]B2       B1[ P ]  /\  [ Q ]B2 
        //     |       |               ^   |  |   ^     
        //  R[ B1]   [ B2]S         R[ B1]<'  '>[ B2]S  
        net.nodes[port(a,0) as usize] = port(a,0);
        net.nodes[port(a,1) as usize] = r;
        net.nodes[port(a,2) as usize] = s;
        net.nodes[port(b,0) as usize] = port(b,0);
        net.nodes[port(b,1) as usize] = p;
        net.nodes[port(b,2) as usize] = q;
    } else {
        // Performs duplications
        //                              Q[ A2]<-,     ,->[ A1]P       
        //                                 v    |     |    v          
        //  Q[ A2]   [ A1]P            A2[Bl0]  |     |  [Br0]A1      
        //     |       |                   v    |     |    v          
        // A2[ Q ]---[ P ]A1          Bl0[ Q ]--'     '--[ P ]Br0     
        //      \  A  /                 /  Bl \Bl2   Br1/  Br \       
        //       [ B0]A0          Bl1[Al2]---[Ar2]   [Al1]---[Ar1]Br2 
        //         |                   |          \ /          |      
        //         |                   |          / \          |      
        //       [ A0]B0          Al2[Bl1]---[Br1]   [Bl2]---[Br2]Ar1 
        //      /  B  \                 \  Al /Al1   Ar2\  Ar /       
        // B1[ R ]---[ S ]B2          Al0[ R ]--,      ,--[ S ]Ar0    
        //     |       |                   ^    |      |    ^         
        //  R[ B1]   [ B2]S            B1[Al0]  |      |  [Ar0]B2     
        //                                 ^    |      |    ^         
        //                              R[ B1]<-'      '->[ B2]S      
        //net.index += 1;
        //let k  : u32 = net.index;
        let k  : u64 = ((a as u64) << 32) + b as u64;
        let i  : u32 = alloc(net, k);
        let al : u32 = i + 0;
        let ar : u32 = i + 1;
        let bl : u32 = i + 2;
        let br : u32 = i + 3;
        net.nodes[port(a,0) as usize]  = port(a,0);
        net.nodes[port(a,1) as usize]  = port(br,0);
        net.nodes[port(a,2) as usize]  = port(bl,0);
        net.nodes[port(b,0) as usize]  = port(b,0);
        net.nodes[port(b,1) as usize]  = port(al,0);
        net.nodes[port(b,2) as usize]  = port(ar,0);
        net.nodes[port(al,0) as usize] = r;
        net.nodes[port(al,1) as usize] = port(br,1);
        net.nodes[port(al,2) as usize] = port(bl,1);
        net.nodes[(al*4+3) as usize]   = kind(net,a) * 4;
        net.nodes[port(ar,0) as usize] = s;
        net.nodes[port(ar,1) as usize] = port(br,2);
        net.nodes[port(ar,2) as usize] = port(bl,2);
        net.nodes[(ar*4+3) as usize]   = kind(net,a) * 4;
        net.nodes[port(bl,0) as usize] = q;
        net.nodes[port(bl,1) as usize] = port(al,2);
        net.nodes[port(bl,2) as usize] = port(ar,2);
        net.nodes[(bl*4+3) as usize]   = kind(net,b) * 4;
        net.nodes[port(br,0) as usize] = p;
        net.nodes[port(br,1) as usize] = port(al,1);
        net.nodes[port(br,2) as usize] = port(ar,1);
        net.nodes[(br*4+3) as usize]   = kind(net,b) * 4;
    }
}
