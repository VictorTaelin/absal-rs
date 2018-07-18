extern crate absal;
pub mod proof;

fn main() {
    // Parses the following λ-program:
    //   two = λf. λx. f (f x)
    //   exp = λn. λm. m n
    //   exp two two
    let (stats, code) = absal::reduce(":~:#f |#x :~f :~f x |#f |#x :~f :~f x |#f |#x :~f :~f x");
    println!("{:?}", stats);
    println!("{}", code);

    println!("{}", proof::reduce(&proof::from_string(b": #A * #B * :A :A :A B #A * #B * :A :A :A B")));
    println!("{}", proof::infer(&proof::from_string(b"#P * #Q * #S @n P P #Z P :S :S Z")));

}
