extern crate libc;
extern crate nix;
use std::io::{self,Write};
use std::env::args;
use std::process::*;
use std::thread;
use libc::*;
use nix::{unistd::*,sys::wait::*};
struct pr_thread_t{
    Tid:pthread_t,
    State:i32
}
fn PR_CreateThread()

pr_thread_t *PR_CreateThread(void *(*start_routine)(void *)) {
    pr_thread_t *p = malloc(sizeof(pr_thread_t));
    if (p == NULL) 
	return NULL;
    p->State = PR_STATE_INIT;
    Pthread_create(&p->Tid, NULL, start_routine, NULL); 
    // turn the sleep off to avoid the fault, sometimes...
    sleep(1);
    return p;
}

void PR_WaitThread(pr_thread_t *p) {
    Pthread_join(p->Tid, NULL); 
}

pr_thread_t *mThread;

void *mMain(void *arg) {
    printf("mMain: begin\n");
    int mState = mThread->State;
    printf("mMain: state is %d\n", mState);
    return NULL;
}


int main(int argc, char *argv[]) {
    printf("ordering: begin\n");
    mThread = PR_CreateThread(mMain);
    PR_WaitThread(mThread);
    printf("ordering: end\n");
    return 0;
}

