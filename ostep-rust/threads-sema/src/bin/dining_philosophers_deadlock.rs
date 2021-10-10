extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;

use nix::{unistd::*,sys::wait::*};


struct arg_t{
    num_loops:i32,
    thread_id:i32
} 

static mut forks:[sem_t;5] = [0;5];

fn left(p:i32)->i32  {
    return p;
}

fn right(p:i32)->i32 {
    return (p + 1) % 5;
}

fn get_forks(p:i32) {
    sem_wait(&mut forks[left(p)]);
    sem_wait(&mut forks[right(p)]);
}

fn put_forks(p:i32) {
    sem_post(&mut forks[left(p)]);
    sem_post(&mut forks[right(p)]);
}

fn think() {
    return;
}

fn eat() {
    return;
}

fn philosopher(arg:*mut c_void)->*mut c_void {
    let args =  arg as *mut arg_t;
    let mut  p = *argshread_id;

    
    for in 0..(*args).num_loops{
	think();
	get_forks(p);
	eat();
	put_forks(p);
    }
    return 0 as *mut c_void;
}
                                                                             
fn main() {
    let mut argv = args();
    let argc = argv.len();
    if argc != 1{
    let mut stderr = io::stderr();
    stderr.write(b"usage: dining_philosophers <num_loops>\n");
    }
    printf("dining: started\n");
    
    
    for i in 0..5
	sem_init(&mut forks[i], 1);

    let mut p:[pthread_t;5] =  [0;5];
    let mut a:[arg_t;5] = [0;5];
    for i in 0..4 {
	a[i].num_loops = args().nth(1).unwrap().parse::<i32>().unwrap();
	a[i].thread_id = i;
	pthread_create(&mut p[i], std::ptr::null(), philosopher, &mut a[i]);
    }

    for i in 0..5 
	pthread_join(p[i], NULL); 

    println!("dining: finished\n");
}
