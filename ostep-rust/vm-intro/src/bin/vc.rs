use std::alloc::{System,Layout};
use std::num::NonZeroUsize;
fn main(){
    println!("location of code : {}\n", main as usize);
    let b = unsafe{
        System.alloc(Layout{size_:100000,align_:NonZeroUsize::new(1).unwrap()});
    };
    println!("location of heap : {}\n", b as usize);
    println!("location of heap : %p\n", )
    //int x = 3;
    //printf("location of stack: %p\n", &x);
} 