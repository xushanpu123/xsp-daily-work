global_asm!(include_str!("/boot/entry.asm"));
const mut allocated_stackNum:usize = 0;
const MAX_allocated:usize = 500;
const KERNEL_STACK_SIZE:usize = 4096 * 4;
const mut space_map:[bool;500] = [false;500];
fn find_first()->usize{    //这里本人希望用树状数组进行重写
    for i in 0..500{
        if space_map[i] == false{
            return i;
        }
        return MAX_allocated;
    }
}


fn alloc_new_stack() ->usize{
    unsafe{
        let first_fit = find_first();
        if first_fit < MAX_allocated{
            space_map[first_fit] = true;
        } 
    }
    bootstacktop as usize + first_fit * KERNEL_STACK_SIZE
}

fn delloc_stack(addr:usize){
    let k = (addr - (bootstacktop as usize) / KERNEL_STACK_SIZE);
    unsafe {
        space_map[k] = false;
    } 
}