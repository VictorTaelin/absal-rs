extern crate absal;

use absal::abstract_algorithm::{*};
use absal::lambda_calculus::{*};

fn main() {
    // Parses the following λ-program:
    //   two = λf. λx. f (f x)
    //   exp = λn. λm. m n
    //   exp two two
    let code = b"/ #f #x /f /f x #f #x /f /f x";
    let term = from_string(code);
    let mut net = to_net(&term);
    println!("net {:?}", net);
    let stats = reduce(&mut net);
    println!("Stats     : {:?}", stats);
    println!("{}", net.nodes.len());
    println!("{}", from_net(&net));
}
