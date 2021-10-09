extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
use nix::{unistd::*,sys::wait::*};
static mut proc_info_lock:pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
struct proc_t{
    pid:i32,
}
struct thread_info_t{
    proc_info: *mut proc_t
}

static mut p:proc_t = proc_t{pid:0};
static mut th:thread_info_t = thread_info_t{proc_info:0 as *mut proc_t};
static mut thd:*mut thread_info_t = 0 as *mut thread_info_t;

pub extern "C" fn thread1(agr:*mut c_void)->*mut c_void{
    println!("t1:before check");
    unsafe{
    pthread_mutex_lock(&mut proc_info_lock);
    if (*thd).proc_info != 0 as *mut proc_t{
        println!("t1: after check");
        nix::unistd::sleep(2);
        println!("t1: use!");
        println!("{}",(*(*thd).proc_info).pid);
    }
    pthread_mutex_unlock(&mut proc_info_lock);
}
    return 0 as *mut c_void;
}
pub extern "C" fn thread2(arg:*mut c_void)->*mut c_void{
    println!("                 t2: begin");
    nix::unistd::sleep(1);
    unsafe{
    pthread_mutex_lock(&mut proc_info_lock);
    println!("                 t2: set to NULL");
    (*thd).proc_info = 0 as *mut proc_t;
    pthread_mutex_unlock(&mut proc_info_lock);
    }
    return 0 as *mut c_void;
}

fn main() {
    let mut argv = args();
    let argc = argv.len();
    if argc != 1{
    let mut stderr = io::stderr();
    stderr.write(b"usage: cpu <string>\n");
    }
    else{
       let mut t = thread_info_t{proc_info:0 as *mut proc_t};
       unsafe{
       p.pid = 100;
       t.proc_info = &p as *const proc_t as *mut proc_t;
       thd = &t as *const thread_info_t as *mut thread_info_t;
       let mut p1:pthread_t = 0;
       let mut p2:pthread_t = 0;
       println!("main: begin");
       pthread_create(&mut p1,std::ptr::null(),thread1,0 as *mut c_void);
       pthread_create(&mut p2,std::ptr::null(),thread2,0 as *mut c_void);
       pthread_join(p1,0 as *mut *mut c_void);
       pthread_join(p2,0 as *mut *mut c_void);
       }
       println!("main: end");
    }
    }




