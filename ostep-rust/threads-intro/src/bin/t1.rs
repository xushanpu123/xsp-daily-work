extern crate libc;
extern crate nix;
use std::alloc::{System,Layout,alloc};
use libc::{c_void,pthread_create,pthread_join,pthread_t};
use std::env::args;
use std::io::{self,Write};
static mut max:i32 = 0;
static mut counter:i32 = 0;
pub extern "C" fn mythread(arg:*mut c_void)->*mut c_void{
    let letter = arg as *mut &str;
    let i:i32 = 0;
    unsafe{
    println!("{}: begin [addr of i: {}]", *letter, &i as *const i32 as usize);
    for i in 0..max{
        counter = counter + 1;
    }
    println!("{}: done",*letter);
}
    return 0 as *mut c_void;
}

fn main(){
    println!("{}",args().len());
    if args().len()!=2{
        let mut stderr = io::stderr();
        stderr.write(b"usage: main-first <loopcount>\n");
        std::process::exit(1);
    }
    else{
    unsafe{
        max = args().nth(1).unwrap().parse::<i32>().unwrap();
    }
    let mut p1:pthread_t = 0;
    let mut p2:pthread_t = 0;
    let mut A = "A";
    let mut B = "B";
    unsafe{
        println!("main: begin [counter = {}] [{}]\n", counter, &counter as *const i32 as usize);
        pthread_create(&mut p1,std::ptr::null(),mythread,&A as *const &str as *mut c_void);
        pthread_create(&mut p2,std::ptr::null(),mythread,&B as *const &str as *mut c_void);
        pthread_join(p1,0 as *mut *mut c_void);
        pthread_join(p2,0 as *mut *mut c_void);
        println!("main: done\n [counter: {}]\n [should: {}]",counter,max*2);
    }
}
}
/*int main(int argc, char *argv[]) {                    
    pthread_t p1, p2;
    printf("main: begin [counter = %d] [%x]\n", counter, 
	   (unsigned int) &counter);
    Pthread_create(&p1, NULL, mythread, "A"); 
    Pthread_create(&p2, NULL, mythread, "B");
    // join waits for the threads to finish
    Pthread_join(p1, NULL); 
    Pthread_join(p2, NULL); 
    printf("main: done\n [counter: %d]\n [should: %d]\n", 
	   counter, max*2);
    return 0;
}*/
