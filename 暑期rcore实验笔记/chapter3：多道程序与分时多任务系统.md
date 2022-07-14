#      chapter3：多道程序与分时多任务系统

## 实验目的

1、实现多任务并存和任务切换。

2、实现以一个时钟周期为时间片的时间片轮转调度。



## 实验过程

#### 1、多道程序的编译、打包和载入内存

​       对chapter2中的脚本进行简单的修改，即可让多道程序同时存在于内核程序的代码段中：（见每段代码开头的注释）

```python
#app_id自增，不同app的入口地址不一样。

import os

base_address = 0x80400000
step = 0x20000
linker = "src/linker.ld"

app_id = 0
apps = os.listdir("build/app")
apps.sort()
chapter = os.getenv("CHAPTER")

for app in apps:
    app = app[: app.find(".")]
    os.system(
        "cargo rustc --bin %s --release -- -Clink-args=-Ttext=%x"
        % (app, base_address + step * app_id)
    )
    print(
        "[build.py] application %s start with address %s"
        % (app, hex(base_address + step * app_id))
    )
    if chapter == '3':
        app_id = app_id + 1
```



```rust
// 不同的app也被载入到了代码段中不同的内存位置

fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    // clear i-cache first
    unsafe {
        core::arch::asm!("fence.i");
    }
    // load apps
    for i in 0..num_app {
        let base_i = get_base_i(i);
        // clear region
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });
        // load app from data section to memory
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };
        dst.copy_from_slice(src);
    }
}
```

```rust
//为每个app分配各自的KernelStack和USER_STACK

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

/// user stack instance
static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];
```

```rust
//对各app的TrapContext的初始化也随之变化

pub fn init_app_cx(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        get_base_i(app_id),
        USER_STACK[app_id].get_sp(),
    ))
}
```



#### 2、任务的保存与切换

**任务保存**

​       与前一个chapter的方法类似，内核实例化了数据结构TaskManager，该数据结构包含了一个用来存储所有任务状态信息的全局数组以及其它整体性的描述信息，而描述任务状态信息的数据结构为TaskControlBlock，其中包含了任务的状态TaskStatus和其运行信息TaskContext，由此，当我们只要有任务的app_id，就可以访问到其任意信息。

```rust
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        for (i, t) in tasks.iter_mut().enumerate().take(num_app) {
            t.task_cx = TaskContext::goto_restore(init_app_cx(i));
            t.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}
```

​      这里，若存在app_id号app，则TASK_MANAGER[app_id]的TaskContext完成初始化，状态设为Ready，否则，TaskContext为TaskContext::zero_init()，状态为UnInit。



**任务切换**

​       任务切换的核心函数为

```rust
pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext)
```

其实现为汇编形式：

```
.altmacro
.macro SAVE_SN n
    sd s\n, (\n+2)*8(a0)
.endm
.macro LOAD_SN n
    ld s\n, (\n+2)*8(a1)
.endm
    .section .text
    .globl __switch
__switch:
    # __switch(
    #     current_task_cx_ptr: *mut TaskContext,
    #     next_task_cx_ptr: *const TaskContext
    # )
    # save kernel stack of current task
    sd sp, 8(a0)
    # save ra & s0~s11 of current execution
    sd ra, 0(a0)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr
    # restore ra & s0~s11 of next execution
    ld ra, 0(a1)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    # restore kernel stack of next task
    ld sp, 8(a1)
    ret
```

这里可以看到，__switch其实就做了一个保存和一个恢复，将寄存器的内容保存在自身的TrapContext中，再把对方的TrapContext写入寄存器中，即完成了切换。而任何一个任务的TrapContext均可以通过TASK_MANAGER找到，因此问题就变成了寻找合适的app_id了。剩下的便都是策略和细节问题了。



#### 3、选择合适的任务进行切换

这一部分代码主要在os/src/task/mod.rs中：

```rust
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
```

从字面意义就可以看出几个函数的含义，我们分别看看这几个函数的底层实现，这几个实现都包裹在impl TaskManager 下：

```rust
//选择app_id为0的task来作为第一个执行的task。

fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }
```



```rust
//将当前被调度的task修改为就绪态，等待让出CPU

fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }
```





```rust
//当前进程退出，状态改为退出态

fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

```



```rust
//寻找下一个切换task的app_id，基本方法为循环查找第一个状态为Ready的task

fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }
```





```rust
//切换到下一个task，包含修改该task的状态，修改调度器的current_task,以及做切换操作。

fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }
```



#### 4、增加系统调用yield()和exit()

```rust
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}
```



#### 5、支持时钟中断以及在时钟中断中增加task切换

**时钟中断支持**

```rust
//  os/src/main.rs

pub fn rust_main() -> ! {
    ......
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    ......
}

```



```rust
//   os/src/trap/mod.rs

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}
```



```rust
//   os/src/time.rs

/// read the `mtime` register
pub fn get_time() -> usize {
    time::read()
}

/// get current time in microseconds
pub fn get_time_us() -> usize {
    time::read() / (CLOCK_FREQ / MICRO_PER_SEC)
}

/// set the next timer interrupt
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);  
}
```



```rust
//   os/src/trap/mod.rs

pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        ......
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        ......
    }
    cx
}
```

通过上述代码，时钟中断和切换task任务就完成了。



## 编程练习

#### 题目：获取任务信息

ch3 中，我们的系统已经能够支持多个任务分时轮流运行，我们希望引入一个新的系统调用 `sys_task_info` 以获取当前任务的信息，定义如下：

```
fn sys_task_info(ti: *mut TaskInfo) -> isize
```

- syscall ID: 410
- 查询当前正在执行的任务信息，任务信息包括任务控制块相关信息（任务状态）、任务使用的系统调用及调用次数、任务总运行时长（单位ms）。

```
struct TaskInfo {
    status: TaskStatus,
    syscall_times: [u32; MAX_SYSCALL_NUM],
    time: usize
}
```



#### 完成步骤

**1、在TaskControl中增加相应的元素来记录需要的值：**

```rust
//   os/src/task/task.rs

pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    syscall_times: [u32; MAX_SYSCALL_NUM],   //lab1 adds
    start_time: usize,                             //lab1 adds
    pub task_cx: TaskContext,
    // LAB1: Add whatever you need about the Task.
}
```

其中，start_time用来记录task第一次被调度的时间。

**2、为所有初始化和赋值TaskControl的函数增加对应域的初始化和赋值**

```rust
pub static ref TASK_MANAGER: TaskManager = {
        ......
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            syscall_times: [0; MAX_SYSCALL_NUM],         //lab1 adds
            start_time: 0,								 //lab1 adds
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        ......
```



**3、在task第一次被调度时为其start_time赋值**

​      这里采用一个小trick，用start_time本身来做一个flag，当调度到某task时发现其start_time为0，则说明给task第一次被调度，将start_time设计为当前时间，否则说明该task已经被调度过，不修改其start_time。

```rust
fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            if inner.tasks[next].start_time == 0{
                inner.tasks[next].start_time = get_time_us();
            }
            ......
    }
```



**4、增加pub 方法使得其它模块也能获取到当前task的start_time和taskstatus，从而获取需要的信息**  

```rust
impl TaskManager {
......
fn get_current_TaskControlBlock_start_time(&self)->usize{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].start_time
    }
fn get_current_status(&self)->TaskStatus{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status
    }
......  
}

pub fn get_current_start_time()->usize{
    TASK_MANAGER.get_current_TaskControlBlock_start_time()
}
pub fn get_current_status()->TaskStatus{
    TASK_MANAGER.get_current_status()
}
```



**5、增加syscall_times的记录和处理**

```rust
// os/src/task/mod.rs

impl TaskManager{
    ......
    fn add_syscall_times(&self,syscall_id:usize){
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[syscall_id] += 1;
    }

    fn get_syscall_times(&self)->[u32;500]{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times
    }
    ......
}

......
pub fn add_syscall_times(syscall_id:usize){
    TASK_MANAGER.add_syscall_times(syscall_id);
}

pub fn get_syscall_times()->[u32;500]{
    TASK_MANAGER.get_syscall_times()
}
```



```rust
//    os/src/syscall/mod.rs

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    add_syscall_times(syscall_id);
    ......
}
```



**6、实现系统调用sys_task_info**

```rust
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    unsafe{
    *ti = TaskInfo{
        status:get_current_status(),
        syscall_times:get_syscall_times(),
        time : (get_time_us() - get_current_start_time())/1000

    };
}
    0
}
```



至此，本实验的主体框架和编程练习均已介绍完毕！