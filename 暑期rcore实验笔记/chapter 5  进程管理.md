# chapter 5 : 进程管理

## 实验内容

1、将当前进程与Ready进程分离开，增加管理当前进程的数据结构以增加灵活性；

2、增加进程创建和进程移除机制；

3、增加按name创建对应进程的机制；

4、加入shell，使用户可以参与创建过程。



## 实验过程

#### 1、按name索引elf文件

​        调整build.rs，进而在构建时在link_app.S增加内容：

```
.global _app_names
_app_names:
    .string "ch2b_bad_address"
    .string "ch2b_bad_instructions"
    .string "ch2b_bad_register"
    .string "ch2b_hello_world"
    .string "ch2b_power_3"
    .string "ch2b_power_5"
    .string "ch2b_power_7"
    .string "ch3b_sleep"
    .string "ch3b_sleep1"
    .string "ch3b_yield0"
    .string "ch3b_yield1"
    .string "ch3b_yield2"
    .string "ch5b_exit"
    .string "ch5b_forktest"
    .string "ch5b_forktest2"
    .string "ch5b_forktest_simple"
    .string "ch5b_forktree"
    .string "ch5b_initproc"
    .string "ch5b_user_shell"

 .global _num_app
_num_app:
    .quad 19
    .quad app_0_start
    .quad app_1_start
    .quad app_2_start
    .quad app_3_start
    .quad app_4_start
    .quad app_5_start
    .quad app_6_start
    .quad app_7_start
    .quad app_8_start
    .quad app_9_start
    .quad app_10_start
    .quad app_11_start
    .quad app_12_start
    .quad app_13_start
    .quad app_14_start
    .quad app_15_start
    .quad app_16_start
    .quad app_17_start
    .quad app_18_start
    .quad app_18_end
```

 然后我们通过如下方法获取到name对应的elf数据：

```rust
/// Get elf data by app name
pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    let num_app = get_num_app();
    (0..num_app)
        .find(|&i| APP_NAMES[i] == name)
        .map(get_app_data)
}
```

其中，get_app_data是根据app_id来获取对应的elf数据：

```rust
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

```

#### 2、分离管理就绪进程和运行进程的数据结构

**管理Running进程的结构：Proccessor**

​       我们用processor结构来管理运行中的进程，Processor代表处理器，这样的抽象更接近进程的本质，也可以更好的应用于多核：

```rust
ub struct Processor {
    /// The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,
    /// The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,
}
```

current中存放着当前运行进程的进程控制块，而idle_task_cx则是idle_task的上下文，idle_task实际上是用来作为进程切换的中转的，在进程被调度的时候，会先切换到idle_task，再从idle_task切换到next_task。我们来看一下与Processor有关的方法：

```rust
impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(|task| Arc::clone(task))
    }
}
```

都是一些简单的创建Processor和获取某属性的方法。

与前面类似，我们实例化了Processor作为我们的处理器管理的结构，并且把对其的操作封装成了各种接口：

```rust
lazy_static! {
    /// PROCESSOR instance through lazy_static!
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get token of the address space of current task
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}

/// Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}
```

当当前进程需要被调度的时候，我们需要使用schedule()方法：

```rust
/// Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}

```

这个函数保存了当前进程的上下文，并且跳转到了idle_task，那么idle_task在什么位置呢？我们先来看这么一个函数：

```rust
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);                    //breakpoint
            }
        }
    }
}
```

可以看到，这个函数执行了一个无限循环，循环完毕之后，便会把当前的上下文（尤其是ra）保存到idle_task_cx中，再跳转到next_task，也就是说，一旦run_tasks执行过，idle_task_cx中便会保留着执行完breakpoint这一行代码后的上下文，当再次返回idle_task时，会继续执行一次这个loop，所以idle_task每次被切换时，实际上都是再次执行一次loop中的内容。而我们在载入内核之后，便会执行一次run_tasks:

```rust

#[no_mangle]
/// the rust entry-point of os
pub fn rust_main() -> ! {
    ......
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}
```

因此，这个过程得以被激活。而idle_task中的过喔工作很简单，就是选择next_task并进行进程切换。



**管理Ready进程的结构：TaskManeger**

 在这一部分，TaskManeger进行了一次减负，把当前运行进程的信息全部放入到了Processor结构，其减负后的结构为：

```rust
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}
```

包含的方法有：

```rust
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}
```

用一个简单的队列来维护就绪队列，在fetch()方法中实现了选择目标next进程的方法。显然，调度方法为FIFO。

同样，对其进行了实例化：

```rust
lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
```



**其它相关变化**

在之前的章节中，当进程主动放弃CPU的时候，会去主动执行run_next_task()去切换进程。而在chaper 5中，进程决定放弃CPU则会执行scheduler()，因此，在几个需要进程切换的函数中，均会有一些变化，以suspend_current_and_run_next() 为例：

```rust
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}
```

可以看到，变化主要包括进程被调度时会把自己加入到ready queue中，以及切换进程变成了调用schedule()。



#### 3、为进程分配Pid和内核堆栈

在本章节中，每个进程在创建的时候都会被分配一个Pid，它是一个usize，我们采用了比较熟悉的栈式分配器，这部分代码在os/src/task/pid.rs中，我们直接给出接口函数：

```rust
pub fn pid_alloc() -> PidHandle {                                //分配一个pid
    PID_ALLOCATOR.exclusive_access().alloc()
}

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {     //根据pid获取内核堆栈，返回堆栈区域的区间
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}
```

**定义内核堆栈**

```rust

pub struct KernelStack {
    pid: usize,
}
```

内核堆栈只包含一个pid，这是因为前面定义了函数kernel_stack_position()，可以由pid获取到对应的堆栈的位置，所以实际上，内核堆栈的位置是和pid一一对应的，对堆栈我们也定义了具体的方法：

```rust
pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        KernelStack { pid: pid_handle.0 }
    }
    #[allow(unused)]
    /// Push a variable of type T into the top of the KernelStack and return its raw pointer
    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }
    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }
}
```

#### 4、实现系统调用sys_fork()

