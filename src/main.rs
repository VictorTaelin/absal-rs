//extern crate absal;

//fn main() {
    //// Parses the following λ-program:
    ////   two = λf. λx. f (f x)
    ////   exp = λn. λm. m n
    ////   exp two two
    //let (stats, code) = absal::reduce("/// #f #x /f /f /f x #f #x /f /f /f x #x x #x x");
    //println!("{:?}", stats);
    //println!("{}", code);
//}

extern crate ocl;
mod term;
mod net;
//extern crate absal;

fn trivial() -> ocl::Result<()> {
    use std::ffi::CString;
    use ocl::{core, flags};
    use ocl::enums::ArgVal;
    use ocl::builders::ContextProperties;

    let src = r#"
        typedef unsigned long u64;

        u64 port(u64,u64);
        u64 node(u64);
        u64 slot(u64);
        u64 isWire(__global u64*, u64);
        u64 enterPort(__global u64*, u64);
        u64 kind(__global u64*, u64);
        void link(__global u64*, u64, u64);
        void push(__global u64*, u64);
        void pushBit(u64*, u64);
        u64 popBit(u64*);
        void rewrite(__global u64*, u64, u64);

        u64 port(u64 node, u64 slot) {
            return (node << 2) | slot;
        }

        u64 node(u64 port) {
            return port >> 2;
        }

        u64 slot(u64 port) {
            return port & 3;
        }

        u64 isWire(__global u64* nodes, u64 node) {
            return nodes[port(node,0)] == port(node,0);
        }

        u64 enterPort(__global u64* nodes, u64 next) {
            do { next = nodes[next]; } while (isWire(nodes, node(next)));
            return next;
        }

        u64 kind(__global u64* nodes, u64 node) {
            return nodes[node * 4 + 3] / 4;
        }

        void link(__global u64* nodes, u64 a, u64 b) { 
            nodes[a] = b;
            nodes[b] = a;
        }

        void pushBit(u64* arr, u64 bit) {
            u64 l = arr[0]++;
            u64 i = 1 + (l >> 5);
            u64 j = l & 31;
            u64 m = bit << j;
            u64 x = arr[i];
            arr[i] = (x & (~m)) | m;
        }

        u64 popBit(u64* arr) {
            u64 l = --arr[0];
            u64 i = 1 + (l >> 5);
            u64 j = l & 31;
            u64 x = arr[i];
            arr[i] = x & (~(1 << j));
            return (x >> j) & 1;
        }

        void push(__global u64* arr, u64 val) {
            u64 idx = atomic_inc(arr) + 1;
            arr[idx] = val;
        }

        void rewrite(__global u64* nodes, u64 A, u64 B) {
            u64 P = nodes[port(A,1)];
            u64 Q = nodes[port(A,2)];
            u64 R = nodes[port(B,1)];
            u64 S = nodes[port(B,2)];
            if (kind(nodes,A) == kind(nodes,B)) {
                printf("AKI %d %d, %d %d, %d %d, %d %d, %d %d, %d %d\n",
                    port(A,0), port(A,0),
                    port(A,1), R,
                    port(A,2), S,
                    port(B,0), port(B,0),
                    port(B,1), P,
                    port(B,2), Q
                );
                nodes[port(A,0)] = port(A,0);
                nodes[port(A,1)] = R;
                nodes[port(A,2)] = S;
                nodes[port(B,0)] = port(B,0);
                nodes[port(B,1)] = P;
                nodes[port(B,2)] = Q;
            } else {
                //u64 L = nodes.length / 4;
                //u64 Al = L + 0;
                //u64 Ar = L + 1;
                //u64 Bl = L + 2;
                //u64 Br = L + 3;
                //nodes[port(A,0)]  = port(A,0);
                //nodes[port(A,1)]  = port(Br,0);
                //nodes[port(A,2)]  = port(Bl,0);
                //nodes[port(B,0)]  = port(B,0);
                //nodes[port(B,1)]  = port(Al,0);
                //nodes[port(B,2)]  = port(Ar,0);
                //nodes[port(Al,0)] = R;
                //nodes[port(Al,1)] = port(Br,1);
                //nodes[port(Al,2)] = port(Bl,1);
                //nodes[Al*4+3]     = kind(nodes,A) * 4;
                //nodes[port(Ar,0)] = S;
                //nodes[port(Ar,1)] = port(Br,2);
                //nodes[port(Ar,2)] = port(Bl,2);
                //nodes[Ar*4+3]     = kind(nodes,A) * 4;
                //nodes[port(Bl,0)] = Q;
                //nodes[port(Bl,1)] = port(Al,2);
                //nodes[port(Bl,2)] = port(Ar,2);
                //nodes[Bl*4+3]     = kind(nodes,B) * 4;
                //nodes[port(Br,0)] = P;
                //nodes[port(Br,1)] = port(Al,1);
                //nodes[port(Br,2)] = port(Ar,1);
                //nodes[Br*4+3]     = kind(nodes,B) * 4;
            }
        }

        void print(__global u64* arr, u64 len) {
            printf("[");
            for (u64 i = 0; i < len; ++i) {
                if (i > 0) printf(",");
                printf("%d", arr[i]);
            }
            printf("]\n");
        }

        __kernel void reduce(__global u64* bots_in, __global u64* bots_out, __global u64* nodes) {
            u64 exit[64];
            for (u64 i = 0; i < 64; ++i) {
                exit[i] = 0;
            }
            u64 gidx = get_global_id(0);
            u64 prev = bots_in[gidx + 1];
            if (prev == 4294967295) return;
            u64 next, A, B;
            while ((next = enterPort(nodes, prev))) {
                A = node(prev);
                B = node(next);
                if (!slot(next) && !slot(prev) && node(prev)) {
                    prev = enterPort(nodes, port(A, popBit(exit) + 1));
                    rewrite(nodes, A, B);
                } else if (slot(next) == 0) {
                    push(bots_out, port(B,1));
                    push(bots_out, port(B,2));
                    break;
                } else {
                    pushBit(exit, slot(next) - 1);
                    prev = port(node(next),0);
                }
            }
        }

        __kernel void set(__global u64* arr, u64 idx, u64 val) {
            arr[idx] = val;
        }
    "#;


    let code = b"/ #f #x /f /f x #f #x /f /f x";
    let code = b"/ #a #t /t a #x x";
    let mut net = term::to_net(&term::from_string(code));
    println!("{:?}", net);
    //net::reduce(&mut net);
    //println!("Result: {} -- {:?}", &term::from_net(&net), net.nodes);

    // Initial setup
    let platform_id = core::default_platform()?;
    let device_ids = core::get_device_ids(&platform_id, None, None)?;
    let device_id = device_ids[0];
    let context_properties = ContextProperties::new().platform(platform_id);
    let context = core::create_context(Some(&context_properties), &[device_id], None, None)?;
    let src_cstring = CString::new(src)?;
    let program = core::create_program_with_source(&context, &[src_cstring])?;
    core::build_program(&program, Some(&[device_id]), &CString::new("")?, None, None)?;
    let queue = core::create_command_queue(&context, &device_id, None)?;

    // Create buffers
    let nodes_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, net.nodes.len(), Some(&net.nodes))?  };
    let bots_len = 256;
    let mut bots_a : Vec<u64> = vec![4294967295; bots_len];
    let bots_a_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, bots_len, Some(&bots_a))?  };
    let mut bots_b : Vec<u64> = vec![4294967295; bots_len];
    let bots_b_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, bots_len, Some(&bots_b))?  };

    println!("nodes  : {:?}", net.nodes);
    println!("bots_a : {:?}", bots_a);
    println!("bots_b : {:?}", bots_b);
    println!("term   : {}", term::from_net(&net));

    // Create kernels
    let reduce_kernel = core::create_kernel(&program, "reduce")?;
    let set_kernel = core::create_kernel(&program, "set")?;

    // Main loop
    unsafe { 
        core::set_kernel_arg(&set_kernel, 0, ArgVal::mem(&bots_a_buf));
        core::set_kernel_arg(&set_kernel, 1, ArgVal::scalar(&1));
        core::set_kernel_arg(&set_kernel, 2, ArgVal::scalar(&0));
        core::enqueue_kernel(&queue, &set_kernel, 1, None, &[1,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;

        for i_ in 0..1 {
            core::set_kernel_arg(&set_kernel, 0, ArgVal::mem(&bots_b_buf));
            core::set_kernel_arg(&set_kernel, 1, ArgVal::scalar(&0));
            core::set_kernel_arg(&set_kernel, 2, ArgVal::scalar(&0));
            core::enqueue_kernel(&queue, &set_kernel, 1, None, &[1,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;

            core::set_kernel_arg(&reduce_kernel, 0, ArgVal::mem(&bots_a_buf))?;
            core::set_kernel_arg(&reduce_kernel, 1, ArgVal::mem(&bots_b_buf))?;
            core::set_kernel_arg(&reduce_kernel, 2, ArgVal::mem(&nodes_buf))?;
            core::enqueue_kernel(&queue, &reduce_kernel, 1, None, &[1,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;

            core::set_kernel_arg(&set_kernel, 0, ArgVal::mem(&bots_a_buf));
            core::set_kernel_arg(&set_kernel, 1, ArgVal::scalar(&0));
            core::set_kernel_arg(&set_kernel, 2, ArgVal::scalar(&0));
            core::enqueue_kernel(&queue, &set_kernel, 1, None, &[1,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;

            core::set_kernel_arg(&reduce_kernel, 0, ArgVal::mem(&bots_b_buf))?;
            core::set_kernel_arg(&reduce_kernel, 1, ArgVal::mem(&bots_a_buf))?;
            core::set_kernel_arg(&reduce_kernel, 2, ArgVal::mem(&nodes_buf))?;
            core::enqueue_kernel(&queue, &reduce_kernel, 1, None, &[1,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        }
    }

    // Readback
    unsafe {
        core::enqueue_read_buffer(&queue, &nodes_buf, true, 0, &mut net.nodes, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &bots_a_buf, true, 0, &mut bots_a, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &bots_b_buf, true, 0, &mut bots_b, None::<core::Event>, None::<&mut core::Event>)?;
    }

    // Print
    println!("nodes  : {:?}", net.nodes);
    println!("bots_a : {:?}", bots_a);
    println!("bots_b : {:?}", bots_b);
    println!("term   : {}", term::from_net(&net));

    Ok(())
}

fn main () {

    let code = b"/ #f #x /f /f x #f #x /f /f x";
    let code = b"/ #a a #a a";
    let code = b"@A #f #x /f /f /f /f x @B #f #x /f /f /f /f x //#a #b //#c #d ///c #e #f #g //g /e /#h #i #j #k /i ///h i j k f /e /#h #i #j #k /j ///h i j k f d #e #f #g g a //#c #d /c /c /c d b #c ///c #d #e #f #g /e ///d e f g #d #e #f #g /f ///d e f g #d #e #f f A B";
    let mut net = term::to_net(&term::from_string(code));
    let stats = net::reduce(&mut net);
    //println!("{:?}", net.nodes);
    println!("{:?}", stats);
    println!("{}", term::from_net(&net));


    //let net = 

    //let mut v = vec![0];
    //for i in 0..65535000000 { inc(&mut v); }
    //println!("{}", v[0]);

    //match trivial() { Ok(_v) => (), Err(e) => { println!("{}",e); } }

}
