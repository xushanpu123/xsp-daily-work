extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
use nix::{unistd::*,sys::wait::*};

static mut max:i32 = 0;
static mut loops:i32 = 0;
static mut buffer:[i32;1000] = [0;1000]; 

static mut use_ptr:i32  0;
static mut fill_ptr:i32 = 0;
static mut num_full:i32 = 0;

static mut cv:= PTHREAD_COND_INITIALIZER;
static mut m:pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

static mut consumers:i32 = 1;
static mut verbose:i32 = 1;


fn do_fill(value:i32) {
    buffer[fill_ptr] = value;
    fill_ptr = (fill_ptr + 1) % max;
    num_full = num_full + 1;
}

fn do_get()->i32 {
    let tmp = buffer[use_ptr];
    use_ptr = (use_ptr + 1) % max;
    num_full = num_full - 1;
    return tmp;
}

unsafe pub extern "C" fn producer(arg:*mut c_void)->*mut c_void {
    for i in 0..loops {
	mutex_lock(&mut m);            // p1
	while num_full == max{    // p2
	    cond_wait(&mut cv, &mut m);    // p3
    }
	do_fill(i);                // p4
	cond_signal(&mut cv);          // p5
	mutex_unlock(&mut m);          // p6
    }

    // end case: put an end-of-production marker (-1) 
    // into shared buffer, one per consumer
    for i in 0..consumers{
	mutex_lock(&mut m);
	while num_full == max 
	    cond_wait(&mut cv, &mut m);
	do_fill(-1);
	cond_signal(&mut cv);
	mutex_unlock(&mut m);
    }

    return 0 as *mut c_void;
}
                                                                               
unsafe pub extern "C" fn consumer(arg:*mut c_void)->*mut c_void {
    let  tmp:i32 = 0;
    // consumer: keep pulling data out of shared buffer
    // until you receive a -1 (end-of-production marker)
    while tmp != -1 { 
	mutex_lock(&mut m);           // c1
	while num_full == 0{     // c2 
	    Cond_wait(&cv, &m);   // c3
    }
	tmp = do_get();           // c4
	cond_signal(&mut cv);         // c5
	mutex_unlock(&mut m);         // c6
    }
    return 0 as *mut c_void;
}


fn main()
{

    let mut argv = args();
  let argc = argv.len();
  if argc != 4{
  let mut stderr = io::stderr();
  stderr.write(b"usage: cpu <string>\n");
  }
  else{
    max = args().nth(1).unwrap().parse::<i32>().unwrap();
    loops = args().nth(2).unwrap().parse::<i32>().unwrap();
    consumers = args().nth(3).unwrap().parse::<i32>().unwrap();

    buffer = alloc(Layout::new::<[i32;4]>());

    for i in 0..max{
	buffer[i] = 0;
    }

    let pid:pthread_t = 0;
    let cid:[pthread_t;consumers] = [0;consumers];
    pthread_create(&mut pid, std::ptr::null(), producer, 0 as *mut c_void); 
    let i:i32 = 0;
    for i in 0..consumers {
	pthread_create(&mut cid[i], std::ptr::null(), consumer,  i as *const i32 as *mut c_void); 
    }
    pthread_join(pid, 0 as *mut *mut c_void); 
    for i in 0..consumers{
	pthread_join(cid[i], 0 as *mut *mut c_void); 
    }
 
}
}

