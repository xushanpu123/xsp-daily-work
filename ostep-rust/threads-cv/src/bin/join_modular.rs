extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;

use nix::{unistd::*,sys::wait::*};
static mut proc_info_lock:pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
struct synchronizer_t{
    c:pthread_cond_t,
    m:pthread_mutex_t,
    done:i32
}
static mut s:synchronizer_t = synchronizer_t{
    c:0,
    m:PTHREAD_MUTEX_INITIALIZER,
    done:0
};

fn sysn_init(s:*mut synchronizer_t){
    (*s).done = 0;
    pthread_mutex_init((*s).m);
    cond_init((*s).c);
}
fn sync_signal(s:*mut synchronizer_t){
    pthread_mutex_init((*s).m);
    (*s).done = 1;
    cond_signal((*s).c);
    pthread_mutex_unlock((*s).m);
}

fn sync_wait(s:*mut synchronizer_t){
    pthread_mutex_lock((*s).m);
    while(*s).done == 0{
        cond_wait((*s),c,(*s).m);
        (*s).done = 0;
        pthread_mutex_unlock((*s).m);
    }
}
pub extern "C" fn child(arg: *mut c_void)->*mut c_void{
    println!("child");
    nix::unistd::sleep(1);
    sync_signal(&mut s);
    return 0 as *mut c_void;
}
//
// Main threads
//
fn main(){
    let p:pthread_t = 0;
    println!("parent: begin");
    sync_init(&mut s);
    pthread_create(&mut p,std::ptr::null(),child,0 as *mut c_void);
    sync_wait(&mut s);
    println!("parent: end");
}


