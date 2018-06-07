extern crate ocl;
mod term;
mod net;

fn next_mul(n : u32, m : u32) -> u32 {
    return if n == m { n } else { (n + m) - (n + m) % m };
}

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

        void push(u32, __global u32*, u32, u32, __global u32*, u32);
        void push(u32 stats_len, __global u32* stats_buf, u32 pos, u32 len, __global u32* buf, u32 val) {
            buf[atomic_inc(&stats_buf[pos])] = val;
        }

        u32 alloc(u32, __global u32*, u32, __global u32*);
        u32 alloc(u32 stats_len, __global u32* stats_buf, u32 reuse_len, __global u32* reuse_buf) {
            u32 i = atomic_dec(&stats_buf[6]); 
            return i > 0 && i < 0x80000000 ? reuse_buf[i - 1] : atomic_inc(&stats_buf[4]);
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

        void redex(u32, __global u32*, u32, __global u32*, u32, __global u32*, u32, __global u32*, u32, __global u32*);
        __kernel void redex
            (          u32  stats_len
            , __global u32* stats_buf
            ,          u32  redex_len
            , __global u32* redex_buf
            ,          u32  visit_len
            , __global u32* visit_buf
            ,          u32  nodes_len
            , __global u32* nodes_buf
            ,          u32  reuse_len
            , __global u32* reuse_buf
            ) {
            __global q32* nodes_vec_buf = (__global q32*)nodes_buf;
            u32 redex_siz = stats_buf[2];
            if (get_global_id(0) < redex_siz) {
                u32 A = redex_buf[get_global_id(0)];
                q32 a = nodes_vec_buf[node(A)];
                if (a.w != 0xFFFFFFFF && atomic_cmpxchg(&nodes_buf[node(A)*4+3], a.w, 0xFFFFFFFF) == a.w) {
                    atomic_inc(&stats_buf[5]);
                    u32 B = nodes_buf[port(node(A),0)]; 
                    q32 b = nodes_vec_buf[node(B)];
                    if (a.w != b.w) {
                        u32 L0 = alloc(stats_len, stats_buf, reuse_len, reuse_buf);
                        u32 L1 = alloc(stats_len, stats_buf, reuse_len, reuse_buf);
                        u32 L2 = alloc(stats_len, stats_buf, reuse_len, reuse_buf);
                        u32 L3 = alloc(stats_len, stats_buf, reuse_len, reuse_buf);
                        nodes_vec_buf[L0] = (q32)(b.y, port(L3,1), port(L2,1), a.w);
                        nodes_vec_buf[L1] = (q32)(b.z, port(L3,2), port(L2,2), a.w);
                        nodes_vec_buf[L2] = (q32)(a.z, port(L0,2), port(L1,2), b.w);
                        nodes_vec_buf[L3] = (q32)(a.y, port(L0,1), port(L1,1), b.w);
                        nodes_vec_buf[node(A)] = (q32)(node(A)*4, port(L3,0), port(L2,0), 0xFFFFFFFF);
                        nodes_vec_buf[node(B)] = (q32)(node(B)*4, port(L0,0), port(L1,0), 0xFFFFFFFF);
                        push(stats_len, stats_buf, 0, visit_len, visit_buf, a.y);
                        push(stats_len, stats_buf, 0, visit_len, visit_buf, a.z);
                        push(stats_len, stats_buf, 0, visit_len, visit_buf, b.y);
                        push(stats_len, stats_buf, 0, visit_len, visit_buf, b.z);
                    } else {
                        nodes_vec_buf[node(A)] = (q32)(node(A)*4, b.y, b.z, 0xFFFFFFFF);
                        nodes_vec_buf[node(B)] = (q32)(node(B)*4, a.y, a.z, 0xFFFFFFFF);
                        push(stats_len, stats_buf, 0, visit_len, visit_buf, a.y);
                        push(stats_len, stats_buf, 0, visit_len, visit_buf, a.z);
                    }
                }
                if (atomic_inc(&stats_buf[3]) == redex_siz - 1) {
                    stats_buf[2] = 0;
                    stats_buf[3] = 0;
                    stats_buf[6] = stats_buf[6] > 0x80000000 ? 0 : stats_buf[6];
                }
            }
        }

        void visit(u32, __global u32*, u32, __global u32*, u32, __global u32*, u32, __global u32*, u32, __global u32*);
        __kernel void visit
            (          u32  stats_len
            , __global u32* stats_buf
            ,          u32  redex_len
            , __global u32* redex_buf
            ,          u32  visit_len
            , __global u32* visit_buf
            ,          u32  nodes_len
            , __global u32* nodes_buf
            ,          u32  reuse_len
            , __global u32* reuse_buf
            ) {
            __global q32* nodes_vec_buf = (__global q32*)nodes_buf;
            u32 visit_siz = stats_buf[0];
            if (get_global_id(0) < visit_siz) {
                atomic_inc(&stats_buf[15]);

                if (atomic_inc(&stats_buf[1]) == visit_siz - 1) {
                    stats_buf[0] = 0;
                    stats_buf[1] = 0;
                }


                u32 A = nodes_buf[visit_buf[get_global_id(0)]];
                q32 a = nodes_vec_buf[node(A)];

                u32 I = A;
                while (a.w >= 0xFFFFFFFE) { // it is a wire
                    if (a.x != 0xFFFFFFFF && atomic_cmpxchg(&nodes_buf[node(A)*4+0], a.x, 0xFFFFFFFF) == a.x) {
                        push(stats_len, stats_buf, 6, reuse_len, reuse_buf, node(A));
                    }
                    A = nodes_buf[A];
                    a = nodes_vec_buf[node(A)];
                    if (A == I) { return; }
                }

                u32 B = nodes_buf[A];
                q32 b = nodes_vec_buf[node(B)];
                while (b.w >= 0xFFFFFFFE) { // it is a wire
                    if (b.x != 0xFFFFFFFF && atomic_cmpxchg(&nodes_buf[node(B)*4+0], b.x, 0xFFFFFFFF) == b.x) {
                        push(stats_len, stats_buf, 6, reuse_len, reuse_buf, node(B));
                    }
                    B = nodes_buf[B]; // moves on 
                    b = nodes_vec_buf[node(B)];
                }

                nodes_buf[A] = B;
                nodes_buf[B] = A;
                if (slot(A) == 0 && slot(B) == 0 && node(A) != 0 && node(B) != 0) {
                    push(stats_len, stats_buf, 2, redex_len, redex_buf, A < B ? A : B);
                }
            }
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
    let code = b"@A #f #x /f /f /f /f /f /f /f /f /f /f /f /f x @B #f #x /f /f /f /f x //#a #b //#c #d ///c #e #f #g //g /e /#h #i #j #k /i ///h i j k f /e /#h #i #j #k /j ///h i j k f d #e #f #g g a //#c #d /c /c /c d b #c ///c #d #e #f #g /e ///d e f g #d #e #f #g /f ///d e f g #d #e #f f A B";
    let mut net = term::to_net(&term::from_string(code));
    let group_len = match core::get_device_info(device_id, core::DeviceInfo::MaxWorkGroupSize) { Ok(ocl::enums::DeviceInfoResult::MaxWorkGroupSize(size)) => size, _ => 0 } as u32; // wtf
    let stats_len = 16;
    let visit_len = 256*256*16;
    let redex_len = visit_len / 4;
    let nodes_len = 256*256*64;
    let reuse_len = nodes_len / 4;
    let mut stats_vec : Vec<u32> = vec![0; stats_len];
    let mut redex_vec : Vec<u32> = vec![0; redex_len];
    let mut visit_vec : Vec<u32> = vec![0; visit_len];
    let mut nodes_vec : Vec<u32> = vec![0; nodes_len];
    let mut reuse_vec : Vec<u32> = vec![0; reuse_len];
    let stats_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, stats_len * 4, None)?  };
    let redex_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, redex_len * 4, None)?  };
    let visit_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, visit_len * 4, None)?  };
    let nodes_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, nodes_len * 4, None)?  };
    let reuse_buf = unsafe { core::create_buffer::<_,u32>(&context, flags::MEM_READ_WRITE, reuse_len * 4, None)?  };
    stats_vec[0] = 0;                            // visit len
    stats_vec[1] = 0;                            // visit done
    stats_vec[2] = redexes(&net).len() as u32;   // redex len
    stats_vec[3] = 0;                            // redex done
    stats_vec[4] = (net.nodes.len() as u32) / 4; // nodes len
    stats_vec[5] = 0;                            // rewrite count
    stats_vec[6] = 0;                            // reuse len

    // Initializes buffers
    let clear_kernel = core::create_kernel(&program, "clear")?;
    unsafe {
        core::set_kernel_arg(&clear_kernel, 0, ArgVal::mem(&redex_buf))?;
        core::enqueue_kernel(&queue, &clear_kernel, 1, None, &[redex_len as usize,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        core::set_kernel_arg(&clear_kernel, 0, ArgVal::mem(&visit_buf))?;
        core::enqueue_kernel(&queue, &clear_kernel, 1, None, &[visit_len as usize,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        core::set_kernel_arg(&clear_kernel, 0, ArgVal::mem(&nodes_buf))?;
        core::enqueue_kernel(&queue, &clear_kernel, 1, None, &[nodes_len as usize,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        core::set_kernel_arg(&clear_kernel, 0, ArgVal::mem(&reuse_buf))?;
        core::enqueue_kernel(&queue, &clear_kernel, 1, None, &[reuse_len as usize,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_write_buffer(&queue, &nodes_buf, true, 0, &net.nodes, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_write_buffer(&queue, &redex_buf, true, 0, &redexes(&net), None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_write_buffer(&queue, &stats_buf, true, 0, &stats_vec, None::<core::Event>, None::<&mut core::Event>)?;
    }

    // Prepares kernels
    let redex_kernel = core::create_kernel(&program, "redex")?;
    core::set_kernel_arg(&redex_kernel, 0, ArgVal::scalar(&stats_len))?;
    core::set_kernel_arg(&redex_kernel, 1, ArgVal::mem(&stats_buf))?;
    core::set_kernel_arg(&redex_kernel, 2, ArgVal::scalar(&redex_len))?;
    core::set_kernel_arg(&redex_kernel, 3, ArgVal::mem(&redex_buf))?;
    core::set_kernel_arg(&redex_kernel, 4, ArgVal::scalar(&visit_len))?;
    core::set_kernel_arg(&redex_kernel, 5, ArgVal::mem(&visit_buf))?;
    core::set_kernel_arg(&redex_kernel, 6, ArgVal::scalar(&nodes_len))?;
    core::set_kernel_arg(&redex_kernel, 7, ArgVal::mem(&nodes_buf))?;
    core::set_kernel_arg(&redex_kernel, 8, ArgVal::scalar(&reuse_len))?;
    core::set_kernel_arg(&redex_kernel, 9, ArgVal::mem(&reuse_buf))?;
    let visit_kernel = core::create_kernel(&program, "visit")?;
    core::set_kernel_arg(&visit_kernel, 0, ArgVal::scalar(&stats_len))?;
    core::set_kernel_arg(&visit_kernel, 1, ArgVal::mem(&stats_buf))?;
    core::set_kernel_arg(&visit_kernel, 2, ArgVal::scalar(&redex_len))?;
    core::set_kernel_arg(&visit_kernel, 3, ArgVal::mem(&redex_buf))?;
    core::set_kernel_arg(&visit_kernel, 4, ArgVal::scalar(&visit_len))?;
    core::set_kernel_arg(&visit_kernel, 5, ArgVal::mem(&visit_buf))?;
    core::set_kernel_arg(&visit_kernel, 6, ArgVal::scalar(&nodes_len))?;
    core::set_kernel_arg(&visit_kernel, 7, ArgVal::mem(&nodes_buf))?;
    core::set_kernel_arg(&visit_kernel, 8, ArgVal::scalar(&reuse_len))?;
    core::set_kernel_arg(&visit_kernel, 9, ArgVal::mem(&reuse_buf))?;

    // Performs computation
    for _i in 0..9999 {
        unsafe {
            core::enqueue_read_buffer(&queue, &stats_buf, true, 0, &mut stats_vec, None::<core::Event>, None::<&mut core::Event>)?;
            let redexes = stats_vec[2];
            core::enqueue_kernel(&queue, &redex_kernel, 1, None, &[next_mul(redexes,group_len) as usize,1,1], Some([group_len as usize,1,1]), None::<core::Event>, None::<&mut core::Event>)?;
            core::enqueue_read_buffer(&queue, &stats_buf, true, 0, &mut stats_vec, None::<core::Event>, None::<&mut core::Event>)?;
            let visits = stats_vec[0];
            core::enqueue_kernel(&queue, &visit_kernel, 1, None, &[next_mul(visits,group_len) as usize,1,1], Some([group_len as usize,1,1]), None::<core::Event>, None::<&mut core::Event>)?;
            //println!("pass {}: {} redexes, {} visits", i, redexes, visits); 
            if stats_vec[0] == 0 { break; }
        }
    }

    // Reads back results
    unsafe {
        core::enqueue_read_buffer(&queue, &stats_buf, true, 0, &mut stats_vec, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &redex_buf, true, 0, &mut redex_vec, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &visit_buf, true, 0, &mut visit_vec, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &nodes_buf, true, 0, &mut nodes_vec, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &reuse_buf, true, 0, &mut reuse_vec, None::<core::Event>, None::<&mut core::Event>)?;
        net.nodes = nodes_vec;
        //println!("{}", term::from_net(&net));
        //println!("{:?}", stats_vec);
        //println!("{:?}", nodes_vec.iter().map(|&x| if x == (0xFFFFFFFF as u32) { 0 } else { 1 }).collect::<Vec<u32>>());
        //println!("{:?}", visit_vec);
        //println!("{:?}", visit_vec);
        println!("Stats: {:?}", stats_vec);
    }


    Ok(())
}

fn main () {
    match trivial() { Ok(_v) => (), Err(e) => { println!("{}",e); } }
}
