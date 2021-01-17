
use super::consts::*;
#[repr(C)]
pub struct ContextContent {
    pub ra: usize,
    s: [usize; 12],
    //tf: TrapFrame,
}

#[repr(C)]
pub struct Context {
    pub content_addr: usize,
}

pub struct KernelStack(usize);
impl KernelStack {
    pub fn new() -> Self {
        let bottom = alloc_new_stack();
            
        
        KernelStack(bottom)
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        delloc_stack(self.0);
   }
}

pub struct task{
    // 线程的状态
    pub context: Context,
    // 线程的栈
    pub kstack: KernelStack,
}

