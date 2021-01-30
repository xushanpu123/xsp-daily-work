mod consts;
mod structs;
pub use structs::*;
mod task_test;
mod schuduler;
mod thread_pool;
use crate::println;
pub unsafe fn test(){
    
    task_test::task_test();
}
pub type Tid = usize;
pub type ExitCode = usize;

