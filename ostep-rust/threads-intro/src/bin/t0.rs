extern crate libc;
extern crate nix;
use std::alloc::{System,Layout,alloc};
use libc::{c_void,pthread_create,pthread_join,pthread_t};
use std::env::args;
use std::io::{self,Write};
pub extern "C" fn mythread(arg:*mut c_void)->*mut c_void{
    let p = arg as *mut &str;
    unsafe{
    println!("{}",(*p));
    }
    return 0 as *mut c_void;
}
fn main(){
    let mut argv = args();
    let argc = argv.len();
    if argc != 1{
         let mut stderr = io::stderr();
         stderr.write(b"usage: main\n");
         std::process::exit(1);
   }
    else{
         let mut p1:pthread_t = 0;
         let mut p2:pthread_t = 0;
         println!("main: begin");
         let A = "A";
         let B = "B";
         unsafe{
         pthread_create(&mut p1,std::ptr::null(),mythread, &A as *const &str as *mut c_void);
         pthread_create(&mut p2,std::ptr::null(),mythread, &B as *const &str as *mut c_void);
         pthread_join(p1,0 as *mut *mut c_void);
         pthread_join(p2,0 as *mut *mut c_void);
         }
         println!("main: end");
  }
}



