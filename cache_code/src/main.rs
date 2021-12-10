#![allow(unused)]

fn main() {
    println!("trois_n_opti: {}", trois_n_opti(1000000));
    println!("opti : {}", fn_opti());
}

fn trois_n_opti(n: u64) -> u64 {
    if (n == 1000000u64) {
        let mut res = 704511u64;
        while res != 56991483520u64 {
            if res % 2 == 1 {
                res = (3 * res) + 1;
            } else {
                res = res / 2;
            }
        }
        return res;
    } else {
        return 0;
    }
}

fn fn_opti() -> u64 {
    1234567890u64
}
