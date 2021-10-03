extern crate nix;
extern crate libc;
use nix::{unistd::*,sys::wait::*};
use std::ptr::null;
use std::ffi::CStr;
fn main()
{
    println!("hello world (pid:{})", getpid());
    match unsafe{fork()} {
        Ok(ForkResult::Parent { child, .. }) => {
            if let Ok(wc) = wait(){
            if let WaitStatus::Exited(pid,exitcode) = wc{
            println!("hello, I am parent of {}(wc:{}) (pid:{})", child, pid,getpid());
            }
            }
        }
        Ok(ForkResult::Child) => {
        println!("hello, I am child (pid:{})",getpid());
        let arg0 = CStr::from_bytes_with_nul(b"wc\0").unwrap();
        let arg1 = CStr::from_bytes_with_nul(b"p3.rs\0").unwrap();
        let arg2 = CStr::from_bytes_with_nul(b"\0").unwrap();
        let myargs:[&CStr;3]=[&arg0,&arg1,&arg2];
        execvp(myargs[0],&myargs);
        sleep(1);
        }
        Err(_) => {
            println!("Fork failed");
            std::process::exit(1);
        }
     }
    /*printf("hello world (pid:%d)\n", (int) getpid());
    int rc = fork();
    if (rc < 0) {
        // fork failed; exit
        fprintf(stderr, "fork failed\n");
        exit(1);
    } else if (rc == 0) {
        // child (new process)
        printf("hello, I am child (pid:%d)\n", (int) getpid());
        char *myargs[3];
        myargs[0] = strdup("wc");   // program: "wc" (word count)
        myargs[1] = strdup("p3.c"); // argument: file to count
        myargs[2] = NULL;           // marks end of array
        execvp(myargs[0], myargs);  // runs word count
        printf("this shouldn't print out");
    } else {
        // parent goes down this path (original process)
        int wc = wait(NULL);
        printf("hello, I am parent of %d (wc:%d) (pid:%d)\n",
	       rc, wc, (int) getpid());
    }
    return 0;*/
}
