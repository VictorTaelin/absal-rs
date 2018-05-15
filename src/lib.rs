pub mod term;
pub mod net;

pub fn reduce(code : &term::Str) -> (net::Stats, Vec<term::Chr>) {
    let term = term::from_string(code);
    let mut net = term::to_net(&term);
    let stats = net::reduce(&mut net);
    let reduced_term = term::from_net(&net);
    let reduced_code = term::to_string(&reduced_term);
    (stats, reduced_code)
}
