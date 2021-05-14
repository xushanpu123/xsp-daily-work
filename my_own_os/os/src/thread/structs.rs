use super::context::*;
use crate::println;
use core::marker::Copy;
pub type TID = usize;
extern crate lazy_static;
#[macro_use]
use lazy_static::lazy_static;
macro_rules! void_thread {
    () => {
        Thread{
            tid:0,
            context:Context{addr:0},
            Kernalstack:0
        }
    };
}
pub static mut threadspool:[Thread;20] = [void_thread!();20];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Thread{
    pub tid:usize,
    pub context:Context,
    pub Kernalstack:usize
}

impl Thread{
    pub unsafe fn switch_to(&mut self,task:&mut Thread){
        self.context.switch(&mut task.context);
    }
    pub unsafe fn new(x:[usize;12],ra:usize,stack_top:usize)->Self{
        Contextcontent::new(x,ra).new_thread(stack_top)
    }
    pub unsafe fn alloc_TID()->TID{
        for i in 0..20usize{
            if threadspool[i].tid == 0{
                return (i+1);
            }
        }
    
        return 0;
    }
}