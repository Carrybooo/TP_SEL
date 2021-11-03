#![allow(unused_imports)]

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
    //get PID
    let pid_trace: i32 = pgrep("tpsel_trace")
        .expect("Erreur lors de la récupération de l'identifiant du programme tracé")
        as i32;
    //get the address (in the program) of the function (name given in arg)
    let address_name = "trois_n";
    let address: u64 = get_offset(pid_trace, address_name)
        .expect("Erreur lors de la récupéraion de l'addresse de la fonction du prog tracé");

    ptrace::attach(Pid::from_raw(pid_trace)) //attaching to process
        .expect("Erreur lors de l'attachement au processus cible");

    inject_trap(pid_trace, address); //injecting trap
    wait().expect("erreur au wait : ");

    ptrace::detach(Pid::from_raw(pid_trace), Signal::SIGCONT) //detaching
        .expect("Erreur lors du détachement du processus");

    println!("Tout s'est bien passé, arrêt du programme."); //end
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
fn get_offset(pid: i32, addr_name: &str) -> Option<u64> {
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
fn get_address(pid: i32) -> Option<u64> {
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
    let offset: u64 =
        get_address(pid).expect("Erreur lors de la recupération de l'adresse mémoire");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Erreur lors de l'ouverture du fichier");
    file.seek(SeekFrom::Start(address as u64 + offset))
        .expect("Erreur lors de la modification du curseur pour l'écriture");
    file.write_all(&[trap])
        .expect("Erreur lors de l'écriture de l'instruction trap dans la mémoire du tracé");
}
//
//
//
//
//
//
//
//
//
