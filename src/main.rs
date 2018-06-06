extern crate absal;

fn main() {
    // Parses the following λ-program:
    //   two = λf. λx. f (f x)
    //   exp = λn. λm. m n
    //   exp two two
    let term = absal::term::from_string(b"
        @mul #a #b #s #z //a /b s z
        @c1 #f #x /f x
        @c2 #f #x /f /f x
        @c4 //mul c2 c2
        @c8 //mul c2 c4
        @c16 //mul c2 c8
        @c32 //mul c2 c16
        @c64 //mul c2 c32
        @c128 //mul c2 c64
        @c256 //mul c2 c128
        @c512 //mul c2 c256
        @c1024 //mul c2 c512
        @c2048 //mul c2 c1024
        @c4096 //mul c2 c2048
        @c8192 //mul c2 c4096
        @c16384 //mul c2 c8192
        @c32768 //mul c2 c16384
        @c65536 //mul c2 c32768
        @c131072 //mul c2 c65536
        @c262144 //mul c2 c131072
        @c524288 //mul c2 c262144
        @c1048576 //mul c2 c524288
        @c2097152 //mul c2 c1048576
        @c4194304 //mul c2 c2097152
        @c8388608 //mul c2 c4194304
        @c16777216 //mul c2 c8388608
        @replicate #n #x #cons #nil //n #r //cons x r nil
        @sum #list //list #a #b + a b 0
        /sum //replicate c16777216 1
    ");
    println!("Input: {}\n", term);
    let mut net = absal::term::to_net(&term);
    let stats = absal::net::reduce(&mut net);
    println!("Output: {}\n", absal::term::from_net(&net));
    println!("Stats: {:?}", stats);
}

/*
$ cargo build --release; time ./main
    Finished release [optimized] target(s) in 0.0 secs
Input: /#a //a #b #c +b c 0 //#a #b #c #d //a #e //c b e d //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b //#a #b #c #d //a /b c d #a #b /a /a b #a #b /a /a b 1

Output: 16777216

Stats: Stats { loops: 67109305, rules: 33554652, betas: 0, dupls: 0, annis: 0 }

real	0m0.920s
user	0m0.677s
sys	    0m0.226s
*/
