extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
use nix::{unistd::*,sys::wait::*};

static mut s:sem_t = 0;

fn child(arg:*mut c_void)-<*mut c_void {
    sem_wait(&mut s); 
    println("child {}\n", arg as i128);
    nix::unistd::sleep(1);
    sem_post(&mut s); 
    return 0 as *mut c_void;
}

fn main() {
   let mut argv = args();
    let argc = argv.len();
    if argc != 3{
    let mut stderr = io::stderr();
    stderr.write(b"usage: throttle <num_threads> <sem_value>\n");
    }
    let mut  num_threads = args().nth(1).unwrap().parse::<i32>().unwrap();
    let mut  sem_value = args().nth(2).unwrap().parse::<i32>().unwrap();
    
    sem_init(&mut s, sem_value); 

    println("parent: begin\n");
    let mut c:[pthread_t;num_threads] =  [0;num_threads];

    let mut i:i32 = 0;
    for i in 0..num_threads{ 
	pthread_create(&mut c[i], std::ptr::null(), child, &i as *const i128 as *mut c_void);
    }

    for i in 0..num_threads{
	Pthread_join(c[i], 0 as *mut *mut c_void);
    }
    println("parent: end\n");
}
    
