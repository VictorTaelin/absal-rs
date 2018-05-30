//extern crate absal;
//Type: Ok(CPU)
//Name: Ok(Intel(R) Core(TM) i7-6567U CPU @ 3.30GHz)
//Platform: Ok(PlatformId(0x7fff0000))
//Vendor: Ok(1.2)
//Version: Ok(Intel)
//Global mem size: Ok(17179869184)
//Local mem type: Ok(Global)
//Local mem size: Ok(32768)
//Max workgroup size: Ok(1024)
//
//Type: Ok(GPU)
//Name: Ok(Intel(R) Iris(TM) Graphics 550)
//Platform: Ok(PlatformId(0x7fff0000))
//Vendor: Ok(1.2)
//Version: Ok(Intel Inc.)
//Global mem size: Ok(1610612736)
//Local mem type: Ok(Local)
//Local mem size: Ok(65536)
//Max workgroup size: Ok(256)

extern crate ocl;
mod term;
mod net;

fn trivial() -> ocl::Result<()> {
    use std::ffi::CString;
    use ocl::{core, flags};
    use ocl::enums::ArgVal;
    use ocl::builders::ContextProperties;

    let src = r#"
        typedef unsigned int u32;
        typedef unsigned long u64;

        void print(u32, __global u32*);
        u64 hash(u64);
        u64 alloc(u32, __global u64*, u64);
        u32 port(u32, u32);
        u32 node(u32);
        u32 slot(u32);
        void push(u32*, u32);
        u32 pop(u32*);
        u32 peek(u32*);
        void push4(uint4*, uint4);
        uint4 pop4(uint4*);
        uint4 peek4(uint4*);
        void enqueue_visit(u32, __global u32*, u32);
        u32 visit(u32, __global u32*);
        //void rewrite(u32, __global u64*, u32, __global u32*, u32, u32);
        void reduce(u32, __global u32*, u32, __global u64*, u32, __global u32*, u32, __global u32*);
        void gpush(__global u32*, u32 val);

        void print(u32 len, __global u32* arr) {
            printf("[");
            for (u32 i = 0; i < len; ++i) {
                if (i > 0) printf(",");
                printf("%d", arr[i]);
            }
            printf("]\n");
        }

        u64 hash(u64 k) {
            u64 C1 = 0xff51afd7ed558ccd;
            u64 C2 = 0xc4ceb9fe1a85ec53;
            u64 R = 33;
            k ^= k >> R;
            k *= C1;
            k ^= k >> R;
            k *= C2;
            k ^= k >> R;
            return k;
        }

        u64 alloc(u32 alloc_len, __global u64* alloc_buf, u64 seed) {
            u64 h = hash(seed);
            u64 i = (u32)h % alloc_len;
            while (1) {
                u64 k = atomic_cmpxchg(&alloc_buf[i], 0xFFFFFFFF, h);
                if (k == 0xFFFFFFFF) {
                    return ((i * 4) << 1) | 1;
                } else if (k == h) {
                    return ((i * 4) << 1);
                }
                i = (i + 1) % alloc_len;
            }
        }

        u32 port(u32 node, u32 slot) {
            return (node << 2) | slot;
        }

        u32 node(u32 port) {
            return port >> 2;
        }

        u32 slot(u32 port) {
            return port & 3;
        }

        void push(u32* stack, u32 val) {
            u32 i = ++stack[0];
            stack[i] = val;
        }

        u32 pop(u32* stack) {
            u32 l = stack[0]--;
            return stack[l];
        }

        u32 peek(u32* stack) {
            return stack[stack[0]];
        }

        void push4(uint4* stack, uint4 val) {
            u32 i = ++stack[0].x;
            stack[i] = val;
        }

        uint4 pop4(uint4* stack) {
            u32 l = stack[0].x--;
            return stack[l];
        }

        uint4 peek4(uint4* stack) {
            return stack[stack[0].x];
        }

        void gpush(__global u32* stack, u32 val) {
            u32 i = ++stack[0];
            stack[i] = val;
        }

        void enqueue_visit(u32 visit_len, __global u32* visit_buf, u32 port) {
            u32 i = get_global_id(0) % visit_len;
            while (1) {
                if (atomic_cmpxchg(&visit_buf[i], 0xFFFFFFFF, port) == 0xFFFFFFFF) {
                    return;
                }
                i = (i + 1) % visit_len;
            }
        }

        u32 visit(u32 visit_len, __global u32* visit_buf) {
            u32 k = 0;
            u32 i = get_global_id(0) % visit_len;
            while (++k < visit_len * 2) {
                u32 p = visit_buf[i];
                if (p != 0xFFFFFFFF && atomic_cmpxchg(&visit_buf[i], p, 0xFFFFFFFF) == p) {
                    return p;
                }
                i = (i + 1) % visit_len;
            }
            return 0xFFFFFFFF;
        }

        __kernel void reduce
            (          u32  visit_len
            , __global u32* visit_buf
            ,          u32  alloc_len
            , __global u64* alloc_buf
            ,          u32  nodes_len
            , __global u32* nodes_buf
            ,          u32  event_len
            , __global u32* event_buf
            ) {
            __global uint4* nodes_buf_vec = (__global uint4*)nodes_buf;

            // Initializes path stack
            u32 path[4096];
            for (u32 i = 0; i < 4096; ++i) {
                path[i] = 0;
            }
            uint4 memo[4096];
            for (u32 i = 0; i < 4096; ++i) {
                memo[i] = (uint4)(0,0,0,0);
            }

            // Gets node to visit
            u32 init = visit(visit_len, visit_buf);
            uint4 inin = nodes_buf_vec[node(init)];
            if (init == 0xFFFFFFFF) {
                //printf("%d: halt \n", get_global_id(0));
                return;
            }
            //printf("%d: visit | %d %d\n", get_global_id(0), node(init), slot(init));

            // Main loop
            u32 next = slot(init) == 0 ? inin.x : slot(init) == 1 ? inin.y : inin.z;
            while (1) {
                //printf("%d: to %d:%d\n", get_global_id(0), node(next), slot(next));

                // Finds node on next's direction
                uint4 b = nodes_buf_vec[node(next)];

                if (!next) {
                    break;

                } else if (b.w & 1) {
                    next = slot(next) == 0 ? (pop4(memo), pop(path)) : slot(next) == 1 ? b.y : b.z;

                // Next is the back port: move up
                } else if (slot(next) != 0) {
                    push(path, next);
                    //printf("!!%d\n", slot(next));
                    push4(memo, b);
                    next = b.x;

                // Next is the main port: reduce
                } else {
                    // Finds prev node
                    u32 prev = path[0] > 0 ? port(node(peek(path)), 0) : init;
                    uint4 a = path[0] > 0 ? peek4(memo) : inin;
                    u32 A = node(prev);
                    u32 B = node(next);

                    //printf("%d: active %d - %d | %d %d %d %d - %d %d %d %d | %d\n", get_global_id(0), node(prev), node(next), a.x, a.y, a.z, a.w, b.x, b.y, b.z, b.w, slot(next));

                    // Reached an active port: reduce it
                    if (slot(prev) == 0 && node(prev) != 0 && node(next) != 0) {
                        if (A > B) {
                            u32 C = A; A = B; B = C;
                            uint4 c = a; a = b; b = c;
                        }
                        if (a.w / 4 == b.w / 4) {
                            //printf("%d: BEG ANI | %d = %d:%d %d:%d %d:%d %d | %d = %d:%d %d:%d %d:%d %d \n", get_global_id(0), A, node(a.x), slot(a.x), node(a.y), slot(a.y), node(a.z), slot(a.z), a.w, B, node(b.x), slot(b.x), node(b.y), slot(b.y), node(b.z), slot(b.z), b.w);
                            nodes_buf_vec[A] = (uint4)(B*4, b.y, b.z, a.w|1);
                            nodes_buf_vec[B] = (uint4)(A*4, a.y, a.z, b.w|1);
                        } else {
                            u32 K = alloc(alloc_len, alloc_buf, (((u64)A << 32) + (u64)B));
                            u32 L = K >> 1;
                            //printf("%d: BEG DUP | %d = %d:%d %d:%d %d:%d %d | %d = %d:%d %d:%d %d:%d %d | ++%d %d %d %d\n", get_global_id(0), A, node(a.x), slot(a.x), node(a.y), slot(a.y), node(a.z), slot(a.z), a.w, B, node(b.x), slot(b.x), node(b.y), slot(b.y), node(b.z), slot(b.z), b.w, L+0, L+1, L+2, L+3);
                            //if (K & 1) {
                                //nodes_buf_vec[L+0] = (uint4)(b.y, port(L+3,1), port(L+2,1), a.w&0xFFFFFFFE);
                                //nodes_buf_vec[L+1] = (uint4)(b.z, port(L+3,2), port(L+2,2), a.w&0xFFFFFFFE);
                                //nodes_buf_vec[L+2] = (uint4)(a.z, port(L+0,2), port(L+1,2), b.w&0xFFFFFFFE);
                                //nodes_buf_vec[L+3] = (uint4)(a.y, port(L+0,1), port(L+1,1), b.w&0xFFFFFFFE);
                                //nodes_buf_vec[A+0] = (uint4)(B*4, port(L+3,0), port(L+2,0), a.w|1);
                                //nodes_buf_vec[B+0] = (uint4)(A*4, port(L+0,0), port(L+1,0), b.w|1);
                            //} else {
                                //printf("%d: waiting\n", get_global_id(0));
                                //uint4 k;
                                //k = nodes_buf_vec[L+0]; while (k.x == 0 && k.y == 0 && k.z == 0) {};
                                //k = nodes_buf_vec[L+1]; while (k.x == 0 && k.y == 0 && k.z == 0) {};
                                //k = nodes_buf_vec[L+2]; while (k.x == 0 && k.y == 0 && k.z == 0) {};
                                //k = nodes_buf_vec[L+3]; while (k.x == 0 && k.y == 0 && k.z == 0) {};
                                //while (!(nodes_buf_vec[A+0].w & 1)) {};
                                //while (!(nodes_buf_vec[B+0].w & 1)) {};
                                //printf("%d: waited\n", get_global_id(0));
                            //}

                            atomic_cmpxchg(&nodes_buf[(L+0)*4+0], 0, b.y);
                            atomic_cmpxchg(&nodes_buf[(L+0)*4+1], 0, port(L+3,1));
                            atomic_cmpxchg(&nodes_buf[(L+0)*4+2], 0, port(L+2,1));
                            atomic_cmpxchg(&nodes_buf[(L+0)*4+3], 0, a.w&0xFFFFFFFE);
                            atomic_cmpxchg(&nodes_buf[(L+1)*4+0], 0, b.z);
                            atomic_cmpxchg(&nodes_buf[(L+1)*4+1], 0, port(L+3,2));
                            atomic_cmpxchg(&nodes_buf[(L+1)*4+2], 0, port(L+2,2));
                            atomic_cmpxchg(&nodes_buf[(L+1)*4+3], 0, a.w&0xFFFFFFFE);
                            atomic_cmpxchg(&nodes_buf[(L+2)*4+0], 0, a.z);
                            atomic_cmpxchg(&nodes_buf[(L+2)*4+1], 0, port(L+0,2));
                            atomic_cmpxchg(&nodes_buf[(L+2)*4+2], 0, port(L+1,2));
                            atomic_cmpxchg(&nodes_buf[(L+2)*4+3], 0, b.w&0xFFFFFFFE);
                            atomic_cmpxchg(&nodes_buf[(L+3)*4+0], 0, a.y);
                            atomic_cmpxchg(&nodes_buf[(L+3)*4+1], 0, port(L+0,1));
                            atomic_cmpxchg(&nodes_buf[(L+3)*4+2], 0, port(L+1,1));
                            atomic_cmpxchg(&nodes_buf[(L+3)*4+3], 0, b.w&0xFFFFFFFE);
                            nodes_buf_vec[A+0] = (uint4)(B*4, port(L+3,0), port(L+2,0), a.w|1);
                            nodes_buf_vec[B+0] = (uint4)(A*4, port(L+0,0), port(L+1,0), b.w|1);

                            //printf("%d: END DUP\n", get_global_id(0));
                        }

                    // Reached a node on normal form: visit each direction
                    } else {
                        //printf("%d: enqueue %d %d\n", get_global_id(0), b.y, b.z);
                        enqueue_visit(visit_len, visit_buf, port(B,1));
                        enqueue_visit(visit_len, visit_buf, port(B,2));
                        break;
                    }
                }
            }
        }
    "#;


    // Initial setup
    let platform_id = core::default_platform()?;
    let device_ids = core::get_device_ids(&platform_id, None, None)?;
    let device_id = device_ids[0];
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
    let context_properties = ContextProperties::new().platform(platform_id);
    let context = core::create_context(Some(&context_properties), &[device_id], None, None)?;
    let src_cstring = CString::new(src)?;
    let program = core::create_program_with_source(&context, &[src_cstring])?;
    core::build_program(&program, Some(&[device_id]), &CString::new("")?, None, None)?;
    let queue = core::create_command_queue(&context, &device_id, None)?;

    // Create buffers
    let code = b"// #f #x /f /f /f /f x #x x #x x";
    let code = b"/ #f #x /f /f x #f #x /f /f x";
    let code = b"@A #f #x /f /f x @B #f #x /f /f x //#a #b //#c #d ///c #e #f #g //g /e /#h #i #j #k /i ///h i j k f /e /#h #i #j #k /j ///h i j k f d #e #f #g g a //#c #d /c /c /c d b #c ///c #d #e #f #g /e ///d e f g #d #e #f #g /f ///d e f g #d #e #f f A B";
    //let code = b"@A #f #x /f /f /f x @B #f #x /f /f /f x //#a #b //#c #d ///c #e #f #g //g /e /#h #i #j #k /i ///h i j k f /e /#h #i #j #k /j ///h i j k f d #e #f #g g a //#c #d /c /c /c d b #c ///c #d #e #f #g /e ///d e f g #d #e #f #g /f ///d e f g #d #e #f f A B";
    let mut net = term::to_net(&term::from_string(code));
    println!("{}", term::from_net(&net));
    //let stats = net::reduce(&mut net);
    //println!("{:?}", stats);
    //println!("{}", term::from_net(&net));
    //println!("{:?}", net.nodes);
    //return Ok(());

    let nodes_len = net.nodes.len();
    let alloc_len = net.alloc.len();
    let visit_len = net.visit.len(); net.visit[0] = 0;
    let event_len = 256 * 2; let mut event : Vec<u32> = vec![0; event_len];
    let nodes_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, nodes_len, Some(&net.nodes))?  };
    let alloc_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, alloc_len, Some(&net.alloc))?  };
    let visit_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, visit_len, Some(&net.visit))?  };
    let event_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, event_len, Some(&event))?  };

    //println!("nodes : {:?}", net.nodes);
    //println!("alloc : {:?}", net.alloc);
    //println!("visit : {:?}", net.visit);
    println!("term  : {}", term::from_net(&net));
    println!("-------|");

    // Create kernels
    let reduce_kernel = core::create_kernel(&program, "reduce")?;

    // Main loop
    unsafe { 
        //core::enqueue_kernel(&queue, &set_kernel, 1, None, &[1,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        
        for _i in 0..64 {
            core::set_kernel_arg(&reduce_kernel, 0, ArgVal::scalar(&visit_len))?;
            core::set_kernel_arg(&reduce_kernel, 1, ArgVal::mem(&visit_buf))?;
            core::set_kernel_arg(&reduce_kernel, 2, ArgVal::scalar(&alloc_len))?;
            core::set_kernel_arg(&reduce_kernel, 3, ArgVal::mem(&alloc_buf))?;
            core::set_kernel_arg(&reduce_kernel, 4, ArgVal::scalar(&nodes_len))?;
            core::set_kernel_arg(&reduce_kernel, 5, ArgVal::mem(&nodes_buf))?;
            core::set_kernel_arg(&reduce_kernel, 6, ArgVal::scalar(&event_len))?;
            core::set_kernel_arg(&reduce_kernel, 7, ArgVal::mem(&event_buf))?;
            core::enqueue_kernel(&queue, &reduce_kernel, 1, None, &[512,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
        }
    
    }

    // Readback
    unsafe {
        core::enqueue_read_buffer(&queue, &nodes_buf, true, 0, &mut net.nodes, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &alloc_buf, true, 0, &mut net.alloc, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &visit_buf, true, 0, &mut net.visit, None::<core::Event>, None::<&mut core::Event>)?;
        core::enqueue_read_buffer(&queue, &event_buf, true, 0, &mut event, None::<core::Event>, None::<&mut core::Event>)?;
    }

    // Print
    //println!("nodes : {:?}", net.nodes);
    //println!("alloc : {:?}", net.alloc);
    //println!("visit : {:?}", net.visit);
    //println!("event : {:?}", event);
    println!("term    : {}", term::from_net(&net));
    println!("-------|");


    Ok(())
}

fn main () {
    //let code = b"/ #f #x /f /f x #f #x /f /f x";
    //let code = b"/ #a a #a a";
    //let mut net = term::to_net(&term::from_string(code));
    //let stats = net::reduce(&mut net);
    //println!("{:?}", net.nodes);
    //println!("{:?}", stats);
    //println!("{}", term::from_net(&net));
    match trivial() { Ok(_v) => (), Err(e) => { println!("{}",e); } }
    //println!("{}", net::hash(123456) as u32);
}

//atomic_cmpxchg(&nodes_buf[(L+0)*4+0], 0, b.y);
//atomic_cmpxchg(&nodes_buf[(L+0)*4+1], 0, port(L+3,1));
//atomic_cmpxchg(&nodes_buf[(L+0)*4+2], 0, port(L+2,1));
//atomic_cmpxchg(&nodes_buf[(L+0)*4+3], 0, a.w&0xFFFFFFFE);
//atomic_cmpxchg(&nodes_buf[(L+1)*4+0], 0, b.z);
//atomic_cmpxchg(&nodes_buf[(L+1)*4+1], 0, port(L+3,2));
//atomic_cmpxchg(&nodes_buf[(L+1)*4+2], 0, port(L+2,2));
//atomic_cmpxchg(&nodes_buf[(L+1)*4+3], 0, a.w&0xFFFFFFFE);
//atomic_cmpxchg(&nodes_buf[(L+2)*4+0], 0, a.z);
//atomic_cmpxchg(&nodes_buf[(L+2)*4+1], 0, port(L+0,2));
//atomic_cmpxchg(&nodes_buf[(L+2)*4+2], 0, port(L+1,2));
//atomic_cmpxchg(&nodes_buf[(L+2)*4+3], 0, b.w&0xFFFFFFFE);
//atomic_cmpxchg(&nodes_buf[(L+3)*4+0], 0, a.y);
//atomic_cmpxchg(&nodes_buf[(L+3)*4+1], 0, port(L+0,1));
//atomic_cmpxchg(&nodes_buf[(L+3)*4+2], 0, port(L+1,1));
//atomic_cmpxchg(&nodes_buf[(L+3)*4+3], 0, b.w&0xFFFFFFFE);
//gpush(event_buf, 2);
//gpush(event_buf, A);
//gpush(event_buf, B);
//gpush(event_buf, L);
//gpush(event_buf, 0);
//gpush(event_buf, 0);
//gpush(event_buf, 0);
//gpush(event_buf, 0);
