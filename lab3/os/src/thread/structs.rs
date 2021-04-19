use super::context::*;
use crate::println;
#[repr(C)]
pub struct Thread{
    pub context:Context,
    pub Kernalstack:usize
}

impl Thread{
    pub unsafe fn switch_to(&mut self,task:&mut Thread){
        //println!("{}",(*(task.context.addr as *mut Contextcontent)).ra);
        self.context.switch(&mut task.context);
    }
    pub unsafe fn new(x:[usize;12],ra:usize,stack_top:usize)->Self{
        Contextcontent::new(x,ra).new_thread(stack_top)
    }
}