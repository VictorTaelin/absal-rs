extern crate ocl;
mod term;
mod net;

fn redexes(net : &net::Net) -> Vec<u32> {
    let mut ports : Vec<u32> = vec![];
    for i in 0..net.nodes.len()/4 {
        let a = net::port(i as u32, 0);
        let b = net::enter(&net, a);
        if net::slot(b) == 0 && net::enter(&net, b) == a {
            if a < b {
                ports.push(a);
            }
        }
    }
    return ports;
}

fn trivial() -> ocl::Result<()> {
    use std::ffi::CString;
    use ocl::{core, flags};
    use ocl::enums::ArgVal;
    use ocl::builders::ContextProperties;

    let src = r#"
        typedef uint  u32;
        typedef uint4 q32;

        u32 hash(u32);
        u32 hash(u32 h) {
            h ^= h >> 16;
            h *= 0x85ebca6b;
            h ^= h >> 13;
            h *= 0xc2b2ae35;
            h ^= h >> 16;
            return h;
        }

        u32 insert(u32, __global u32*, u32);
        u32 insert(u32 set_len, __global u32* set_buf, u32 val) {
            for (u32 i = hash(val) % set_len; 1; i = (i + 1) % set_len) {
                u32 v = atomic_cmpxchg(&set_buf[i], 0xFFFFFFFF, val);
                if (v == 0xFFFFFFFF || v == val) {
                    //printf("%u: inserted at %u: %u\n", get_global_id(0), i, val);
                    return i;
                }
            }
        }

        u32 alloc(u32, __global u32*, u32);
        u32 alloc(u32 nodes_len, __global u32* nodes_buf, u32 idx) {
            for (u32 i = idx % (nodes_len/4); 1; i = (i + 1) % (nodes_len/4)) {
                u32 v = atomic_cmpxchg(&nodes_buf[i*4], 0xFFFFFFFF, i*4);
                if (v == 0xFFFFFFFF) {
                    return i;
                }
            }
        }

        u32 port(u32, u32);
        u32 port(u32 node, u32 slot) {
            return (node << 2) | slot;
        }

        u32 node(u32);
        u32 node(u32 port) {
            return port >> 2;
        }

        u32 slot(u32);
        u32 slot(u32 port) {
            return port & 3;
        }

        void redex(u32, __global u32*, u32, __global u32*, u32, __global u32*, u32, __global u32*);
        __kernel void redex
            (          u32  redex_len
            , __global u32* redex_buf
            ,          u32  visit_len
            , __global u32* visit_buf
            ,          u32  nodes_len
            , __global u32* nodes_buf
            ,          u32  stats_len
            , __global u32* stats_buf
            ) {
            __global q32* nodes_vec_buf = (__global q32*)nodes_buf;
            u32 i = get_global_id(0);
            u32 A = redex_buf[i];
            if (A != 0xFFFFFFFF) {
                atomic_inc(stats_buf);
                u32 B = nodes_buf[port(node(A),0)]; 
                q32 a = nodes_vec_buf[node(A)];
                //printf("%u: redex %u:%u - %u:%u\n", i, node(A), slot(A), node(B), slot(B));
                q32 b = nodes_vec_buf[node(B)];
                if (a.w >> 2 == b.w >> 2) {
                    //printf("ani\n");
                    nodes_vec_buf[node(A)] = (q32)(node(A)*4, b.y, b.z, 0xFFFFFFFF);
                    nodes_vec_buf[node(B)] = (q32)(node(B)*4, a.y, a.z, 0xFFFFFFFF);
                } else {
                    //printf("dup\n");
                    u32 IX = hash(A ^ B) % (nodes_len/4);
                    u32 L0 = alloc(nodes_len, nodes_buf, IX + 0);
                    u32 L1 = alloc(nodes_len, nodes_buf, IX + 1);
                    u32 L2 = alloc(nodes_len, nodes_buf, IX + 2);
                    u32 L3 = alloc(nodes_len, nodes_buf, IX + 3);
                    //printf("%u allocs %u %u %u %u\n", i, L0, L1, L2, L3);
                    nodes_vec_buf[L0] = (q32)(b.y, port(L3,1), port(L2,1), a.w);
                    nodes_vec_buf[L1] = (q32)(b.z, port(L3,2), port(L2,2), a.w);
                    nodes_vec_buf[L2] = (q32)(a.z, port(L0,2), port(L1,2), b.w);
                    nodes_vec_buf[L3] = (q32)(a.y, port(L0,1), port(L1,1), b.w);
                    nodes_vec_buf[node(A)] = (q32)(node(A)*4, port(L3,0), port(L2,0), 0xFFFFFFFF);
                    nodes_vec_buf[node(B)] = (q32)(node(B)*4, port(L0,0), port(L1,0), 0xFFFFFFFF);
                }
                insert(visit_len, visit_buf, a.y);
                insert(visit_len, visit_buf, a.z);
                insert(visit_len, visit_buf, b.y);
                insert(visit_len, visit_buf, b.z);
                //printf("%u: to visit %u:%u %u:%u %u:%u %u:%u\n", i, node(a.y), slot(a.y), node(a.z), slot(a.z), node(b.y), slot(b.y), node(b.z), slot(b.z));
                redex_buf[i] = 0xFFFFFFFF;
            }
        }

        u32 enter(u32, __global u32*, u32);
        u32 enter(u32 nodes_len, __global u32* nodes_buf, u32 A) {
            u32 I = A;
            while (A != 0xFFFFFFFF && nodes_buf[node(A) * 4 + 3] == 0xFFFFFFFF) { // it is a wire
                nodes_buf[node(A) * 4 + 0] = 0xFFFFFFFF; // free its space for future allocations
                A = nodes_buf[A]; // moves on 
                if (A == I) {
                    return 0xFFFFFFFF;
                }
            }
            return A;
        }

        void visit(u32, __global u32*, u32, __global u32*, u32, __global u32*, u32, __global u32*);
        __kernel void visit
            (          u32  redex_len
            , __global u32* redex_buf
            ,          u32  visit_len
            , __global u32* visit_buf
            ,          u32  nodes_len
            , __global u32* nodes_buf
            ,          u32  stats_len
            , __global u32* stats_buf
            ) {
            u32 i = get_global_id(0);
            u32 A = enter(nodes_len, nodes_buf, visit_buf[i]);
            if (A != 0xFFFFFFFF) {
                u32 B = enter(nodes_len, nodes_buf, nodes_buf[A]);
                nodes_buf[A] = B;
                nodes_buf[B] = A;
                if (slot(A) == 0 && slot(B) == 0 && node(A) != 0 && node(B) != 0) {
                    insert(redex_len, redex_buf, A < B ? A : B);
                    //printf("%d: inserted redex\n", i);
                }
            }
            visit_buf[i] = 0xFFFFFFFF;
        }

        void clear(__global u32*);
        __kernel void clear(__global u32* buf) {
            buf[get_global_id(0)] = 0xFFFFFFFF;
        }
    "#;


    // Initial setup
    let platform_id = core::default_platform()?;
    let device_ids = core::get_device_ids(&platform_id, None, None)?;
    let device_id = device_ids[1];
    let context_properties = ContextProperties::new().platform(platform_id);
    let context = core::create_context(Some(&context_properties), &[device_id], None, None)?;
    let src_cstring = CString::new(src)?;
    let program = core::create_program_with_source(&context, &[src_cstring])?;
    core::build_program(&program, Some(&[device_id]), &CString::new("")?, None, None)?;
    let queue = core::create_command_queue(&context, &device_id, None)?;

    // Prints platform info
    println!("{:?}", device_ids);
    println!("Type: {:?}", core::get_device_info(device_id, core::DeviceInfo::Type));
    println!("Name: {:?}", core::get_device_info(device_id, core::DeviceInfo::Name));
    println!("Platform: {:?}", core::get_device_info(device_id, core::DeviceInfo::Platform));
    println!("Vendor: {:?}", core::get_device_info(device_id, core::DeviceInfo::Version));
    println!("Version: {:?}", core::get_device_info(device_id, core::DeviceInfo::Vendor));
    println!("Global mem size: {:?}", core::get_device_info(device_id, core::DeviceInfo::GlobalMemSize));
    println!("Local mem type: {:?}", core::get_device_info(device_id, core::DeviceInfo::LocalMemType));
    println!("Local mem size: {:?}", core::get_device_info(device_id, core::DeviceInfo::LocalMemSize));
    println!("Max workgroup size: {:?}", core::get_device_info(device_id, core::DeviceInfo::MaxWorkGroupSize));
    println!("Max compute units: {:?}", core::get_device_info(device_id, core::DeviceInfo::MaxComputeUnits));

    // Create buffers
    let code = b"@A #f #x /f /f /f /f /f /f /f /f /f /f /f x @B #f #x /f /f /f /f /f x //#a #b //#c #d ///c #e #f #g //g /e /#h #i #j #k /i ///h i j k f /e /#h #i #j #k /j ///h i j k f d #e #f #g g a //#c #d /c /c /c d b #c ///c #d #e #f #g /e ///d e f g #d #e #f #g /f ///d e f g #d #e #f f A B";
    //let code = b"@A #f #x /f /f x @B #f #x /f /f x //#a #b //#c #d ///c #e #f #g //g /e /#h #i #j #k /i ///h i j k f /e /#h #i #j #k /j ///h i j k f d #e #f #g g a //#c #d /c /c /c d b #c ///c #d #e #f #g /e ///d e f g #d #e #f #g /f ///d e f g #d #e #f f A B";
    let mut net = term::to_net(&term::from_string(code));

    // Creates buffers
    let redex_len = 256*256;
    let visit_len = 256*256*4;
    let nodes_len = 256*256*64;
    let stats_len = 5;
    let mut redex_vec : Vec<u32> = vec![0; redex_len];
    let mut visit_vec : Vec<u32> = vec![0; visit_len];
    let mut nodes_vec : Vec<u32> = vec![0; nodes_len];
    let mut stats_vec : Vec<u32> = vec![0; stats_len];
    let redex_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, redex_len * 4, None)?  };
    let visit_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, visit_len * 4, None)?  };
    let nodes_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, nodes_len * 4, None)?  };
    let stats_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, stats_len * 4, None)?  };

    // Initializes buffers
    let clear_kernel = core::create_kernel(&program, "clear")?;
    unsafe {
        core::set_kernel_arg(&clear_kernel, 0, ArgVal::mem(&redex_buf))?;
        core::enqueue_kernel(&queue, &clear_kernel, 1, None, &[redex_len as usize,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        core::set_kernel_arg(&clear_kernel, 0, ArgVal::mem(&visit_buf))?;
        core::enqueue_kernel(&queue, &clear_kernel, 1, None, &[visit_len as usize,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        core::set_kernel_arg(&clear_kernel, 0, ArgVal::mem(&nodes_buf))?;
        core::enqueue_kernel(&queue, &clear_kernel, 1, None, &[nodes_len as usize,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_write_buffer(&queue, &nodes_buf, true, 0, &net.nodes, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_write_buffer(&queue, &redex_buf, true, 0, &redexes(&net), None::<core::Event>, None::<&mut core::Event>)?;
    }

    // Prepares kernels
    let redex_kernel = core::create_kernel(&program, "redex")?;
    core::set_kernel_arg(&redex_kernel, 0, ArgVal::scalar(&redex_len))?;
    core::set_kernel_arg(&redex_kernel, 1, ArgVal::mem(&redex_buf))?;
    core::set_kernel_arg(&redex_kernel, 2, ArgVal::scalar(&visit_len))?;
    core::set_kernel_arg(&redex_kernel, 3, ArgVal::mem(&visit_buf))?;
    core::set_kernel_arg(&redex_kernel, 4, ArgVal::scalar(&nodes_len))?;
    core::set_kernel_arg(&redex_kernel, 5, ArgVal::mem(&nodes_buf))?;
    core::set_kernel_arg(&redex_kernel, 6, ArgVal::scalar(&stats_len))?;
    core::set_kernel_arg(&redex_kernel, 7, ArgVal::mem(&stats_buf))?;
    let visit_kernel = core::create_kernel(&program, "visit")?;
    core::set_kernel_arg(&visit_kernel, 0, ArgVal::scalar(&redex_len))?;
    core::set_kernel_arg(&visit_kernel, 1, ArgVal::mem(&redex_buf))?;
    core::set_kernel_arg(&visit_kernel, 2, ArgVal::scalar(&visit_len))?;
    core::set_kernel_arg(&visit_kernel, 3, ArgVal::mem(&visit_buf))?;
    core::set_kernel_arg(&visit_kernel, 4, ArgVal::scalar(&nodes_len))?;
    core::set_kernel_arg(&visit_kernel, 5, ArgVal::mem(&nodes_buf))?;
    core::set_kernel_arg(&visit_kernel, 6, ArgVal::scalar(&stats_len))?;
    core::set_kernel_arg(&visit_kernel, 7, ArgVal::mem(&stats_buf))?;

    // Prints state
    unsafe {
        core::enqueue_read_buffer(&queue, &redex_buf, true, 0, &mut redex_vec, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &visit_buf, true, 0, &mut visit_vec, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &nodes_buf, true, 0, &mut nodes_vec, None::<core::Event>, None::<&mut core::Event>)?;
        //println!("- Redex: {:?}", redex_vec);
        //println!("- Visit: {:?}", visit_vec);
        //println!("- Nodes: {:?}\n\n", nodes_vec);
        //println!("histo = [ {{nodes: {:?}, visit: {:?}}}", nodes_vec, visit_vec);
    }

    for i in 0..550 {
        unsafe {
            core::enqueue_kernel(&queue, &redex_kernel, 1, None, &[redex_len as usize,1,1], Some([256,1,1]), None::<core::Event>, None::<&mut core::Event>)?;
            core::enqueue_kernel(&queue, &visit_kernel, 1, None, &[visit_len as usize,1,1], Some([256,1,1]), None::<core::Event>, None::<&mut core::Event>)?;
            println!("# ROUND {}", i);
        }
    }


    unsafe {
        core::enqueue_read_buffer(&queue, &nodes_buf, true, 0, &mut nodes_vec, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &stats_buf, true, 0, &mut stats_vec, None::<core::Event>, None::<&mut core::Event>)?;
        net.nodes = nodes_vec;
        println!("{}", term::from_net(&net));
        println!("Stats: {:?}", stats_vec);
    }


    Ok(())
}

fn main () {
    match trivial() { Ok(_v) => (), Err(e) => { println!("{}",e); } }
}
