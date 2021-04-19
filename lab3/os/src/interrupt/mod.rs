mod handler;
mod trapframe;
mod timer;
pub fn init(){
    handler::init();
    timer::init();
}