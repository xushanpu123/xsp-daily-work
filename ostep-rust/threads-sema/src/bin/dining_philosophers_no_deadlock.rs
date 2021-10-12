extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;

use nix::{unistd::*,sys::wait::*};

struct arg_t{
    num_loops:i32;
    thread_id:i32;
} 

static mut sem_t forks:[i32;5] = [0;5];

fn space(s:i32) {
    sem_wait(&mut print_lock);
    for i in 0..(s*10)
	println(" ");
}

fn space_end() {
    sem_post(&mut print_lock);
}

fn left(p:i32) ->i32 {
    return p;
}

fn right(p:i32)->i32 {
    return (p + 1) % 5;
}



fn get_forks(p:i32) {
    if p == 4{
	sem_wait(&mut forks[right(p)]);
	sem_wait(&mut forks[left(p)]);
    } else {
	sem_wait(&mut forks[left(p)]);
	sem_wait(&mut forks[right(p)]);
    }
}


fn put_forks(p:i32) {
    sem_post(&mut forks[left(p)]);
    sem_post(&mut forks[right(p)]);
}

vfn think() {
    return;
}

fn eat() {
    return;
}

unsafe pub extern "C" fn philosopher(arg:*mut c_void)->*mut c_void {
    let args = arg as *mut arg_t;

    let p = *args.thread_id;
    for 0..(*args).num_loops {
	think();
	get_forks(p);
	eat();
	put_forks(p);
    }
    return 0 as *mut c_void;
}

fn main(){
    let mut argv = args();
    let argc = argv.len();
    if argc != 1{
    let mut stderr = io::stderr();
    stderr.write(b"usage: dining_philosophers <num_loops>\n");
    std::process::exit(1);
    }
    println!("dining: started\n");
    
    for i in 0..5{
    sem_init(&mut print_lock, 1);
    }

    let mut p:[pthread_t;5] = [0;5];
    let mut a:[arg_t;5] = [0;5];
    for i in 0..5{
	a[i].num_loops = args().nth(1).unwrap().parse::<i32>().unwrap();
	a[i].thread_id = i;
	pthread_create(&mut p[i], std::ptr::null(), philosopher, &mut a[i]);
    }

    for i in 0..5{
	pthread_join(p[i], 0 as *mut *mut c_void); 
    }

    println("dining: finished\n");
}                                                                          

