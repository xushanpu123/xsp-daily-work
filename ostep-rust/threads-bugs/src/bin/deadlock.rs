extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
use nix::{unistd::*,sys::wait::*};
static mut L1:pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
static mut L2:pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
pub extern "C" fn thread1(arg:*mut c_void)->*mut  c_void{
    println!("t1:  begin");
    println!("t1: try to acquire L1...");
    unsafe{
    pthread_mutex_lock(&mut L1);
    nix::unistd::sleep(1);
    println!("t1: L1 acquired");
    println!("t1: try to acquire L2...");
    pthread_mutex_lock(&mut L2);
    println!("t1: L2 acquired");
    pthread_mutex_unlock(&mut L1);
    pthread_mutex_unlock(&mut L2);
    }
    return 0 as *mut c_void;
}
pub extern "C" fn thread2(arg:*mut c_void)->*mut c_void{
    println!("                           t2: begin");
    println!("                           t2: try to acquire L2...");
    unsafe{
    pthread_mutex_lock(&mut L2);
    println!("                           t2: L2 acquired");
    println!("                           t2: try to acquire L1..");
    pthread_mutex_lock(&mut L1);
    println!("                           t2: L1 acquired");
    pthread_mutex_unlock(&mut L1);
    pthread_mutex_unlock(&mut L2);
    }
    return 0 as *mut c_void;
}
fn main(){
    let mut argv = args();
    let argc = argv.len();
    if argc != 1{
    let mut stderr = io::stderr();
    stderr.write(b"usage: cpu <string>\n");
    std::process::exit(1);
    }
    else{
      let mut p1:pthread_t = 0;
      let mut p2:pthread_t = 0;
      println!("main: begin");
      unsafe{
        pthread_create(&mut p1, std::ptr::null(), thread1, 0 as *mut c_void); 
        pthread_create(&mut p2, std::ptr::null(), thread2, 0 as *mut c_void); 
    // join waits for the threads to finish
    pthread_join(p1, 0 as *mut *mut c_void); 
    pthread_join(p2, 0 as *mut *mut c_void); 
      }
      println!("main: end");
    }
}


