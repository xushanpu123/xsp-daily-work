mod context;
mod structs;
mod test;
use crate::println;
pub unsafe fn Test(){
    println!("test!");
    test::Test();
}