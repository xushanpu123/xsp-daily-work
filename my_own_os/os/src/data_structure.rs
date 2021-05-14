#[derive(Copy, Clone)]
#[repr(C)]
pub struct queue{
    x:[usize;200],
    low:usize,
    hi:usize
}
impl queue{
    pub fn new()->Self{
        Self{
            x:[0;200],
          low:0,
           hi:0
        }
    }
    pub fn size(self)->usize{
        self.hi - self.low
    }
    pub fn dequeue(&mut self)->usize{
        if self.low < self.hi{
            self.low+=1;
            self.x[self.low-1]
        }
        else{
            panic!("The queue is empty!");
        }
    }
  
    pub fn enqueue(&mut self,added:usize){
        if self.hi<200{
            self.x[self.hi] = added;
            self.hi+=1;
        }
        else if self.size()>=200{
            panic!("The queue is full!");
        }
        else{
            for i in self.low..200{
                self.x[i - self.low] = self.x[i];
            }
            self.low = 0;
            self.hi = self.size();
        }
    }
}