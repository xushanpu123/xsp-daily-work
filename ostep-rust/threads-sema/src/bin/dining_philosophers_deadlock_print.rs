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

static mut em_t forks:[i32;5] = [0;5];
static mut print_lock:sem_t = PTHREAD_MUTEX_INITIALIZER;

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
    space(p);
     println!("%d: try {}", p, left(p)); space_end();
    sem_wait(&mut forks[left(p)]);
    space(p); println!("{}: try %d", p, right(p)); space_end();
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
    let arg_t  =  arg as *mut arg_t;

    space(*args.thread_id); println!("{}: start", *args.thread_id); space_end();

    for i in 0..args->num_loops{
	space(*args.thread_id); println!("{}: think", *args.thread_id); space_end();
	think();
	get_forks(*args.thread_id);
	space(*args.thread_id); println!("{}: eat", *args.thread_id); space_end();
	eat();
	put_forks(*args.thread_id);
	space(*args.thread_id); println!("{}: done", *args.thread_id); space_end();
    }
    return 0 as *mut c_void;
}
                                                                             
fn main() {
    let mut argv = args();
    let argc = argv.len();
    if argc != 2{
    let mut stderr = io::stderr();
    stderr.write(b"usage: dining_philosophers <num_loops>\n");
    }
    println!("dining: started\n");
    
    for i 0..5{ 
	sem_init(&mut forks[i], 1);
    }
    sem_init(&mut print_lock, 1);

    let mut p:[pthread_t;5] = [0;5];
    let mut a:[arg_t;5] = [arg_t{num_loops:0,thread_id:0};5];
    for i in 0..4{
	a[i].num_loops = args().nth(1).unwrap().parse::<i32>().unwrap;
	a[i].thread_id = i;
	pthread_create(&mut p[i], std::ptr::null(), philosopher, &mut a[i]);
    }

    for i in 0..5{
	pthread_join(p[i], 0 as *mut *mut c_void);
    } 

    println!("dining: finished\n");
}
