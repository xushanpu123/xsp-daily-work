use riscv::register::sstatus::*;
#[repr(C)]
pub struct Context{
    pub regs:[usize;32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}