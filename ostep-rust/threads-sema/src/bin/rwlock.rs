extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
use nix::{unistd::*,sys::wait::*};


struct rwlock_t {
    writelock:sem_t,
    lock:sem_t,
    readers:i32;
} 

fn rwlock_init(lock:*mut rwlock_t) {
    *lock.readers = 0;
    sem_init(&mut *lock.lock, 1); 
    sem_init(&mut *lock.writelock, 1); 
}

fn rwlock_acquire_readlock(lock:*mut rwlock_t) {
    sem_wait(&mut *lock.lock);
    *lock.readers = *lock.readers + 1;
    if *lock.readers == 1
	sem_wait(&mut *lock.writelock);
    sem_post(&mut *lock.lock);
}

fn rwlock_release_readlock(lock:*mut rwlock_t) {
    sem_wait(&mut *lock.lock);
    *lock.readers = *lock.readers-1;
    if *lock.readers == 0
	sem_post(&mut *lock.writelock);
    sem_post(&mut *lock.lock);
}

fn rwlock_acquire_writelock(lock:*mut rwlock_t ) {
    sem_wait(&mut *lock.writelock);
}

fn rwlock_release_writelock(lock:*mut rwlock_t) {
    sem_post(&mut lock->writelock);
}

static mut read_loops:i32 = 0;
static mut write_loops:i32 = 0;
static mut counter:i32 = 0;

static mut mutex:rwlock_t = rwlock_t{writelock:0,
    lock:0,
    readers:0};

fn reader(arg:*mut c_void)->*mut c_void {
    let mut local:i32 = 0;
    for i in 0..read_loops {
	rwlock_acquire_readlock(&mut mutex);
	local = counter;
	rwlock_release_readlock(&mut mutex);
	println("read {}\n", local);
    }
    println("read done: {}\n", local);
    return 0 as *mut c_void;
}

fn writer(arg:*mut c_void)->*mut c_void {
    for i in 0..write_loops{
	rwlock_acquire_writelock(&mut mutex);
	counter = counter + 1;
	rwlock_release_writelock(&mut mutex);
    }
    println("write done\n");
    return 0 as *mut c_void;
}

int main(int argc, char *argv[]) {
   let mut argv = args();
    let argc = argv.len();
    if argc != 3{
    let mut stderr = io::stderr();
    stderr.write(b"usage: rwlock readloops writeloops\n");
    std::process::exit(1);
    }
    read_loops = args().nth(1).unwrap().parse::<i32>().unwrap();
    write_loops = args().nth(2).unwrap().parse::<i32>().unwrap();
    
    rwlock_init(&mutex); 
    pthread_t c1, c2;
    Pthread_create(&c1, NULL, reader, NULL);
    Pthread_create(&c2, NULL, writer, NULL);
    Pthread_join(c1, NULL);
    Pthread_join(c2, NULL);
    printf("all done\n");
    return 0;
}
    

