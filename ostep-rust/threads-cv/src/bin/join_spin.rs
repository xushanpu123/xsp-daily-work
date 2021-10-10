extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;

use nix::{unistd::*,sys::wait::*};
static mut done:i32 = 0;

fn child(arg:*mut c_void)->*mut c_void {
    println("child\n");
    nix::unistd::sleep(5);
    done = 1;
    return 0 as *mut c_void;
}

fn main() {
    let p:pthread_t = 0;
    println("parent: begin\n");
    pthread_create(&mut p, std::ptr::null(), child, 0 as *mut c_void);
    while done == 0{}
	; // spin
    println("parent: end\n");
}
