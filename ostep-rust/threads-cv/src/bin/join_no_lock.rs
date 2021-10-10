use std::env::args;
use std::process::*;
use std::thread;
use libc::*;

use nix::{unistd::*,sys::wait::*};
static mut m:pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
static mut c:pthread_cond_t = PTHREAD_COND_INITIALIZER;
static mut done:i32 = 0;

pub extern "C" fn child(arg: *mut c_void)->*mut c_void{
    println!("child: begin");
    nix::unitd::sleep(1);
    done = 1;
    println!("child: signal");
    cond_signal(&mut c);
    return 0 as *mut c_void;
}

int main(int argc, char *argv[]) {
    pthread_t p;
    printf("parent: begin\n");
    Pthread_create(&p, NULL, child, NULL);
    Mutex_lock(&m);
    printf("parent: check condition\n");
    while (done == 0) {
	sleep(2);
	printf("parent: wait to be signalled...\n");
	Cond_wait(&c, &m); 
    }
    Mutex_unlock(&m);
    printf("parent: end\n");
    return 0;
}


//
// Main threads
//
fn main(){
    let p:pthread_t = 0;
    println!("parent: begin");
    pthread_create(&mut p,std::ptr::null(),child,0 as *mut c_void);
    mutex_lock(&mut m);
    println!("parent: check conditionn");
    while done ==0{
        nix::unistd::sleep((2);
        println!("parent: wait to be signalled...");
        cond_wait(&mut c,&mut m);
    }
    mutex_unlock(&mut m);
    println!("parent: end");
    }


