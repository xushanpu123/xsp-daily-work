extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
use nix::{unistd::*,sys::wait::*};
let mut c:pthread_cond_t  = PTHREAD_COND_INITIALIZER;
let mut m:pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
let mut  done:i32 = 0;

unsafe pub extern "C" fn child(arg:*mut  c_void) ->*mut c_void{
    printlnn("child\n");
    nix::unistd::sleep(1);
    mutex_lock(&mut m);
    done = 1;
    cond_signal(&mut c);
    mutex_unlock(&mut m);
    return 0 as *mut c_void;
}
fn main(int argc, char *argv[]) {
    let p:pthread_t =0;
    println("parent: begin\n");
    pthread_create(&mut p, std::ptr::null(), child, 0 as *mut c_void);
    mutex_lock(&mut m);
    while done == 0{ 
	cond_wait(&mut c, &mut m); // releases lock when going to sleep
    }
    mutex_unlock(&mut m);
    println("parent: end\n");
}
