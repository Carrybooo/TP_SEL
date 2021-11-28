#![allow(unused_imports)]
#![allow(unused)]

use std::env; //pour collect les arguments passés à l'appel du programme.
use subprocess::*; //pour les pipes

use nix;
use std;
use std::fs::{write, File, OpenOptions};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};

// use libc;
// use libc::malloc;
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

use nix::libc::{c_int, c_void, mprotect, posix_memalign};
use std::mem::{align_of, size_of};

use std::ptr;

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
    //bloc pour gérer d'éventuels paramètres à l'appel du programme
    //1er param : fonction à remplacer
    //2eme param : fonction qui remplacera
    let args: Vec<String> = env::args().collect();
    let functions: (&str, &str) = match args.len() {
        2 => (&args[1].as_str(), "add_sub"),
        3 => (&args[1].as_str(), &args[2].as_str()),
        _ => ("trois_n", "add_sub"),
    };

    //
    //
    //----------------PID + CALCUL ADDRESSES-------------------------------------------------------
    //

    //this PID
    let pid_local: i32 = process::id() as i32;
    println!("PID local : {}", pid_local);

    //get PID
    let pid_trace: i32 = pgrep("tpsel_trace")
        .expect("Erreur lors de la récupération de l'identifiant du programme tracé")
        as i32;

    //pour ptrace il faut un type spécial "Pid" :
    let pid_ptrace: Pid = Pid::from_raw(pid_trace);

    //address of the start of the code
    let mem_address = get_address(pid_trace)
        .expect("Erreur lors de la récupération de l'adresse de début du code");
    //get the offset (in the program) of the function (name given in arg)
    let offset_fct_to_replace: u64 = get_offset(pid_trace, functions.0)
        .expect("Erreur lors de la récupéraion de l'addresse de la fonction du prog tracé");
    //offset of the 2nd function
    let offset_fct_replacing: u64 = get_offset(pid_trace, functions.1)
        .expect("Erreur lors de la récupération de l'addresse de la 2e fonction du prog tracé");

    //address of libc in mem
    let libc_address = get_libc_address(pid_trace)
        .expect("Erreur lors de la récupération de l'addresse de la libc");
    let posix_memalign_offset = get_libc_offset("posix_memalign")
        .expect("Erreur lors de la récupération de l'offset de posix_memalign");

    //print de toutes les variables calculées avant
    println!(
        //print des addresses.
        "offset fonction 1 : {:x}\n\
        offset fonction 2 : {:x}\n\
        addresse page mémoire : {:x}\n\
        addresse libc : {:x}\n\
        offset posix_memalign : {:x}\n",
        offset_fct_to_replace,
        offset_fct_replacing,
        mem_address,
        libc_address,
        posix_memalign_offset,
    );

    //POSIX_MEMALIGN EXAMPLE
    // unsafe {
    //     let mut out = ptr::null_mut();
    //     let align = size_of::<usize>();
    //     let ret = libc::posix_memalign(&mut out, align, align);
    //     println!("ret : {}", ret);
    //     println!("out : {:?}\nout : {:x}\n", out, out as u64);
    // }

    //
    //
    //---------------- ATTACHING + MODIF --------------------
    //

    ptrace::attach(pid_ptrace) //attaching to process
        .expect("Erreur lors de l'attachement au processus cible");

    wait().expect("erreur au wait : "); //wait for 1st trap
    inject(pid_trace, offset_fct_to_replace, false); //injecting
    ptrace::cont(pid_ptrace, Signal::SIGCONT);

    wait().expect("erreur au wait : ");

    let mut regs =
        ptrace::getregs(pid_ptrace).expect("Erreur récupération des regs après 1er trap");
    println!(
        "Avant modif des regs:\nrax = {:x}\nrip = {:x}\nrsp = {:x}\n",
        regs.rax, regs.rip, regs.rsp,
    );

    // chall2 avec 3 args au lieu d'1 pour tester
    // regs.rax = mem_address + offset_fct_replacing;
    // regs.rdi = 12;
    // regs.rsi = 5;
    // regs.rdx = 3;

    let ptr_to_ptr: *mut *mut c_void = ptr::null_mut();

    regs.rax = get_libc_address(pid_trace).unwrap() + get_libc_offset("posix_memalign").unwrap();
    regs.rsp = regs.rsp - (size_of::<*mut *mut c_void>() as u64);
    regs.rdi = ptr_to_ptr as u64;
    regs.rsi = size_of::<usize>() as u64;
    regs.rdx = 0; //size_of::<usize>() as u64;

    println!(
        "Avant l'execution de la fonction\n\
            rax = {:x}\n\
            rdi = {:x}\n\
            rip = {:x}\n\
            rsp = {:x}\n",
        regs.rax, regs.rdi, regs.rip, regs.rsp,
    );

    ptrace::setregs(pid_ptrace, regs); //set regs with modification

    ptrace::cont(pid_ptrace, Signal::SIGCONT);

    wait().expect("erreur au wait2 : ");

    let regs = ptrace::getregs(pid_ptrace).expect("Erreur récupération des regs APRES modif regs");

    println!(
        "Après l'execution de la fonction\n\
            rax = {}\n\
            rdi = {:x}\n\
            rip = {:x}\n\
            rsp = {:x}\n",
        regs.rax, regs.rdi, regs.rip, regs.rsp,
    );

    // wait();
    // wait();

    // let rdi = read_rdi(pid_trace, regs.rdi, mem_address);
    //
    // println!("lecture sur rdi : {}", rdi);

    //
    //------------------ DETACHING ----------------------------------------------------------------
    //
    ptrace::detach(pid_ptrace, Signal::SIGCONT) //detaching
        .expect("Erreur lors du détachement du processus");

    println!("Tout s'est bien passé, sortie du programme."); //end
}

//
//
//
//-------------------------------------------------------------------------------------------------
//-------------------------------------FUNCTIONS---------------------------------------------------
//-------------------------------------------------------------------------------------------------
//
//FONCTIONS
fn pgrep(name: &str) -> Option<isize> {
    let output = Command::new("pgrep").arg(name).output().unwrap();
    if output.status.success() {
        let pid = str::from_utf8(&output.stdout)
            .unwrap()
            .trim()
            .parse()
            .expect("Erreur à la récupération du PID");
        Some(pid)
    } else {
        None
    }
}

//cat /proc/xxx/maps | grep -m1 tpsel_trace | cut -c1-12
fn get_address(pid: i32) -> Option<u64> {
    let cmd1 = format!("cat /proc/{}/maps", pid); //on crée la bonne commande à partir du param

    //on met dans un vecteur toutes les commandes à utiliser à la suite (via des pipes)
    let commands = vec![
        Exec::shell(cmd1),
        Exec::shell("head -n 1"),
        Exec::shell("cut -c1-12"),
    ];
    let pipeline = subprocess::Pipeline::from_exec_iter(commands); //on execute les commandes
    let output = pipeline.capture().unwrap().stdout_str(); //on récupère le résultat

    let result = output.trim_end(); //on vire le retour à la ligne situé à la fin de l'output
    let result = u64::from_str_radix(result, 16);
    result.ok()
}

//cat /proc/xxx/maps | grep -m1 tpsel_trace | cut -c1-12
fn get_libc_address(pid: i32) -> Option<u64> {
    let cmd1 = format!("cat /proc/{}/maps", pid); //on crée la bonne commande à partir du param

    //on met dans un vecteur toutes les commandes à utiliser à la suite (via des pipes)
    let commands = vec![
        Exec::shell(cmd1),
        Exec::shell("grep libc"),
        Exec::shell("head -n 1"),
        Exec::shell("cut -c1-12"),
    ];
    let pipeline = subprocess::Pipeline::from_exec_iter(commands); //on execute les commandes
    let output = pipeline.capture().unwrap().stdout_str(); //on récupère le résultat

    let result = output.trim_end(); //on vire le retour à la ligne situé à la fin de l'output
    let result = u64::from_str_radix(result, 16);
    result.ok()
}

//objdump -t /proc/xxx/exe | grep trois_n | cut -c1-16
fn get_offset(pid: i32, addr_name: &str) -> Option<u64> {
    let cmd1 = format!("objdump -t /proc/{}/exe", pid); //on crée la bonne string à partir des params
    let cmd2 = format!("grep {}", addr_name);

    //on met dans un vecteur toutes les commandes
    let commands = vec![
        Exec::shell(cmd1),
        Exec::shell(cmd2),
        Exec::shell("cut -c1-16"),
    ];
    let pipeline = subprocess::Pipeline::from_exec_iter(commands); //on execute les commandes
    let output = pipeline.capture().unwrap().stdout_str(); //on récupère le résultat

    let result = output.trim_end(); //on vire le retour à la ligne situé à la fin de l'output
    let result = u64::from_str_radix(result, 16);
    result.ok()
}

//nm /usr/lib64/libc.so.6 | grep "fn_name"
fn get_libc_offset(fn_name: &str) -> Option<u64> {
    //on met dans un vecteur toutes les commandes
    let cmd1 = format!("grep {}", fn_name);

    let commands = vec![
        Exec::shell("nm /usr/lib64/libc.so.6"),
        Exec::shell(cmd1),
        Exec::shell("head -n 1"),
        Exec::shell("cut -c1-16"),
    ];
    let pipeline = subprocess::Pipeline::from_exec_iter(commands); //on execute les commandes
    let output = pipeline.capture().unwrap().stdout_str(); //on récupère le résultat

    let result = output.trim_end(); //on vire le retour à la ligne situé à la fin de l'output
    let result = u64::from_str_radix(result, 16);
    result.ok()
}

fn inject(pid: i32, offset: u64, force_chall_1: bool) {
    let trap: [u8; 4] = match force_chall_1 {
        true => [0xCC, 0xCC, 0xCC, 0xCC],
        false => [0xCC, 0xFF, 0xD0, 0xCC],
    };
    let path = format!("/proc/{}/mem", pid);
    let address: u64 = get_address(pid)
        .expect("Erreur lors de la recupération de l'adresse mémoire dans inject()");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Erreur lors de l'ouverture du fichier");
    file.seek(SeekFrom::Start(offset + address))
        .expect("Erreur lors de la modification du curseur pour l'écriture");
    file.write(&trap)
        .expect("Erreur lors de l'écriture des instructions dans la mémoire du tracé");
}

fn read_rdi(pid: i32, rdi: u64, address: u64) -> String {
    let mut string: String = String::from("");
    let path = format!("/proc/{}/mem", pid);
    // let address: u64 =
    //     get_address(pid).expect("Erreur lors de la recupération de l'adresse mémoire");
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .expect("Erreur lors de l'ouverture du fichier");
    file.seek(SeekFrom::Start(rdi))
        .expect("Erreur lors de la modification du curseur pour la lecture");
    file.read_to_string(&mut string)
        .expect("Erreur lors de la lecture à l'offset rdi");
    return string;
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
