# chapter 8：并发与互斥访问

## 实验目的

1、对进程的处理器分配进行再划分，实现线程机制；

2、实现锁机制；

3、实现信号量机制；

4、实现管程机制。



## 实验过程

### 线程机制的实现

#### 1、线程机制下提供的系统调用

```rust
pub fn sys_thread_create(entry: usize, arg: usize) -> isize
```

语义：在当前进程中创建一个新的线程，线程的入口点为entry，arg作为传给线程的一个参数。



```rust
pub fn sys_gettid() -> isize
```

语义：获取当前线程的tid。



```rust
pub fn sys_waittid(tid: usize) -> i32
```

语义：当一个线程执行完代表它的功能后，会通过 `exit` 系统调用退出。内核在收到线程发出的 `exit` 系统调用后， 会回收线程占用的部分资源，即用户态用到的资源，比如用户态的栈，用于系统调用和异常处理的跳板页等。 而该线程的内核态用到的资源，比如内核栈等，需要通过进程/主线程调用 `waittid` 来回收了， 这样整个线程才能被彻底销毁。



#### 2、构建与线程相关的数据结构及其方法

**线程与进程的关系**

一个进程拥有一个主线程和多个创建出来的线程，共同占有进程的地址空间、打开文件等资源，但有各自的TaskTrapContext和堆栈等。当线程结束时，需要由进程（主线程）来回收它们的分配资源和退出值。因此，我们先看看进程与线程对应的数据结构：

```rust
pub struct ProcessControlBlock {
    // immutable
    pub pid: PidHandle,
    // mutable
    inner: UPSafeCell<ProcessControlBlockInner>,
}

pub struct ProcessControlBlockInner {
    ......
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,				//自己所拥有的线程的tcb
    pub task_res_allocator: RecycleAllocator,                   //用来分配线程的内核堆栈
    ......
}
```

```rust
/// Task control block structure
///
/// Directly save the contents that will not change during running
pub struct TaskControlBlock {
    // immutable
    pub process: Weak<ProcessControlBlock>,
    /// Kernel stack corresponding to TID
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

/// Structure containing more process content
///
/// Store the contents that will change during operation
/// and are wrapped by UPSafeCell to provide mutual exclusion
pub struct TaskControlBlockInner {
    /// The physical page number of the frame where the trap context is placed
    pub trap_cx_ppn: PhysPageNum,
    /// Save task context
    pub task_cx: TaskContext,
    /// Maintain the execution status of the current process
    pub task_status: TaskStatus,
    /// It is set when active exit or execution error occurs
    pub exit_code: Option<i32>,
    /// Tid and ustack will be deallocated when this goes None
    pub res: Option<TaskUserRes>,
}
```

可以看到，每个线程拥有自己的TaskContext，TaskStatus，KernelStack以及跳板trap_cx_ppn。



**创建线程的实现**

```rust
pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    let task = current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    // create a new thread
    let new_task = Arc::new(TaskControlBlock::new(
        Arc::clone(&process),
        task.inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .ustack_base,
        true,
    ));
    let new_task_inner = new_task.inner_exclusive_access();
    let new_task_res = new_task_inner.res.as_ref().unwrap();
    let new_task_tid = new_task_res.tid;
    let new_task_trap_cx = new_task_inner.get_trap_cx();
    *new_task_trap_cx = TrapContext::app_init_context(
        entry,
        new_task_res.ustack_top(),
        kernel_token(),
        new_task.kernel_stack.get_top(),
        trap_handler as usize,
    );
    (*new_task_trap_cx).x[10] = arg;

    let mut process_inner = process.inner_exclusive_access();
    // add new thread to current process
    let tasks = &mut process_inner.tasks;
    while tasks.len() < new_task_tid + 1 {
        tasks.push(None);
    }
    tasks[new_task_tid] = Some(Arc::clone(&new_task));
    // add new task to scheduler
    add_task(Arc::clone(&new_task));
    new_task_tid as isize
}
```

其基本过程与前面进程相关章节类似，根据需要对new_task进行赋值并装入其内核堆栈中，再将其加入到所属进程的tasks队列中。



**fork()方法的修改**

```rust
    /// Fork from parent to child
    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        ......
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kernel_stack here
            false,
        ));
        ......
        add_task(task);
        child
    }
```

可以看到，在创建进程的时候，会同步创建其主线程，而且我们看到同样会有add_task()方法，说明引入线程机制后，操作系统调度的基本单位是线程。



**sys_exit()方法的修改**

```rust
pub fn sys_exit(exit_code: i32) -> ! {
    // debug!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    // **** access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let process = task.process.upgrade().unwrap();
    let tid = task_inner.res.as_ref().unwrap().tid;
    // Record exit code
    task_inner.exit_code = Some(exit_code);
    task_inner.res = None;

    // here we do not remove the thread since we are still using the kstack
    // it will be deallocated when sys_waittid is called
    drop(task_inner);
    drop(task);
    // debug!("task {} dropped", tid);

    if tid == 0 {
        ......
    }
    // debug!("pcb dropped");

    // ++++++ release parent PCB
    drop(process);

    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}
```

前半部分为回收线程所占的部分资源，如果线程为主线程，则执行前面章节提到的退出进程的步骤。



**sys_waittid()的实现**

```rust
/// thread does not exist, return -1
/// thread has not exited yet, return -2
/// otherwise, return thread's exit code
pub fn sys_waittid(tid: usize) -> i32 {
    let task = current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    let task_inner = task.inner_exclusive_access();
    let mut process_inner = process.inner_exclusive_access();
    // a thread cannot wait for itself
    if task_inner.res.as_ref().unwrap().tid == tid {
        return -1;
    }
    let mut exit_code: Option<i32> = None;
    let waited_task = process_inner.tasks[tid].as_ref();
    if let Some(waited_task) = waited_task {
        if let Some(waited_exit_code) = waited_task.inner_exclusive_access().exit_code { //退出码存在，则说明线程退出，记录退出码
            exit_code = Some(waited_exit_code);    
        }
    } else {
        // waited thread does not exist
        return -1;
    }
    if let Some(exit_code) = exit_code {   
        // dealloc the exited thread
        process_inner.tasks[tid] = None;
        exit_code
    } else {
        // waited thread has not exited
        -2
    }
}
```



### 实现锁机制

在本实验中，同一个进程的不同线程共用一把锁，所以锁结构被存放在了进程控制块中:

```rust
pub struct ProcessControlBlockInner {
    ......
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    ......
}
```

其中，dyn Mutex表示mutex_list中的元素是要求互斥访问的，这就保证了对锁结构本身访问的原子性。由此，我们编写了与之相关的三个系统调用：

```rust
pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}

```

我们用锁在向量中的索引来唯一标识锁。



```rust
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
```

这里是申请锁和解锁的操作，重点在于对锁本身的操作。



**锁的结构与方法**

```rust
pub trait Mutex: Sync + Send {
    fn lock(&self);
    fn unlock(&self);
}

pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    fn lock(&self) {
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Mutex for MutexBlocking {
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
```

这里提供了两种结构自旋锁MutexSpin与阻塞锁MutexBlocking，我们重点看MutexBlocking，它包含一个bool值和一个等待队列，可以看到，在lock()时，如果锁被占用，则当前线程被阻塞，否则占有锁。在unlock()中，会从锁的等待队列中取出一个线程唤醒。接下来我们来看block_current_and_run_next()：

```rust
pub fn block_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    task_inner.task_status = TaskStatus::Blocking;
    drop(task_inner);
    schedule(task_cx_ptr);
}
```

这里，我们把当前线程的状态设为Blocking，且使其回到就绪态的方法只存在于unlock()中。就此，便实现了与锁相关联的一对阻塞 与唤醒。至此，锁机制便实现了。



### 信号量机制

```rust
pub struct Semaphore {
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn up(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                add_task(task);
            }
        }
    }

    pub fn down(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
    }
}
```



### 条件变量机制

```rust
pub struct Condvar {
    pub inner: UPSafeCell<CondvarInner>,
}

pub struct CondvarInner {
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(CondvarInner {
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn signal(&self) {
        let mut inner = self.inner.exclusive_access();
        if let Some(task) = inner.wait_queue.pop_front() {
            add_task(task);
        }
    }

    pub fn wait(&self, mutex: Arc<dyn Mutex>) {
        mutex.unlock();
        let mut inner = self.inner.exclusive_access();
        inner.wait_queue.push_back(current_task().unwrap());
        drop(inner);
        block_current_and_run_next();
        mutex.lock();
    }
}
```



## 编程练习

### 死锁检测

目前的 mutex 和 semaphore 相关的系统调用不会分析资源的依赖情况，用户程序可能出现死锁。 我们希望在系统中加入死锁检测机制，当发现可能发生死锁时拒绝对应的资源获取请求。 一种检测死锁的算法如下：

定义如下三个数据结构：

- 可利用资源向量 Available ：含有 m 个元素的一维数组，每个元素代表可利用的某一类资源的数目， 其初值是该类资源的全部可用数目，其值随该类资源的分配和回收而动态地改变。 Available[j] = k，表示第 j 类资源的可用数量为 k。
- 分配矩阵 Allocation：n * m 矩阵，表示每类资源已分配给每个线程的资源数。 Allocation[i,j] = g，则表示线程 i 当前己分得第 j 类资源的数量为 g。
- 需求矩阵 Need：n * m 的矩阵，表示每个线程还需要的各类资源数量。 Need[i,j] = d，则表示线程 i 还需要第 j 类资源的数量为 d 。

算法运行过程如下：

1. 设置两个向量: 工作向量 Work，表示操作系统可提供给线程继续运行所需的各类资源数目，它含有 m 个元素。初始时，Work = Available ；结束向量 Finish，表示系统是否有足够的资源分配给线程， 使之运行完成。初始时 Finish[0..n-1] = false，表示所有线程都没结束；当有足够资源分配给线程时， 设置 Finish[i] = true。
2. 从线程集合中找到一个能满足下述条件的线程

```
1Finish[i] == false;
2Need[i,j] ≤ Work[j];
```

若找到，执行步骤 3，否则执行步骤 4。

1. 当线程 thr[i] 获得资源后，可顺利执行，直至完成，并释放出分配给它的资源，故应执行:

```
1Work[j] = Work[j] + Allocation[i, j];
2Finish[i] = true;
```

跳转回步骤2

1. 如果 Finish[0..n-1] 都为 true，则表示系统处于安全状态；否则表示系统处于不安全状态，即出现死锁。

出于兼容性和灵活性考虑，我们允许进程按需开启或关闭死锁检测功能。为此我们将实现一个新的系统调用： `sys_enable_deadlock_detect` 。

**enable_deadlock_detect**：

- syscall ID:  469

- 功能：为当前进程启用或禁用死锁检测功能。

- C 接口： `int enable_deadlock_detect(int is_enable)`

- Rust 接口： `fn enable_deadlock_detect(is_enable: i32) -> i32`

- - 参数：

    is_enable: 为 1 表示启用死锁检测， 0 表示禁用死锁检测。

- - 说明：

    开启死锁检测功能后， `mutex_lock` 和 `semaphore_down` 如果检测到死锁， 应拒绝相应操作并返回 -0xDEAD (十六进制值)。 简便起见可对 mutex 和 semaphore 分别进行检测，无需考虑二者 (以及 `waittid` 等) 混合使用导致的死锁。

- 返回值：如果出现了错误则返回 -1，否则返回 0。

- - 可能的错误

    参数不合法 死锁检测开启失败



#### 实验过程

**1、在process中增加相应的域，来检测当前的死锁检测机制是否已经开启**

```rust
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    match _enabled {
        0 => {
            process_inner.deadlock_det_enabled = false;
            0
        }
        1 => {
            process_inner.deadlock_det_enabled = true;
            0
        }
        _ => -1,
    }
}
```

```rust
pub struct ProcessControlBlockInner {
    ......
    pub deadlock_det_enabled: bool,
}

```



**2、在进程控制块中加入死锁检测需要的结构**

```rust
pub struct ProcessControlBlockInner {
    ......
    pub mutex_alloc: Vec<Option<usize>>,   // [mutex_id] -> tid，用来表示各个锁的分配情况
    pub mutex_request: Vec<Option<usize>>, // [tid] -> mutex_id，用来表示各个线程对锁的请求情况
    pub sem_avail: Vec<usize>,           // [mid] -> num，表示各个信号量的可用数量
    pub sem_alloc: Vec<Vec<usize>>,      // [tid] -> {sid, num}，用来表示各个信号量给每个线程的分配情况
    pub sem_request: Vec<Option<usize>>, // [tid] -> sid，表示各个线程对信号量的请求情况
    pub deadlock_det_enabled: bool,
}
```

**3、在sys_mutex_lock()和sys_semaphore_down()中进行银行家算法的检测**

```rust
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let det = process_inner.deadlock_det_enabled;
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    process_inner.mutex_request[tid] = Some(mutex_id);
    if det {
        let mut visited = BTreeSet::<usize>::new();
        visited.insert(tid);
        let mut mid = mutex_id;
        while let Some(tid2) = process_inner.mutex_alloc[mid] {
            if visited.contains(&tid2) {
                println!(
                    " ----- deadlock! pid: {}, tid: {}, mutex_id: {} ------",
                    process.pid.0, tid, mutex_id
                );
                return -0xdead;
            } else {
                visited.insert(tid2);
                if let Some(mid2) = process_inner.mutex_request[tid2] {
                    mid = mid2;
                } else {
                    break;
                }
            }
        }
    }
    drop(process_inner);
    drop(process);
    mutex.lock();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.mutex_alloc[mutex_id] = Some(tid);
    process_inner.mutex_request[tid] = None;
    0
}

pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let tid = sys_gettid() as usize;
    process_inner.sem_request[tid] = Some(sem_id);
    let det = process_inner.deadlock_det_enabled;
    if det {
        // deadlock detection
        // init
        let mut work = process_inner.sem_avail.clone();
        let mut not_finished = BTreeSet::<usize>::new();
        for (tid2, t_alloc) in process_inner.sem_alloc.iter().enumerate() {
            if !t_alloc.is_empty() {
                not_finished.insert(tid2);
            }
        }

        let mut all_released = false;
        let mut all_finished = not_finished.is_empty();
        while !all_finished && !all_released {
            all_released = true;
            let mut finished = Vec::<usize>::new();
            for tid2 in not_finished.iter() {
                // step 2
                if let Some(sid) = process_inner.sem_request[*tid2] {
                    if work[sid] == 0 {
                        continue;
                    }
                }
                all_released = false;
                // step 3
                finished.push(*tid2);
                for (sid, num) in process_inner.sem_alloc[*tid2].iter().enumerate() {
                    work[sid] += num;
                }
            }
            for tid2 in finished.iter() {
                not_finished.remove(tid2);
            }
            // not_finished = not_finished.difference(&finished).collect();
            all_finished = not_finished.is_empty();
        }

        if !not_finished.is_empty() {
            println!(
                "--- deadlock! pid: {}, tid: {}, sem_id: {}",
                process.pid.0, tid, sem_id
            );
            return -0xdead;
        }
    }
    drop(process_inner);
    sem.down();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.sem_request[tid] = None;
    process_inner.sem_avail[sem_id] -= 1;
    process_inner.sem_alloc[tid][sem_id] += 1;
    0
}
```

至此，chapter 8的编程实验任务便完成了。