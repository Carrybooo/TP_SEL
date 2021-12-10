#![allow(unused_variables)]
use nix::unistd::Pid;
use std::process::id;

fn main() {
    let pid: Pid = Pid::from_raw(id() as i32);
    let res = add_sub(4, 1, 3); //fct test registres rdi/rsi/rdx
    let mut cpt = 0i32;
    loop {
        let trois_n: u64 = trois_n(1000000);
        println!("max trois_n : {}, pid: {}, cpt : {}", trois_n, pid, cpt);
        cpt = cpt + 1;
    }
}

//
//
//

/** function to remplace
*   it's a basic Collatz maximum function for a given range.
*/
pub fn trois_n(n: u64) -> u64 {
    let mut max: u64 = 0;
    //let mut max_index: u64 = 0;
    let mut x: u64;
    let mut last: u64 = 0;

    for i in 1..n {
        x = i;
        while x != 1 {
            if x % 2 == 1 {
                x = (3 * x) + 1;
            } else {
                x = x / 2;
            }

            if x > max {
                max = x;
                //max_index = i;
            }

            //println!("{}", x);
            if x == last {
                break;
            }
            last = x;
        }
    }
    return max;
}

/**
* function used to test good registers order.
*/
pub fn add_sub(p1: u64, p2: u64, p3: u64) -> u64 {
    (p1 + p2) - p3
}
