#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused)]

use subprocess::*; //pour les pipes

use nix;
use std;
use std::fs::{write, File, OpenOptions};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};

use libc;
use libc::malloc;
use nix::sys::ptrace;
use std::alloc::{GlobalAlloc, Layout, System};
use std::process;
use std::process::Command;
use std::str;

use nix::sys::signal::Signal;
use nix::sys::wait::{wait, WaitStatus};
use nix::unistd::{fork, ForkResult, Pid};
use std::os::unix::process::CommandExt;

use std::thread::sleep;
use std::time::Duration;

/* NOTES:
POUR ALLOUER UNE VARIABLE
faire remonter le pointeur de pile pour créer une place pour une variable
faire (- sizeof(taille de la valeur qu'on souhaite)) sur le ptr

BLOC A GARDER POUR pouvoir UTILISER LE MALLOC plus tard
unsafe {
    let res_malloc = malloc(1024);
    println!("resultat : {:?}", res_malloc);
    //loop {} //test
}

*/

fn main() {
    let local_pid: Pid = Pid::from_raw(process::id() as i32);
    // println!("local pid : {}", local_pid);

    //get PID
    let pid_trace: i32 = pgrep("tpsel_trace")
        .expect("Erreur lors de la récupération de l'identifiant du programme tracé")
        as i32;
    //test print PID
    println!("pid trace : {}\n", pid_trace);

    //get the address of function name given in arg
    let address_name = "trois_n";
    let address: u64 = get_addr(pid_trace, address_name)
        .expect("Erreur lors de la récupéraion de l'addresse de la fonction du prog tracé");

    //print the address of function_name in hexa and decimal
    println!(
        "address of function \"{}\" :\nhexa (filled) : \"{:0>16x}\"\ndecimal : \"{}\"\n",
        address_name, address, address
    );

    // attaching to process
    let attaching = ptrace::attach(Pid::from_raw(pid_trace));
    println!("attaching result : {:?}\n", attaching);

    inject_trap(pid_trace, address);
    wait();

    // detaching from process
    let detaching = ptrace::cont(Pid::from_raw(pid_trace), Signal::SIGCONT);
    println!("detaching result : {:?}\n", detaching);

    println!("end");
}

//FONCTION UTILES
fn pgrep(name: &str) -> Option<isize> {
    let output = Command::new("pgrep").arg(name).output().unwrap();
    if output.status.success() {
        let pid = str::from_utf8(&output.stdout)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        Some(pid)
    } else {
        None
    }
}

//objdump -t /proc/xxx/exe | grep trois_n | cut -c1-16
fn get_addr(pid: i32, addr_name: &str) -> Option<u64> {
    //on crée la bonne string à partir du param
    let arg1 = format!("objdump -t /proc/{}/exe", pid);
    let arg2 = format!("grep {}", addr_name);
    //println!("{}", &arg);

    //on met dans un vecteur toutes les commandes à utiliser à la suite (via des pipes)
    let commands = vec![
        Exec::shell(arg1),
        Exec::shell(arg2),
        Exec::shell("cut -c1-16"),
    ];
    //on execute les commandes
    let pipeline = subprocess::Pipeline::from_exec_iter(commands);
    let output = pipeline.capture().unwrap().stdout_str();
    // println!("TEST RESULT GETADDR (avant parse) : \"{}\"", output);

    //on réduit le retour à l'addresse seule (on vire le "\n" récupéré derriere)
    let result = output.trim_end();
    let result = u64::from_str_radix(result, 16);
    result.ok()
}

//cat /proc/xxx/maps | grep -m1 tpsel_trace | cut -c1-12
fn get_offset(pid: i32) -> Option<u64> {
    //on crée la bonne string à partir du param
    let arg = format!("cat /proc/{}/maps", pid);

    let commands = vec![
        Exec::shell(arg),
        Exec::shell("head -n 1"),
        Exec::shell("cut -c1-12"),
    ];
    let pipeline = subprocess::Pipeline::from_exec_iter(commands);
    let output = pipeline.capture().unwrap().stdout_str();
    let result = output.trim_end();
    let result = u64::from_str_radix(result, 16);
    result.ok()
}

fn inject_trap(pid: i32, address: u64) {
    let trap: u8 = 0xCC;
    let path = format!("/proc/{}/mem", pid);
    let offset: u64 = get_offset(pid).expect("Erreur lors de la recupération de l'adresse mémoire");
    //let addr = format!("{:0>16x}", address);
    //let file_read = File::open(path).expect("Erreur à l'ouverture de la memoire du processus");
    // let content : &str = trap_file(file_read);
    // let file_write = std::fs::write(path, content).expect("Erreur à l'écriture dans la mémoire")
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Erreur lors de l'ouverture du fichier");
    //let seek_param = seek_addr(&mut file, addr.as_str());
    file.seek(SeekFrom::Start(address as u64 + offset))
        .expect("Erreur lors de la modification du curseur pour l'écriture");
    file.write_all(&[trap])
        .expect("Erreur lors de l'écriture de l'instruction trap dans la mémoire du tracé");
}

// fn seek_addr(file: &mut File, addr: &str) -> u64 {
//     let mut buff: Vec<u8> = vec![];
//     println!("file : {:?}", &file);
//     let res_read = file.read(&mut buff);
//     match res_read {
//         Err(err) => println!("erreur lecture bytes fichier : {}", err),
//         Ok(ok) => println!("lecture fichier bien passée : {}", ok),
//     }
//     let mut buff = buff.as_slice();
//     println!("buff : {:?}", &buff);
//     let addr = addr.as_bytes();
//     let mut start_indice = 0u64;
//     while !buff.starts_with(addr) {
//         start_indice = start_indice + 1; //on incrémente l'indice de départ
//         buff = &buff[1..]; //on garde tout sauf le first
//     }
//     start_indice
// }
//
//
//
//
//
//
//
//
//
