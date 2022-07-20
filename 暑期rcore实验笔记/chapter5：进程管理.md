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

**sys_fork()的语义**

```rust
pub fn sys_fork() -> isize
```

建立一个子进程，子进程复制了父进程的地址空间和上下文，在父进程中，该函数返回分配的pid，在子进程中，该函数返回0，若分配发生错误，则返回-1。

我们先考虑如何实现地址空间的完全复制，实际上地址空间的信息都保存在TaskControlBlock中，而产生一个新的进程也需要产生一个新的TaskControlBlock中，因此，我们可以为TaskControlBlock增加fork()方法，来产生新进程所需要的TaskControlBlock：

```rust
pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        // ---- access parent PCB exclusively
        let mut parent_inner = self.inner_exclusive_access();
        // copy user space(include trap context)
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set); //这里复制了一份地址空间，具体作法后面分析
        let trap_cx_ppn = memory_set                                             //新分配了trap_cx_ppn的物理页
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        });
        // add child
        parent_inner.children.push(task_control_block.clone());
        // modify kernel_sp in trap_cx
        // **** access children PCB exclusively
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;
        // return
        task_control_block
        // ---- release parent PCB automatically
        // **** release children PCB automatically
    }
```

下面看from_existed_user()的具体实现：

```rust
impl MapArea{
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.get_start(), another.vpn_range.get_end()),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
        }
    }
}


impl MemorySet{
 pub fn from_existed_user(user_space: &MemorySet) -> MemorySet {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // copy data sections/trap_context/user_stack
        for area in user_space.areas.iter() {
            let new_area = MapArea::from_another(area);
            memory_set.push(new_area, None);
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().ppn();
                let dst_ppn = memory_set.translate(vpn).unwrap().ppn();
                dst_ppn
                    .get_bytes_array()
                    .copy_from_slice(src_ppn.get_bytes_array());
            }
        }
        memory_set
    }
}
```

具体来说，就是建立一个与原地址空间有相同的虚拟地址与虚拟页的地址空间，且所有虚拟页中的内容都一样，但是它们的物理页是重新分配的，不同于原地址空间的物理页帧。



由此，我们可以实现sys_fork()了：

```rust
/// Syscall Fork which returns 0 for child process and child_pid for parent process
pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();   //获取新进程的TaskContext
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}
```

这里需要注意，新进程完全复制了父进程的地址空间和上下文（包括sepc），而子进程继承的sepc是+4了的，因此并不会去执行系统调用，而是直接把x[10]中的值作为函数的返回地址，因此返回值为0。



#### 5、实现系统调用sys_exec()

**sys_exec()语义**

```rust
  pub fn sys_exec(path: *const u8) -> isize
```

用path所对应的程序来覆盖当前进程的程序。



**sys_exec()实现**

​     同理，需要在TaskControlBlock中加入对应方法：

```rust
pub fn exec(&self, elf_data: &[u8]) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // **** access inner exclusively
        let mut inner = self.inner_exclusive_access();
        // substitute memory_set
        inner.memory_set = memory_set;
        // update trap_cx ppn
        inner.trap_cx_ppn = trap_cx_ppn;
        // initialize trap_cx
        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        // **** release inner automatically
    }
```

这里利用 elf_data构建新的地址空间，然后将inner值来进行赋值。这里有一个细节，当inner.memory_set换绑定到memory_set时，原先绑定的地址空间便被释放了，此时会自动调用drop函数来释放掉之前占用的物理页帧，所以不用担心内存泄漏的问题。

```rust
pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}
```

这里需要注意，传入字符串的方式是传入用户态下的字符串首地址，在内核态下当然还需要进行对应的转化才可以，转化函数为：

```rust
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .get_mut());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}

```

基本逻辑为借助转化后的字符串起始地址和字符串末尾的"\0"来获取整个字符串。

进行一系列处理后，便构建好了新的TaskControlBlock和TaskContext，等到中断恢复后便会执行新加载的程序了。



#### 6、实现sys_read()系统调用

```rust
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}
```

实现把标准输入的字符串（长度为len）读取到buf[0..len]中。



## 编程练习

### 进程创建

大家一定好奇过为啥进程创建要用 fork + exec 这么一个奇怪的系统调用，就不能直接搞一个新进程吗？ 思而不学则殆，我们就来试一试！这章的编程练习请大家实现一个完全 DIY 的系统调用 spawn，用以创建一个新进程。

spawn 系统调用定义( [标准spawn看这里](https://man7.org/linux/man-pages/man3/posix_spawn.3.html) )：

```
fn sys_spawn(path: *const u8) -> isize
```

- syscall ID: 400

- 功能：新建子进程，使其执行目标程序。

- 说明：成功返回子进程id，否则返回 -1。

- - 可能的错误：

    无效的文件名。 进程池满/内存不足等资源错误。

TIPS：虽然测例很简单，但提醒读者 spawn **不必** 像 fork 一样复制父进程的地址空间。

#### 实验过程

借鉴fork()和exec()的过程，我们可以看到实现spawn()只需要进行如下几步即可：

1、根据exec()中的参数创建新的地址空间memory_set。

2、创建新的TaskControlBlock，模仿fork()进行初始化赋值工作，除了把复制地址空间的操作修改为把地址空间赋值为memory_set。

```rust
pub fn sys_spawn(_path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let new_task: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(data));
        let mut new_inner = new_task.inner_exclusive_access();
        let parent = current_task().unwrap();
        let mut parent_inner = parent.inner_exclusive_access();
        new_inner.parent = Some(Arc::downgrade(&parent));
        parent_inner.children.push(new_task.clone());
        drop(new_inner);
        drop(parent_inner);
        let new_pid = new_task.pid.0;
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}
```



### stride 调度算法

ch3 中我们实现的调度算法十分简单。现在我们要为我们的 os 实现一种带优先级的调度算法：stride 调度算法。

算法描述如下:

(1) 为每个进程设置一个当前 pass，表示该进程当前已经运行的“长度”。另外设置其对应的 stride 值（只与进程的优先权有关系），表示对应进程在调度后，pass 需要进行的累加值。

1. 每次需要调度时，从当前 runnable 态的进程中选择 pass 最小的进程调度。对于获得调度的进程 P，将对应的 pass 加上其对应的步长 stride。
2. 一个时间片后，回到上一步骤，重新调度当前 pass 最小的进程。

可以证明，如果令 P.stride = BigStride / P.priority 其中 P.priority 表示进程的优先权（大于 1），而 BigStride 表示一个预先定义的大常数，则该调度方案为每个进程分配的时间将与其优先级成正比。证明过程我们在这里略去，有兴趣的同学可以在网上查找相关资料。

其他实验细节：

- stride 调度要求进程优先级 

，所以设定进程优先级 

-  会导致错误。
- 进程初始 pass 设置为 0 即可。
- 进程初始优先级设置为 16。

为了实现该调度算法，内核还要增加 set_prio 系统调用

```
// syscall ID：140
// 设置当前进程优先级为 prio
// 参数：prio 进程优先级，要求 prio >= 2
// 返回值：如果输入合法则返回 prio，否则返回 -1
fn sys_set_priority(prio: isize) -> isize;
```

实现 tips:

- 你可以在TCB加入新的字段来支持优先级等。
- 为了减少整数除的误差，BIG_STRIDE 一般需要很大，但为了不至于发生反转现象（详见问答作业），或许选择一个适中的数即可，当然能进行溢出处理就更好了。
- stride 算法要找到 pass 最小的进程，使用优先级队列是效率不错的办法，但是我们的实验测例很简单，所以效率完全不是问题。事实上，很推荐使用暴力扫一遍的办法找最小值。
- 注意设置进程的初始优先级。



#### 实验过程

**1、在TCB中增加新字段**

```rust
pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
    pub task_cx: TaskContext,
    pub task_status: TaskStatus,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: i32,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub start_time: usize,
    pub task_priority: usize,                          //added
    pub task_stride: usize,                            //added    
}
```



**2、修改所有与TCB赋值和初始化有关的操作**

```rust
pub fn new(elf_data: &[u8]) -> Self {
        ......
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    syscall_times: [0; MAX_SYSCALL_NUM],
                    start_time: get_time_us() / 1000,
                    task_priority: 16,
                    task_stride: 0,
                })
            },
        };
        .......
    }
    pub fn exec(&self, elf_data: &[u8]) {
        ......
        inner.task_priority = 16;
        ......
    }
    pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        ......
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    syscall_times: parent_inner.syscall_times,
                    start_time: parent_inner.start_time,
                    task_priority: parent_inner.task_priority,
                    task_stride: parent_inner.task_stride,
                })
            },
        });
       .......
    }
  
```



**3、由于每次需要从就绪队列中选择stride最小的进程来运行，所以我们只需要维护一个单调的队列就可以满足这个要求了**

```rust
pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        let task_inner = task.inner_exclusive_access();
        let stride = task_inner.task_stride;
        drop(task_inner);
        let len = self.ready_queue.len();
        for queue in 0..len{
            let task1 = self.ready_queue.get_mut(queue).unwrap();
            let stride1 = task1.inner_exclusive_access().task_stride;
            if stride < stride1 {
                self.ready_queue.insert(queue, task);
                return
            }
        }
        self.ready_queue.push_back(task)
    }
```



**4、进程暂停且切换时，需要更新其stride值**

```rust
pub fn suspend_current_and_run_next() {
    ......
    task_inner.task_status = TaskStatus::Ready;
    task_inner.task_stride = task_inner.task_stride + BIG_STRIDE / task_inner.task_priority;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}
```

**5、实现系统调用sys_set_priority()**

```rust
pub fn sys_set_priority(pri: isize) -> isize {
    if pri < 2{
        return -1;
    }
    set_task_priority(pri as usize);
    pri as isize
}
```

至此，chapter 5 的编程练习任务完成。