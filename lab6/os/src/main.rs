
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]
mod panic;
mod sbi;
mod console;
mod interrupt;
mod task;
mod consts;
mod init;
global_asm!(include_str!("boot/entry.asm"));
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("helloworld!");
    interrupt::init();
    println!("111");
    /*unsafe {
    task::test();
    }*/
    init::dynamic_allocating_test();
    println!("111");
    unsafe {
        llvm_asm!("ebreak"::::"volatile");
    }
    //interrupt::init_interrupt();
    loop{};
}

