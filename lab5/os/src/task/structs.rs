#![feature(llvm_asm)]
use super::consts::*;
use crate::interrupt::Trapframe;
use riscv::register::satp;
#[repr(C)]
pub struct ContextContent {
    pub ra: usize,
    satp: usize,
    s: [usize; 12],
}

#[repr(C)]
pub struct Context{                         //store the current sp when switched
    pub content_addr: usize,
}
impl Context{
    #[naked]
    #[inline(never)]
    pub unsafe extern "C" fn switch(&mut self, target: &mut Context) {
        llvm_asm!(include_str!("./switch.asm")::::"violate");
    }
    pub unsafe fn new_kernel_thread(
        entry: usize,
        kstack_top: usize,
        satp: usize
        ) -> Context {
        ContextContent::new_kernel_thread(entry, kstack_top, satp).push_at(kstack_top)
    }
}
pub struct KernelStack(pub usize);
#[repr(C)]
pub struct Task{
    // 线程的状态
    pub context: Context,
    // 线程的栈
    pub kstack: KernelStack,
}
impl Task {
    /*pub fn new(context:Context,kstack:KernelStack)->Self{
        Self{
            context,
            kstack
        }
    }*/
    pub fn switch_to(&mut self,to_task:&mut Task){
        unsafe {
        self.context.switch(&mut to_task.context);
        }
    }
    pub unsafe fn new(entry:usize,kstack:usize)->Self{
        Self{
            context:Context::new_kernel_thread(entry, kstack, satp::read().bits()),
            kstack:KernelStack(kstack)
        }
    }
}
impl ContextContent {
    // 为一个新内核线程构造栈上的初始状态信息
    // 其入口点地址为 entry ，其内核栈栈顶地址为 kstack_top ，其页表为 satp
    fn new_kernel_thread(
        entry: usize,
        kstack_top: usize,
        satp: usize,
        ) -> ContextContent {

        let mut content = ContextContent {
            ra: entry,
            satp,
            s: [0; 12],
            
        };
        content
    }
    // 将自身压到栈上，并返回 Context
    unsafe fn push_at(self, stack_top: usize) -> Context {
        let ptr = (stack_top as *mut ContextContent).sub(1);
        *ptr = self;
        Context { content_addr: ptr as usize }
    }
}