use crate::sbi;
use riscv::register::{time,sie,sstatus};
use crate::println;
const INTERVAL:usize = 100000;
fn set_next_timer_intr(){
    sbi::set_time(time::read()+INTERVAL);
    
}
pub fn init(){
    println!("timer init!");
    unsafe {
        // 开启 STIE，允许时钟中断
        sie::set_stimer(); 
        // 开启 SIE（不是 sie 寄存器），允许内核态被中断打断
        sstatus::set_sie();
    }
    set_next_timer_intr();
}
pub fn ticks(){
    println!("timer interrupt!");
    set_next_timer_intr();
}