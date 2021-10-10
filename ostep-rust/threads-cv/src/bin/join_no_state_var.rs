extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;

use nix::{unistd::*,sys::wait::*};
static mut c:pthread_cond_t = PTHREAD_COND_INITIALIZER;
static mut m:pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

fn child(arg:*mut c_void)->*mut c_void {
    println!("child: begin");
    mutex_lock(&mut m);
    println!("child: signal");
    cond_signal(&mut c);
    mutex_unlock(&mut m);
    return 0 as *mut c_void;
}
fn main() {
    let p:pthread_t = 0;
    println!("parent: begin");
    pthread_create(&mut p, std::ptr::null(), child, 0 as *mut c_void);
    nix::unistd::sleep(2);
    println("parent: wait to be signalled...\n");
    mutex_lock(&mut m);
    cond_wait(&mut c, &mut m); 
    mutex_unlock(&mut m);
    println("parent: end\n");
}
