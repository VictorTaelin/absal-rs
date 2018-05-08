extern crate absal_lib;

use absal_lib::abstract_algorithm::{*};
use absal_lib::lambda_calculus::{*};

fn main() {
    // Parses the following λ-program:
    //   two = λf. λx. f (f x)
    //   exp = λn. λm. m n
    //   exp two two
    let code = b"/ #f #x /f /f x #f #x /f /f x";
    let term = from_string(code);
    let mut net = to_net(&term);
    let stats = reduce(&mut net);
    println!("Stats     : {:?}", stats);
    println!("{}", net.nodes.len());
}
