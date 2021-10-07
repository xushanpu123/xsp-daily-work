extern crate libc;
extern crate nix;
use libc::{c_void,pthread_create,pthread_join,pthread_t};
pub extern "C" fn mythread(arg:*mut c_void)->*mut c_void{
    let value:i128 = arg as i128;
    println!("{}",value);
    (value+1) as *mut c_void
}

fn main(){
   let  mut p = 0;
   let  mut rvalue : i128 = 0;
   unsafe{
   pthread_create(&mut p,std::ptr::null(),mythread,100 as *mut c_void);
   pthread_join(p,&rvalue as *const i128 as *mut *mut c_void);
   }
   println!("returned {}",rvalue);
}
/*void *mythread(void *arg) {
    long long int value = (long long int) arg;
    printf("%lld\n", value);
    return (void *) (value + 1);
}

int main(int argc, char *argv[]) {
    pthread_t p;
    long long int rvalue;
    Pthread_create(&p, NULL, mythread, (void *) 100);
    Pthread_join(p, (void **) &rvalue);
    printf("returned %lld\n", rvalue);
    return 0;
}*/