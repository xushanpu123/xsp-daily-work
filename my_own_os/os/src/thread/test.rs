use lazy_static::lazy_static;
use super::structs::Thread;
use super::context::*;
use super::processor::Processor;
use crate::println;
static mut root:Thread = Thread{tid:0,context:Context{addr:0},Kernalstack:0};
static mut th1:Thread = Thread{tid:0,context:Context{addr:0},Kernalstack:0};
static mut th2:Thread = Thread{tid:0,context:Context{addr:0},Kernalstack:0};
static mut th3:Thread = Thread{tid:0,context:Context{addr:0},Kernalstack:0};
  extern "C" {
      /// `interrupt.asm` 中的中断入口
      fn bootstacktop();
      fn bootstacktop1();
      fn bootstacktop2();
      fn bootstacktop3();
  }
pub unsafe extern "C" fn thread1(){
   println!("It's thread1!");
   th1.switch_to(&mut root);
   println!("return from root!");
   th1.switch_to(&mut root);
}
pub unsafe extern "C" fn thread2(){
  println!("It's thread2!");
  th2.switch_to(&mut root);
}
pub unsafe extern "C" fn thread3(){
  println!("It's thread3!");
  th3.switch_to(&mut root);
}
pub unsafe fn Test(){
   root = Thread::new([0,0,0,0,0,0,0,0,0,0,0,0],0,bootstacktop as usize);
   println!("It's root thread!");
   th1 =  Thread::new([0,0,0,0,0,0,0,0,0,0,0,0],thread1 as usize,bootstacktop1 as usize);
  // println!("{}",(*(th1.context.addr as (*mut Contextcontent))).ra);
   th2 =  Thread::new([0,0,0,0,0,0,0,0,0,0,0,0],thread2 as usize,bootstacktop2 as usize);
   th3 =  Thread::new([0,0,0,0,0,0,0,0,0,0,0,0],thread3 as usize,bootstacktop3 as usize);
   //println!("{}",(*(th1.context.addr as (*mut Contextcontent))).ra);
   /*if (*(th1.context.addr as *mut Contextcontent)).ra == thread1 as usize{
        println!("pass test!");
   }*/
   let processor = Processor::init();
   
   root.switch_to(&mut th1);
   println!("return from thread1!");
   root.switch_to(&mut th2);
   println!("return from thread2!");
   root.switch_to(&mut th3);
   println!("return from thread3!");

}