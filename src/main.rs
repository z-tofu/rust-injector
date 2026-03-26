use std::ffi::c_void;
use sysinfo::System;
use windows::{
    Win32::{Foundation::{CloseHandle, HANDLE}, System::{Diagnostics::Debug::WriteProcessMemory, LibraryLoader::{GetModuleHandleA, GetProcAddress}, Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx}, Threading::{CreateRemoteThread, GetExitCodeThread, OpenProcess, PROCESS_ALL_ACCESS}}}, core::s
};

fn main() {
    let target_process = "Target.exe";
    let dll_path = "Path\\To\\.dll\0";

    let mut system = System::new_all();
    system.refresh_all();

    let pid = system
        .processes()
        .values()
        .find(|p| p.name().to_lowercase() == target_process.to_lowercase())
        .map(|p| p.pid().as_u32())
        .expect("Could not find target_process");

    println!("[+] found {} with PID: {}", target_process, pid);

    unsafe {
        let process_handle: HANDLE = OpenProcess(PROCESS_ALL_ACCESS, false, pid)
            .expect("failed to open");   

        let remote_mem = VirtualAllocEx(
            process_handle,
            None,
            dll_path.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,

        );

        if remote_mem.is_null() {
            panic!("failed to alloc memory");
        }

        WriteProcessMemory(
            process_handle,
            remote_mem,
            dll_path.as_ptr() as *const c_void,
            dll_path.len(),
            None,
            ).expect("failed to write process memory");

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
            ).expect("failed to create remote thread");

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
