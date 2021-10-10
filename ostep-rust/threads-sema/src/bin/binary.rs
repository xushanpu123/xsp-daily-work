extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;

use nix::{unistd::*,sys::wait::*};
static mut mutex:sem_t =PTHREAD_MUTEX_INITIALIZER;
static mut counter:i32 = 0;

pub extern "C" fn child(arg:*mut c_void)->*mut c_void {
    let mut  i = 0;
    for i in 0..1000000 {
	sem_wait(&mut mutex);
	counter = counter + 1;
	sem_post(&mut mutex);
    }
    return 0 as *mut c_void;
}

fn main() {
    unsafe{
    sem_init(&mut mutex, 1); 
    let mut c1:pthread_t = 0;
    let mut c2:pthread_t = 0;
    pthread_create(&mut c1, std::ptr::null(), child, 0 as *mut c_void);
    pthread_create(&mut c2, std::ptr::null(), child, 0 as *mut c_void);
    pthread_join(c1, 0 as *mut *mut c_void);
    pthread_join(c2, 0 as *mut *mut c_void);
    println!("result: {} (should be 20000000)\n", counter);
    }
}
    
