pub struct ProcessorInner {
    // 线程池
    pool: Box<ThreadPool>,
    // idle 线程
    idle: Box<Thread>,
    // 当前正在运行的线程
    current: Option<(Tid, Box<Thread>)>,
}
pub struct Processor {
    inner: UnsafeCell<Option<ProcessorInner>>,
}
unsafe impl Sync for Processor {}

// src/process/mod.rs
use processor::Processor;
static CPU: Processor = Processor::new();
impl Processor {
    // 新建一个空的 Processor
    pub const fn new() -> Processor {
        Processor {  inner: UnsafeCell::new(None),  }
    }
    // 传入 idle 线程，以及线程池进行初始化
    pub fn init(&self, idle: Box<Thread>, pool: Box<ThreadPool>) {
        unsafe {
            *self.inner.get() = Some(
                ProcessorInner {
                    pool,
                    idle,
                    current: None,
                }
            );
        }
    }
    // 内部可变性：获取包裹的值的可变引用
    fn inner(&self) -> &mut ProcessorInner {
        unsafe { &mut *self.inner.get() }
            .as_mut()
            .expect("Processor is not initialized!")
    }
    // 通过线程池新增线程
    pub fn add_thread(&self, thread: Box<Thread>) {
        self.inner().pool.add(thread);
    }
}
impl Processor {
    pub fn idle_main(&self) -> ! {
        let inner = self.inner();
        // 在 idle 线程刚进来时禁用异步中断
        disable_and_store();

        loop {
            // 如果从线程池中获取到一个可运行线程
            if let Some(thread) = inner.pool.acquire() {
                // 将自身的正在运行线程设置为刚刚获取到的线程
                inner.current = Some(thread);
                // 从正在运行的线程 idle 切换到刚刚获取到的线程
                println!("\n>>>> will switch_to thread {} in idle_main!", inner.current.as_mut().unwrap().0);
                inner.idle.switch_to(
                    &mut *inner.current.as_mut().unwrap().1
                );

                // 上个线程时间耗尽，切换回调度线程 idle
                println!("<<<< switch_back to idle in idle_main!");
                // 此时 current 还保存着上个线程
                let (tid, thread) = inner.current.take().unwrap();
                // 通知线程池这个线程需要将资源交还出去
                inner.pool.retrieve(tid, thread);
            }
            // 如果现在并无任何可运行线程
            else {
                // 打开异步中断，并等待异步中断的到来
                enable_and_wfi();
                // 异步中断处理返回后，关闭异步中断
                disable_and_store();
            }
        }
    }
}
