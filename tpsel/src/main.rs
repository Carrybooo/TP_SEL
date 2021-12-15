// #![allow(unused)]

//use std::env; //pour collect les arguments passés à l'appel du programme.
use nix;
use nix::sys::ptrace;
use nix::sys::signal::Signal;
use nix::sys::wait::wait;
use nix::unistd::Pid;
use std;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process;
use std::process::Command;
use std::str;
use subprocess::*; //pour les pipes

fn main() {
    //
    //----------------PID + DECLARATIONS ADDRESSES-------------------------------------------------
    ///////////////////////////////////////////////////////////////////////////////////////////////
    //

    //this PID
    let pid_local: i32 = process::id() as i32;
    println!("\nPID local : {}", pid_local);

    //get PID
    let pid_trace: i32 = pgrep("tpsel_trace")
        .expect("Erreur lors de la récupération de l'identifiant du programme tracé")
        as i32;

    //pour ptrace et d'autres instructions il faut une struct spéciale "Pid" :
    let pid_trace_struct: Pid = Pid::from_raw(pid_trace);

    //address of the start of the code
    let mem_address = get_address(pid_trace)
        .expect("Erreur lors de la récupération de l'adresse de début du code");

    //get the offset (in the program) of the function (name given in arg)
    let offset_fct_to_replace: u64 = get_offset(pid_trace, "trois_n")
        .expect("Erreur lors de la récupéraion de l'addresse de la fonction du prog tracé");

    //offset of the 2nd function
    let offset_fct_replacing: u64 = get_offset(pid_trace, "add_sub")
        .expect("Erreur lors de la récupération de l'addresse de la 2e fonction du prog tracé");

    let libc_address = get_libc_address(pid_trace)
        .expect("Erreur lors de la récupération de l'addresse de la libc");
    let malloc_offset = get_libc_offset("__libc_malloc")
        .expect("Erreur lors de la récupération de l'offset de malloc");
    let memalign_offset = get_libc_offset("__libc_memalign")
        .expect("Erreur lors de la récupération de l'address de memalign");
    let mprotect_offset = get_libc_offset("__mprotect")
        .expect("Erreur lors de la récupération de l'offset de mprotect");

    //print des variables calculées avant
    println!(
        "offset fonction 1 : {:x}\n\
        offset fonction 2 : {:x}\n\
        addresse mémoire proc : {:x}\n\
        addresse libc : {:x}\n\
        offset malloc : {:x}\n",
        offset_fct_to_replace, offset_fct_replacing, mem_address, libc_address, malloc_offset,
    );

    //
    //
    //---------------- ATTACHING + TRAP -----------------------------------------------------------
    ///////////////////////////////////////////////////////////////////////////////////////////////
    //

    ptrace::attach(pid_trace_struct) //attaching to process
        .expect("Erreur lors de l'attachement au processus cible");
    wait().expect("erreur wait après attachement : "); //wait after attaching

    //on récupère les 12 premiers bytes de la fonction à remplacer pour plus tard
    let _origin_function_bytes: [u8; 12] = read_12(pid_trace, mem_address, offset_fct_to_replace);

    inject_trap(pid_trace, mem_address, offset_fct_to_replace, false); //injecting

    ptrace::cont(pid_trace_struct, Signal::SIGCONT).expect("Erreur SIGCONT après inject_trap");
    wait().expect("erreur wait pour premier trap : "); //wait for 1st trap

    let mut regs = ptrace::getregs(pid_trace_struct)
        .expect("Erreur récupération des regs après avant exec memalign");
    println!("Premier getregs avant toute modif : {}", print_regs(regs));

    //on garde le rip d'origine pour pouvoir réstaurer l'état plus tard
    let mut _origin_rip = regs.rip.clone();

    println!("----------------------------------------------------------------------\nMEMALIGN");
    //
    //
    //                           1 1 1 1 1 1 1 1 1 1 1 1
    //
    ////////////////////////////PREMIER APPEL : MEMALIGN///////////////////////////////////////////
    regs.rax = libc_address + memalign_offset; // __libc_memalign address
    regs.rdi = 4096; //page size for align
    regs.rsi = 16; //size of mem to allocate

    println!("Avant l'execution de memalign : {}", print_regs(regs));

    ptrace::setregs(pid_trace_struct, regs).expect("Erreur lors du setregs pour le memalign");

    ptrace::cont(pid_trace_struct, Signal::SIGCONT).expect("Erreur SIGCONT execution memalign");
    wait().expect("erreur wait après exécution memalign: ");

    let mut regs = ptrace::getregs(pid_trace_struct)
        .expect("Erreur récupération des regs APRES exec memalign");

    println!("Après l'execution de memalign : {}", print_regs(regs));

    let addr_allocated = regs.rax; //addr of the allocated memory
    println!("address allocated : {:x}", addr_allocated);

    println!("----------------------------------------------------------------------\nMPROTECT");
    //
    //
    //                          2 2 2 2 2 2 2 2 2 2 2 2 2
    //
    ////////////////////////////DEUXIEME APPEL : MPROTECT//////////////////////////////////////////
    regs.rip = regs.rip - 3; //decrement the instruction pointer to get back just after 1st trap

    regs.rax = libc_address + mprotect_offset; //addr of __mprotect
    regs.rdi = addr_allocated; //addr of allocated mem
    regs.rsi = 16; //size of the memory allocated
    regs.rdx = 7; //type of modif, 1=read, 2=write, 4=exec. (so 6=w+x, 7=r+w+x)

    println!("Avant execution memprotect : {}", print_regs(regs));

    ptrace::setregs(pid_trace_struct, regs).expect("Erreur setregs mprotect");

    ptrace::cont(pid_trace_struct, Signal::SIGCONT).expect("Erreur SIGCONT execution mprotect");
    wait().expect("Erreur wait après exécution mprotect : ");

    let mut regs = ptrace::getregs(pid_trace_struct)
        .expect("Erreur récupération des regs APRES exec mprotect");

    println!("Après execution memprotect : {}", print_regs(regs));

    println!("----------------------------------------------------------------------\nCACHE CODE");
    //
    //
    //                       3 3 3 3 3 3 3 3 3 3 3 3 3 3
    //
    ////////////////////////TROISIEME APPEL : CODE CACHE///////////////////////////////////////////
    inject_cache(pid_trace, addr_allocated); //inject cache code into allocated memory

    //création du jump à injecter
    let addr_bytes = addr_allocated.to_ne_bytes(); //convert address to bytes
    print_bytes(addr_bytes); //print bytes to be sure, then make the jump (with a trap at the end
                             //to allow a getregs to verify the function call)
    let jump: [u8; 12] = [
        0x48,
        0xB8,
        addr_bytes[0],
        addr_bytes[1],
        addr_bytes[2],
        addr_bytes[3],
        addr_bytes[4],
        addr_bytes[5],
        addr_bytes[6],
        addr_bytes[7],
        0xff,
        0xe0,
    ];

    inject_12(pid_trace, mem_address + offset_fct_to_replace, jump);
    regs.rip = regs.rip - 4; //decrement instruction pointer to get back to origin function call
    ptrace::setregs(pid_trace_struct, regs).expect("Erreur setregs trampoline");
    println!("Avant exécution du code cache : {}", print_regs(regs));

    //
    //
    //----------------------- DETACHING -----------------------------------------------------------
    ///////////////////////////////////////////////////////////////////////////////////////////////
    //

    ptrace::detach(pid_trace_struct, Signal::SIGCONT) //detaching
        .expect("Erreur lors du détachement du processus");

    println!("Tout s'est bien passé, sortie du programme."); //end
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
//
//-------------------------------------------------------------------------------------------------
//-------------------------------------FUNCTIONS & CONSTS------------------------------------------
//-------------------------------------------------------------------------------------------------
//
//CONSTS
//Cache code, this one is just a fct that returns 1234567890u64. (as simple as possible at first.)
const CACHE_CODE: [u8; 16] = [
    0xb8, 0xd2, 0x02, 0x96, 0x49, 0xc3, 0x66, 0x2e, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00,
];

//FONCTIONS
fn print_bytes(addr_bytes: [u8; 8]) {
    println!(
        "address bytes : {:X} {:X} {:X} {:X} {:X} {:X} {:X} {:X} (IN BIG ENDIAN)\n",
        addr_bytes[0],
        addr_bytes[1],
        addr_bytes[2],
        addr_bytes[3],
        addr_bytes[4],
        addr_bytes[5],
        addr_bytes[6],
        addr_bytes[7],
    );
}

/** Fonction pour print les regs, appelée dans un unique println, pour avoir un code moins lourd
*/
fn print_regs(regs: libc::user_regs_struct) -> String {
    let res = format!(
        "\n\
            rax = {:x}\n\
            rdi = {:x}\n\
            rsi = {:x}\n\
            rdx = {:x}\n\
            rip = {:x}\n",
        regs.rax, regs.rdi, regs.rsi, regs.rdx, regs.rip,
    );
    res
}

fn pgrep(name: &str) -> Option<isize> {
    let output = Command::new("pgrep").arg(name).output().unwrap();
    if output.status.success() {
        let pid = str::from_utf8(&output.stdout)
            .unwrap()
            .trim()
            .parse()
            .expect(
                "Erreur à la récupération du PID, il y a \
                probablement plus d'1 processus ayant ce nom",
            );
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

//objdump -t /proc/xxx/exe | grep trois_n | cut -c1-16
fn get_offset(pid: i32, addr_name: &str) -> Option<u64> {
    let cmd1 = format!("objdump -t /proc/{}/exe", pid); //on crée la string depuis les params
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

fn inject_trap(pid: i32, address: u64, offset: u64, force_chall_1: bool) {
    let trap: [u8; 4] = match force_chall_1 {
        true => [0xCC, 0xCC, 0xCC, 0xCC],
        false => [0xCC, 0xFF, 0xD0, 0xCC],
    };
    let path = format!("/proc/{}/mem", pid);
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

fn inject_cache(pid: i32, address: u64) {
    let path = format!("/proc/{}/mem", pid);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Erreur lors de l'ouverture du fichier dans inject_cache");
    file.seek(SeekFrom::Start(address))
        .expect("Erreur lors de la modification du curseur pour l'écriture du code cache");
    file.write(&CACHE_CODE)
        .expect("Erreur lors de l'écriture du code cache dans la mémoire du tracé");
}

/**injects 12 bytes, used to inject trap or reinject origin function bytes
*/
fn inject_12(pid: i32, address: u64, content: [u8; 12]) {
    let path = format!("/proc/{}/mem", pid);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Erreur lors de l'ouverture du fichier dans inject_12");
    file.seek(SeekFrom::Start(address))
        .expect("Erreur lors de la modification du curseur pour l'écriture du code cache");
    file.write(&content)
        .expect("Erreur lors de l'écriture du code cache dans la mémoire du tracé");
}

fn read_12(pid: i32, address: u64, offset: u64) -> [u8; 12] {
    let mut buff: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let path = format!("/proc/{}/mem", pid);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Erreur lors de l'ouverture du fichier dans read_12");
    file.seek(SeekFrom::Start(offset + address))
        .expect("Erreur lors de la modification du curseur pour l'écriture");
    file.read_exact(&mut buff)
        .expect("Erreur lors de la lecture des bytes de la fonction");
    return buff;
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
