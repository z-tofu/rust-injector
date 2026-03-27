use std::ffi::c_void;
use std::env;
use std::process;
use sysinfo::System;
use windows::{
    Win32::{Foundation::{CloseHandle, HANDLE}, System::{Diagnostics::Debug::WriteProcessMemory, LibraryLoader::{GetModuleHandleA, GetProcAddress}, Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx}, Threading::{CreateRemoteThread, GetExitCodeThread, OpenProcess, PROCESS_ALL_ACCESS}}}, core::s
};

struct Config {
    target_process: String,
    dll_path: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("not enough args");
        }
        let target_process = args[1].clone();
        let mut dll_path = args[2].clone();
        dll_path.push_str("\0");

        Ok(Config { target_process, dll_path })
        
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|e| {
        println!("Problem passing args: {e}");
        process::exit(1);
    });

    let mut system = System::new_all();
    system.refresh_all();

    let pid = system
        .processes()
        .values()
        .find(|p| p.name().to_lowercase() == config.target_process.to_lowercase())
        .map(|p| p.pid().as_u32())
        .expect("Could not find target process");

    println!("[+] found {} with PID: {}", config.target_process, pid);

    unsafe {
        let process_handle: HANDLE = OpenProcess(PROCESS_ALL_ACCESS, false, pid)
            .expect("Failed to open");   

        let remote_mem = VirtualAllocEx(
            process_handle,
            None,
            config.dll_path.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,

        );

        if remote_mem.is_null() {
            panic!("Failed to alloc memory");
        }

        WriteProcessMemory(
            process_handle,
            remote_mem,
            config.dll_path.as_ptr() as *const c_void,
            config.dll_path.len(),
            None,
            ).expect("Failed to write process memory");

        let kernel32 = GetModuleHandleA(s!("kernel32.dll")).unwrap();
        let load_library_addr = GetProcAddress(kernel32, s!("LoadLibraryA")).expect("failed to load LoadLibraryA");

        println!("[+] Spawning remote thread...");
        
        let thread_handle = CreateRemoteThread(
            process_handle,
            None,
            0,
            Some(std::mem::transmute(load_library_addr)),
            Some(remote_mem),
            0,
            None,
            ).expect("Failed to create remote thread");

        println!("[+] Injection successful");

        println!("[+] Waiting for remote thread...");

        let mut exit_code: u32 = 0;
        GetExitCodeThread(thread_handle, &mut exit_code).expect("Failed to get code");

        if exit_code == 0 {
            println!("[-] Error. LoadLibraryA returned 0. DLL failed to load");
        } else {
            println!("[+] Success: DLL loaded at address 0x{:X}", exit_code);
        }

        CloseHandle(thread_handle).ok();
        CloseHandle(process_handle).ok();
    }

}
