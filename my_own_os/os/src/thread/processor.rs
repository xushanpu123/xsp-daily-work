use super::structs::*;
use crate::data_structure::*;
pub type TID = usize;
const thread_cnt:usize = 20;
#[derive(Copy, Clone)]
pub struct Processor{
   current_thread:TID,
   runnable_threads:queue,
   round:usize
}
impl Processor{
    pub unsafe fn init()->Self{
        let mut runnable_threads = queue::new();
        for i in 0..thread_cnt{
           if threadspool[i].tid>0{
               runnable_threads.enqueue(threadspool[i].tid);
           } 
       
        }
        let current_thread = runnable_threads.dequeue();
        Self{
            current_thread,
            runnable_threads,
            round:10
        }
    }
    pub fn get_current(self)->TID{
        self.current_thread
    }
    pub fn push(&mut self,added:TID){
       self.runnable_threads.enqueue(added);
    }
    pub fn pop(&mut self)->TID{
        self.runnable_threads.dequeue()
    }
    pub fn set_current(&mut self,setted:TID){
        self.current_thread = setted;
    }
    pub unsafe fn tick(&mut self){
        if self.round > 0 {
            self.round-=1;
        }
        else{
            let pre_current = self.get_current();
            self.push(pre_current);
            let next_current = self.pop();
            self.set_current(next_current);
            self.round = 10;
            threadspool[pre_current].switch_to(&mut threadspool[next_current]);
        }
    }
}