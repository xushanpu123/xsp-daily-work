use core::usize;
use crate::{console, println};
use super::structs::*;
static mut root:Task = Task{context:Context{content_addr:0},kstack:KernelStack(0)};
extern  "C"{
    pub fn bootstacktop();
    fn t1stacktop();
    fn t2stacktop();
    fn t3stacktop();

}
pub unsafe extern "C" fn test1(){
    println!("I'm task1,I love you!");
    let mut task2 = Task::new(test2 as usize,t2stacktop as usize);
    let mut current_task = Task::new(test1 as usize,t1stacktop as usize);
    current_task.switch_to(&mut root);
}

pub unsafe extern "C" fn test2(){
    println!("I'm task2,It's my turn!");
    let mut task3 = Task::new(test3 as usize,t3stacktop as usize);
    let mut current_task = Task::new(0,t2stacktop as usize);
    current_task.switch_to(&mut root);
}

pub unsafe extern "C" fn test3(){
    println!("I'm task3,hello world!");
    let mut task3 = Task::new(test3 as usize,t3stacktop as usize);
    let mut current_task = Task::new(0,t3stacktop as usize);
    current_task.switch_to(&mut root);
}
pub unsafe  fn task_test(){
    println!("start testing!");
    let mut task1 = Task::new(test1 as usize,t1stacktop as usize);
    root = Task::new(0,bootstacktop as usize);
    root.switch_to(&mut task1);
    println!("I'm coming from task1");
    let mut task2 = Task::new(test2 as usize,t2stacktop as usize);
    root.switch_to(&mut task2);
    println!("I'm coming from task2");
    let mut task3 = Task::new(test3 as usize,t3stacktop as usize);
    root.switch_to(&mut task3);
    println!("I'm coming from task3");
}