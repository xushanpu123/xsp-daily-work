use super::structs::*;
use super::test::*;
use crate::println;
#[repr(C)]
pub struct Contextcontent{
    pub x:[usize;12],
    pub ra:usize
}

pub struct Context{
    pub addr:usize
}

impl Contextcontent{
    pub fn new(x:[usize;12],ra:usize)->Self{
        Contextcontent{
            x,
            ra
        }
    }
    pub unsafe fn load_context(self,addr:usize)->Context{
        let ptr = addr as *mut Contextcontent;
        ptr.sub(1);
        *ptr = self;
        Context{addr:addr-13*8}
    }
    pub unsafe fn new_thread(self,stack_top:usize)->Thread{
        self.load_context(stack_top-13*8);
        Thread{
            context:Context::new(stack_top),
            Kernalstack:stack_top
        }
    }
}

impl Context{
    #[naked]
    #[inline(never)]
    pub unsafe extern "C" fn switch(&mut self, target: &mut Context) {
        
        llvm_asm!(include_str!("./switch.asm")::::"violate");
    }
    pub fn new(addr:usize)->Self{
        Context{addr}
    }
}