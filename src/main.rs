extern crate absal;

fn main() {
    // Parses the following λ-program:
    //   two = λf. λx. f (f x)
    //   exp = λn. λm. m n
    //   exp two two
    //let (stats, code) = absal::reduce("/// #f #x /f /f /f x #f #x /f /f /f x #x x #x x");
    let term = absal::term::from_string(b"#f #x /f /f x");
    let mut net = absal::term::to_net(&term);
    absal::net::reduce(&mut net);
    println!("{}", term);
    println!("{}", absal::term::from_net(&net));
    //println!("{}", code);
}
