extern crate libc;
extern crate nix;
use std::alloc::{System,Layout,alloc};
use libc::{c_void,pthread_create,pthread_join,pthread_t};
struct myarg_t{
    a:i32,
    b:i32
}
pub extern "C" fn mythread(arg:*mut c_void)->*mut c_void{
    let args = arg as *mut myarg_t;
    unsafe{
    println!("{} {}",(*args).a,(*args).b);
    }
    return 0 as *mut c_void;
}

fn main(){
    let mut p:pthread_t = 0;
    let mut args = myarg_t{
        a:10,
        b:20
    };
    unsafe{
    let rc = pthread_create(&mut p,std::ptr::null(),mythread,&mut args as *const myarg_t as *mut c_void);
    pthread_join(p,0 as *mut *mut c_void);
    }
    println!("done");
}


