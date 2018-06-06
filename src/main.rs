extern crate absal;

fn main() {
    // Parses the following λ-program:
    //   two = λf. λx. f (f x)
    //   exp = λn. λm. m n
    //   exp two two
    //let (stats, code) = absal::reduce("/// #f #x /f /f /f x #f #x /f /f /f x #x x #x x");
    let term = absal::term::from_string(b"
        @toNum #nat //nat #x + x 1 0
        @c2 #f #x /f /f x
        @c4 /c2 c2
        @c256 /c4 c4
        @c65536 /c2 c256
        /toNum c65536
    ");
    println!("{}", term);
    let mut net = absal::term::to_net(&term);
    let stats = absal::net::reduce(&mut net);
    println!("{}", absal::term::from_net(&net));
    println!("{:?}", stats);
}
