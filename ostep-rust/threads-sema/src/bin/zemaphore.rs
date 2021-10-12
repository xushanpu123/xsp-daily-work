extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
use nix::{unistd::*,sys::wait::*};
static mut s:zem_t = 0;
unsafe pub extern "C" fn child(arg:*mut c_void)->*mut c_void {
    nix::unistd::sleep(4);
    println("child\n");
    zem_post(&mut s); // signal here: child is done
    return 0 as *mut c_void;
}

unsafe fn main(int argc, char *argv[]) {
    zem_init(&mut s, 0); 
    println("parent: begin\n");
    let c:pthread_t = 0;
    pthread_create(&mut c, std::ptr::null(), child, 0 as *mut c_void);
    zem_wait(&mut s); // wait here for child
    println("parent: end\n");
}
    
