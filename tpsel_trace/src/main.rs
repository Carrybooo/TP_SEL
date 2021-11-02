#![allow(unused_imports)]
// use nix;
// use std;
// use std::io::{Read, Seek, Write};
//
// use libc;
// use libc::malloc;
// use nix::sys::ptrace;
// use std::alloc::{GlobalAlloc, Layout, System};

use std::process::id;
use std::thread::sleep;
use std::time::Duration;

// use nix::sys::signal::Signal;
// use nix::sys::wait::{wait, WaitStatus};
// use nix::unistd::{fork, ForkResult, Pid};
// use std::os::unix::process::CommandExt;
use nix::unistd::Pid;

fn main() {
    let pid: Pid = Pid::from_raw(id() as i32);
    let mut cpt = 0i32;
    loop {
        if cpt % 5 == 0 {
            let trois_n: u64 = trois_n(10000);
            println!("max trois_n : {}, pid: {}, cpt : {}", trois_n, pid, cpt);
        } else {
            println!("and counting... cpt = {}", cpt);
        }
        sleep(Duration::new(1, 0));
        cpt = cpt + 1;
    }
}

//
//
//

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
