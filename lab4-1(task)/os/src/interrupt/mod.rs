mod handler;
mod context;
mod timer;
pub fn init(){
    handler::init();
    timer::init();
}