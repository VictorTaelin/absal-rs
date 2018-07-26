extern crate absal;
pub mod proof;

fn main() {
    let term = &proof::from_string(b"
        $Nat
            &Nat *
            @s   !@n Nat Nat
            !@z  Nat
            Nat

        $C0
            ^Nat *
            #s   !@n Nat Nat =S s
            |#z  Nat
            z

        $C1
            ^Nat *
            #s   !@n Nat Nat =S s
            |#z  Nat
            :S z

        $C2
            ^Nat *
            #s   !@n Nat Nat =S s
            |#z  Nat
            :S :S z

        $C3
            ^Nat *
            #s   !@n Nat Nat =S s
            |#z  Nat
            :S :S :S z

        $mul
            #a   Nat
            #b   Nat
            ^Nat *
            #s   !@n Nat Nat
            ::a Nat ::b Nat s

        $add
            #a   Nat
            #b   Nat
            ^Nat *
            #s   !@n Nat Nat
            =S s
            =f ::a Nat |S
            =g ::b Nat |S
            |#z  Nat
            :f :g z

        ::add C3 C3
    ");
    proof::is_stratified(term);
    println!("{}", proof::reduce(term));
    println!("{}", proof::infer(term));
}

/*
~
^a *
#b !@b a a =c b =d ::a b c
|#e b
:d :c :c :c e

@a &a * @b !@b a a !@c a a
@b &b * @c !@c b b !@d b b
   &c * @d !@d c c
        @e !@e c c !@f c c
*/

//@a &a * @b !@b a a !@c a a
//@b &b * @c !@c b b !@d b b
   //&c * @d !@d c c
        // !@e c c
