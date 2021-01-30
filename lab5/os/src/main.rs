
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(asm)]
#![feature(naked_functions)]
mod panic;
mod sbi;
mod console;
mod interrupt;
mod task;
global_asm!(include_str!("boot/entry.asm"));
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    crate::interrupt::init();

    extern "C" {
        fn end();
    }
    
    crate::task::init();
    crate::timer::init();
    crate::task::run();
    loop {}
}


