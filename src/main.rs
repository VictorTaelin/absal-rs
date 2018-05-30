extern crate ocl;
mod term;
mod net;

fn trivial() -> ocl::Result<()> {
    use std::ffi::CString;
    use ocl::{core, flags};
    use ocl::enums::ArgVal;
    use ocl::builders::ContextProperties;

    // Kernel
    let src = r#"
        typedef unsigned int u32;
        typedef unsigned long u64;
        typedef uint4 q32;

        void print(u32, __global u32*);
        u64 hash(u64);
        u64 alloc(u32, __global u64*, u64);
        u32 port(u32, u32);
        u32 node(u32);
        u32 slot(u32);
        void push(u32*, u32);
        u32 pop(u32*);
        u32 peek(u32*);
        void push4(q32*, q32);
        q32 pop4(q32*);
        q32 peek4(q32*);
        void enqueue_visit(u32, __global u32*, u32);
        u32 visit(u32, __global u32*);
        void rewrite(u32, __global u64*, u32, __global u32*, u32, q32, u32, q32);
        void reduce(u32, __global u32*, u32, __global u64*, u32, __global u32*);
        void gpush(__global u32*, u32 val);

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
                u64 k = atomic_cmpxchg(&alloc_buf[i], 0xFFFFFFFFFFFFFFFF, h);
                if (k == 0xFFFFFFFFFFFFFFFF | k == h) {
                    return i * 4;
                }
                i = (i + 1) % alloc_len;
            }
        }

        void enqueue_visit(u32 visit_len, __global u32* visit_buf, u32 port) {
            u32 idx = atomic_inc(&visit_buf[0]) + 1;
            visit_buf[idx] = port;
        }

        u32 visit(u32 visit_len, __global u32* visit_buf) {
            u32 idx = atomic_dec(&visit_buf[0]);
            return visit_buf[idx];
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

        void push4(q32* stack, q32 val) {
            u32 i = ++stack[0].x;
            stack[i] = val;
        }

        q32 pop4(q32* stack) {
            u32 l = stack[0].x--;
            return stack[l];
        }

        q32 peek4(q32* stack) {
            return stack[stack[0].x];
        }

        void gpush(__global u32* stack, u32 val) {
            u32 i = ++stack[0];
            stack[i] = val;
        }

        __kernel void rewrite
            (          u32  alloc_len
            , __global u64* alloc_buf
            ,          u32  nodes_len
            , __global u32* nodes_buf
            ,          u32  prevAddr
            ,          q32  prevNode
            ,          u32  nextAddr
            ,          q32  nextNode) {
            __global q32* nodes_buf_vec = (__global q32*)nodes_buf;
            u32 A = prevAddr < nextAddr ? prevAddr : nextAddr;
            u32 B = prevAddr < nextAddr ? nextAddr : prevAddr;
            q32 a = prevAddr < nextAddr ? prevNode : nextNode;
            q32 b = prevAddr < nextAddr ? nextNode : prevNode;
            if (a.w / 4 == b.w / 4) {
                nodes_buf_vec[A] = (q32)(B*4, b.y, b.z, a.w|1);
                nodes_buf_vec[B] = (q32)(A*4, a.y, a.z, b.w|1);
            } else {
                u32 L = alloc(alloc_len, alloc_buf, (((u64)A << 32) + (u64)B));
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
                nodes_buf_vec[A+0] = (q32)(B*4, port(L+3,0), port(L+2,0), a.w|1);
                nodes_buf_vec[B+0] = (q32)(A*4, port(L+0,0), port(L+1,0), b.w|1);
            }
        }

        __kernel void reduce
            (          u32  visit_len
            , __global u32* visit_buf
            ,          u32  alloc_len
            , __global u64* alloc_buf
            ,          u32  nodes_len
            , __global u32* nodes_buf
            ) {
            __global q32* nodes_buf_vec = (__global q32*)nodes_buf;

            // Initializes path stack
            u32 path[8192];
            q32 memo[8192];
            for (u32 i = 0; i < 8192; ++i) {
                path[i] = 0;
                memo[i] = (q32)(0,0,0,0);
            }

            // Gets node to visit
            u32 initPort = visit(visit_len, visit_buf);
            q32 initNode = nodes_buf_vec[node(initPort)];
            //printf("%d/%d: visits %d(%d:%d)\n", get_global_id(0), get_global_size(0), initPort, node(initPort), slot(initPort));

            // Main loop
            u32 nextPort
                = slot(initPort) == 0 ? initNode.x
                : slot(initPort) == 1 ? initNode.y
                : slot(initPort) == 2 ? initNode.z : 0;

            while (1) {
                // Finds node on nextPort's direction
                q32 nextNode = nodes_buf_vec[node(nextPort)];

                // If next port is root, halt
                if (!nextPort) {
                    break;

                // If nextNode is a wire...
                } else if (nextNode.w & 1) {

                    // If it is a main port, go back to the last visited node 
                    if (slot(nextPort) == 0) {
                        nextPort = (pop4(memo), pop(path));

                    // Otherwise, walk through
                    } else {
                        nextPort = slot(nextPort) == 1 ? nextNode.y : nextNode.z;
                    }

                // If nextPort is the back port of a node, move up
                } else if (slot(nextPort) != 0) {
                    push (path, nextPort);
                    push4(memo, nextNode);
                    nextPort = nextNode.x;

                // If nextPort is the main port...
                } else {

                    // Finds previous port and node
                    u32 prevPort = path[0] > 0 ? port(node(peek(path)), 0) : initPort;
                    q32 prevNode = path[0] > 0 ? peek4(memo) : initNode;

                    // If it is an active pair, reduce it
                    if (slot(prevPort) == 0 && node(prevPort) != 0 && node(nextPort) != 0) {
                        rewrite(alloc_len, alloc_buf, nodes_len, nodes_buf, node(prevPort), prevNode, node(nextPort), nextNode);

                    // Otherwise, this node is on normal form, so, explore its secondary ports
                    } else {
                        enqueue_visit(visit_len, visit_buf, port(node(nextPort),1));
                        enqueue_visit(visit_len, visit_buf, port(node(nextPort),2));
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

    // Params
    let max_alloc_spaces = 3000000; // Maximum number of alloc-spaces. 1 alloc-space = 4 nodes = 64 bytes.
    let max_visit_stack_len = 4096; // Maximum length of visit stack.

    // Create buffers
    let code = b"@A #f #x /f /f /f /f /f /f x @B #f #x /f /f /f /f /f /f x //#a #b //#c #d ///c #e #f #g //g /e /#h #i #j #k /i ///h i j k f /e /#h #i #j #k /j ///h i j k f d #e #f #g g a //#c #d /c /c /c d b #c ///c #d #e #f #g /e ///d e f g #d #e #f #g /f ///d e f g #d #e #f f A B";
    let mut net = term::to_net(&term::from_string(code), max_alloc_spaces);
    println!("Input term: {}", term::from_net(&net));

    // Prepares buffers
    let visit_len = max_visit_stack_len; 
    let mut visit : Vec<u32> = vec![0xFFFFFFFF; visit_len];
    let mut thread_count : Vec<u32> = vec![1];
    visit[0] = 1;
    visit[1] = 0;
    let nodes_len = net.nodes.len();
    let alloc_len = net.alloc.len();
    let nodes_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, nodes_len, Some(&net.nodes))?  };
    let alloc_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, alloc_len, Some(&net.alloc))?  };
    let visit_buf = unsafe { core::create_buffer(&context, flags::MEM_READ_WRITE | flags::MEM_COPY_HOST_PTR, visit_len, Some(&visit))?  };

    // Creates kernel
    let reduce_kernel = core::create_kernel(&program, "reduce")?;
    core::set_kernel_arg(&reduce_kernel, 0, ArgVal::scalar(&visit_len))?;
    core::set_kernel_arg(&reduce_kernel, 1, ArgVal::mem(&visit_buf))?;
    core::set_kernel_arg(&reduce_kernel, 2, ArgVal::scalar(&alloc_len))?;
    core::set_kernel_arg(&reduce_kernel, 3, ArgVal::mem(&alloc_buf))?;
    core::set_kernel_arg(&reduce_kernel, 4, ArgVal::scalar(&nodes_len))?;
    core::set_kernel_arg(&reduce_kernel, 5, ArgVal::mem(&nodes_buf))?;

    // Spawns n parallel threads until done
    unsafe { 
        while thread_count[0] > 0 {
            core::enqueue_kernel(&queue, &reduce_kernel, 1, None, &[thread_count[0] as usize,1,1], None, None::<core::Event>, None::<&mut core::Event>)?;
            core::enqueue_read_buffer(&queue, &visit_buf, true, 0, &mut thread_count, None::<core::Event>, None::<&mut core::Event>)?;
        }
    }

    // Buffer readback (slow, could be avoided by simply collecting garbage on chip)
    unsafe {
        core::enqueue_read_buffer(&queue, &nodes_buf, true, 0, &mut net.nodes, None::<core::Event>, None::<&mut core::Event>)?;
    }

    // Print result
    println!("Output: {}", term::from_net(&net));

    Ok(())
}

fn main () {
    match trivial() { Ok(_v) => (), Err(e) => { println!("{}",e); } }
}
