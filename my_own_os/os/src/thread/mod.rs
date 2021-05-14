mod context;
mod structs;
mod test;
mod processor;
use crate::println;
pub unsafe fn Test(){
    println!("test!");
    test::Test();
}