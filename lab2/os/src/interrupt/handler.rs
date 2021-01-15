extern crate riscv;
use riscv::register::{mtvec::TrapMode::Direct,stvec, sstatus::Sstatus, scause::{Scause,Trap,Exception,Interrupt}};
use super::context::Context;
#[macro_use]
use crate::println;
global_asm!(include_str!("./interrupt.asm"));
pub fn init(){
    unsafe {
        extern "C" {
            /// `interrupt.asm` 中的中断入口
            fn __interrupt();
        }
        
    stvec::write(__interrupt as usize,Direct);
    }
}
#[no_mangle]
pub fn handle_interrupt(context:&Context,scause:Scause,stval:usize){
      println!("ebreak!");
}