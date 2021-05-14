use core::panic::PanicInfo;
use crate::println;
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("panic");
    loop {}
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!("abort!");
}