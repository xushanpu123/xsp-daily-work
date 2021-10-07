extern crate libc;
extern crate nix;
use std::alloc::{System,Layout,alloc};
use libc::{c_void,pthread_create,pthread_join,pthread_t};
struct myarg_t{
    a:i32,
    b:i32
}

struct myret_t{
    x:i32,
    y:i32
}
pub extern "C" fn mythread(arg:*mut c_void)->*mut c_void{
    let args = arg as *mut myarg_t;
    unsafe{
    println!("args {} {}\n", (*args).a, (*args).b);
    }
    let layout = Layout::new::<myret_t>();
    let rvals = unsafe{alloc(layout) as *mut myret_t};
    unsafe{
    (*rvals).x = 1;
    (*rvals).y = 2;
    }
    return rvals as *mut c_void;
}
fn main(){
    let mut p:pthread_t = 0;
    let mut rvals:*mut myret_t = 1 as *mut myret_t;
    let mut args = myarg_t{
        a:10,
        b:20
    };
    unsafe{
    pthread_create(&mut p,std::ptr::null(),mythread,&mut args as *const myarg_t as *mut c_void);
    pthread_join(p,&mut rvals  as *mut *mut c_void);
    println!("returned {} {}", (*rvals).x, (*rvals).y);
    libc::free(rvals as *const myret_t as *mut c_void);
    }
}


/*int main(int argc, char *argv[]) {
    pthread_t p;
    myret_t *rvals;
    myarg_t args = { 10, 20 };
    Pthread_create(&p, NULL, mythread, &args);
    Pthread_join(p, (void **) &rvals);
    printf("returned %d %d\n", rvals->x, rvals->y);
    free(rvals);
    return 0;
}*/
