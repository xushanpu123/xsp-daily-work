mod handler;
mod trapframe;
mod timer;
pub use trapframe::Trapframe;
pub fn init(){
    handler::init();
    timer::init();
}