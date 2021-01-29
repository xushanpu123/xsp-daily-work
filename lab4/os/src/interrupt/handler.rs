extern crate riscv;
use riscv::register::{mtvec::TrapMode::Direct,stvec, sstatus::Sstatus, scause::{Scause,Trap,Exception,Interrupt}};
use super::{trapframe::{self, Trapframe}, timer::ticks};
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
pub fn handle_interrupt(context:&mut Trapframe,scause:Scause,stval:usize){
    match scause.cause() {
        // 断点中断（ebreak）
        Trap::Exception(Exception::Breakpoint) => breakpoint(context),
        Trap::Interrupt(Interrupt::SupervisorTimer) => supervisor_timer(context),
        _=>{}
    }
}
fn breakpoint(context:&mut Trapframe){
    println!("ebreak!");
    context.sepc+=2;
}
fn supervisor_timer(context:&mut Trapframe){
    ticks();
}