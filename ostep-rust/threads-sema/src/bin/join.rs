extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
sem_t s;

fn child(arg:*mut c_void )->*mut c_void {
    nix::unistd::sleep(2);
    println("child\n");
    sem_post(&mut s); // signal here: child is done
    return 0 as *mut c_void;
}

fn main() {
    sem_init(&mut s, 0); 
    println("parent: begin\n");
    let mut c:pthread_t = 0;
    pthread_create(&mut c, std::ptr::null(), child, 0 as *mut c_void);
    sem_wait(&mut s); // wait here for child
    println("parent: end\n");
}
    
