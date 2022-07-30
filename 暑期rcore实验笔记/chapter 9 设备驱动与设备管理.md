# 					chapter 9: 设备驱动与设备管理

## 实验目的

在内核中增加功能 ，使之能够支持一定的图形界面，并且能够获取鼠标、键盘等外设的输入并作出反应。



## 实验分析

### Virtio-drivers部分

#### 初始化

```rust
#[no_mangle]
extern "C" fn main(_hartid: usize, device_tree_paddr: usize) {
    log::set_max_level(LevelFilter::Info);
    init_dt(device_tree_paddr);
    info!("test end");
}
```

在进入main()函数之前，opensbi把_hartid和device_tree_paddr两个参数放入了a0和 a1两个参数寄存器中，由此，os执行init_dt(device_tree_paddr)。



#### 初始化函数init_dt()

```rust
fn init_dt(dtb: usize) {
    info!("device tree @ {:#x}", dtb);
    #[repr(C)]
    struct DtbHeader {
        be_magic: u32,
        be_size: u32,
    }
    let header = unsafe { &*(dtb as *const DtbHeader) };
    let magic = u32::from_be(header.be_magic);
    const DEVICE_TREE_MAGIC: u32 = 0xd00dfeed;
    assert_eq!(magic, DEVICE_TREE_MAGIC);
    let size = u32::from_be(header.be_size);
    let dtb_data = unsafe { core::slice::from_raw_parts(dtb as *const u8, size as usize) };
    let dt = DeviceTree::load(dtb_data).expect("failed to parse device tree");
    walk_dt_node(&dt.root);
}

fn walk_dt_node(dt: &Node) {
    if let Ok(compatible) = dt.prop_str("compatible") {
        if compatible == "virtio,mmio" {
            virtio_probe(dt);
        }
    }
    for child in dt.children.iter() {
        walk_dt_node(child);
    }
}
```

初始化除了进行一些简单的检测以外，一个重要的任务是对设备树进行探测。可以看到walk_dt_node()函数，它是一个递归函数，可以探测整个树。由此，我们也可以看出，opensbi实际上是把整个设备树写入了操作系统的内存中，并且把根元素的地址给到了寄存器a1中，因此，通过这个操作便可以对设备树中的每个满足条件的节点，即每个满足条件的设备执行virtio_probe()。



#### 探测函数virtio_probe()

```rust
fn virtio_probe(node: &Node) {
    if let Some(reg) = node.prop_raw("reg") {
        let paddr = reg.as_slice().read_be_u64(0).unwrap();
        let size = reg.as_slice().read_be_u64(8).unwrap();
        let vaddr = paddr;
        info!("walk dt addr={:#x}, size={:#x}", paddr, size);
        let header = unsafe { &mut *(vaddr as *mut VirtIOHeader) };
        info!(
            "Detected virtio device with vendor id {:#X}",
            header.vendor_id()
        );
        info!("Device tree node {:?}", node);
        match header.device_type() {
            DeviceType::Block => virtio_blk(header),
            DeviceType::GPU => virtio_gpu(header),
            DeviceType::Input => virtio_input(header),
            DeviceType::Network => virtio_net(header),
            t => warn!("Unrecognized virtio device: {:?}", t),
        }
    }
}
```

主体过程为获取每个设备对应的VirtIOHeader，然后根据其设备类型，去执行其对应的处理程序。



#### 各类设备的初始化处理程序

```rust
// 主要是检测write_block()和read_block()函数
fn virtio_blk(header: &'static mut VirtIOHeader) {
    let mut blk = VirtIOBlk::new(header).expect("failed to create blk driver");
    let mut input = vec![0xffu8; 512];
    let mut output = vec![0; 512];
    for i in 0..32 {
        for x in input.iter_mut() {
            *x = i as u8;
        }
        blk.write_block(i, &input).expect("failed to write");
        blk.read_block(i, &mut output).expect("failed to read");
        assert_eq!(input, output);
    }
    info!("virtio-blk test finished");
}

//主要是给gpu的各个像素赋值，以测试效果
fn virtio_gpu(header: &'static mut VirtIOHeader) {
    let mut gpu = VirtIOGpu::new(header).expect("failed to create gpu driver");
    let fb = gpu.setup_framebuffer().expect("failed to get fb");
    for y in 0..768 {
        for x in 0..1024 {
            let idx = (y * 1024 + x) * 4;
            fb[idx] = x as u8;
            fb[idx + 1] = y as u8;
            fb[idx + 2] = (x + y) as u8;
        }
    }
    gpu.flush().expect("failed to flush");
    info!("virtio-gpu test finished");
}
```

可以看到，初始化检测程序就是对设备的情况进行检测，调用设备驱动提供的方法来运行设备。下面，我们进入具体的设备驱动程序来研究它们的实现。



#### VirtIOHeader与VirtQueue结构

在对设备树进行检索后，我们可以获取设备的VirtIOHeader，通过这个结构，我们就可以获取设备的全部信息，并且获取访问设备的方式：

```rust
pub struct VirtIOHeader {
    /// Magic value
    magic: ReadOnly<u32>,

    /// Device version number
    ///
    /// Legacy device returns value 0x1.
    version: ReadOnly<u32>,

    /// Virtio Subsystem Device ID
    device_id: ReadOnly<u32>,

    /// Virtio Subsystem Vendor ID
    vendor_id: ReadOnly<u32>,

    /// Flags representing features the device supports
    device_features: ReadOnly<u32>,

    /// Device (host) features word selection
    device_features_sel: WriteOnly<u32>,

    /// Reserved
    __r1: [ReadOnly<u32>; 2],

    /// Flags representing device features understood and activated by the driver
    driver_features: WriteOnly<u32>,

    /// Activated (guest) features word selection
    driver_features_sel: WriteOnly<u32>,

    /// Guest page size
    ///
    /// The driver writes the guest page size in bytes to the register during
    /// initialization, before any queues are used. This value should be a
    /// power of 2 and is used by the device to calculate the Guest address
    /// of the first queue page (see QueuePFN).
    guest_page_size: WriteOnly<u32>,

    /// Reserved
    __r2: ReadOnly<u32>,

    /// Virtual queue index
    ///
    /// Writing to this register selects the virtual queue that the following
    /// operations on the QueueNumMax, QueueNum, QueueAlign and QueuePFN
    /// registers apply to. The index number of the first queue is zero (0x0).
    queue_sel: WriteOnly<u32>,

    /// Maximum virtual queue size
    ///
    /// Reading from the register returns the maximum size of the queue the
    /// device is ready to process or zero (0x0) if the queue is not available.
    /// This applies to the queue selected by writing to QueueSel and is
    /// allowed only when QueuePFN is set to zero (0x0), so when the queue is
    /// not actively used.
    queue_num_max: ReadOnly<u32>,

    /// Virtual queue size
    ///
    /// Queue size is the number of elements in the queue. Writing to this
    /// register notifies the device what size of the queue the driver will use.
    /// This applies to the queue selected by writing to QueueSel.
    queue_num: WriteOnly<u32>,

    /// Used Ring alignment in the virtual queue
    ///
    /// Writing to this register notifies the device about alignment boundary
    /// of the Used Ring in bytes. This value should be a power of 2 and
    /// applies to the queue selected by writing to QueueSel.
    queue_align: WriteOnly<u32>,

    /// Guest physical page number of the virtual queue
    ///
    /// Writing to this register notifies the device about location of the
    /// virtual queue in the Guest’s physical address space. This value is
    /// the index number of a page starting with the queue Descriptor Table.
    /// Value zero (0x0) means physical address zero (0x00000000) and is illegal.
    /// When the driver stops using the queue it writes zero (0x0) to this
    /// register. Reading from this register returns the currently used page
    /// number of the queue, therefore a value other than zero (0x0) means that
    /// the queue is in use. Both read and write accesses apply to the queue
    /// selected by writing to QueueSel.
    queue_pfn: Volatile<u32>,

    /// new interface only
    queue_ready: Volatile<u32>,

    /// Reserved
    __r3: [ReadOnly<u32>; 2],

    /// Queue notifier
    queue_notify: WriteOnly<u32>,

    /// Reserved
    __r4: [ReadOnly<u32>; 3],

    /// Interrupt status
    interrupt_status: ReadOnly<u32>,

    /// Interrupt acknowledge
    interrupt_ack: WriteOnly<u32>,

    /// Reserved
    __r5: [ReadOnly<u32>; 2],

    /// Device status
    ///
    /// Reading from this register returns the current device status flags.
    /// Writing non-zero values to this register sets the status flags,
    /// indicating the OS/driver progress. Writing zero (0x0) to this register
    /// triggers a device reset. The device sets QueuePFN to zero (0x0) for
    /// all queues in the device. Also see 3.1 Device Initialization.
    status: Volatile<DeviceStatus>,

    /// Reserved
    __r6: [ReadOnly<u32>; 3],

    // new interface only since here
    queue_desc_low: WriteOnly<u32>,
    queue_desc_high: WriteOnly<u32>,

    /// Reserved
    __r7: [ReadOnly<u32>; 2],

    queue_avail_low: WriteOnly<u32>,
    queue_avail_high: WriteOnly<u32>,

    /// Reserved
    __r8: [ReadOnly<u32>; 2],

    queue_used_low: WriteOnly<u32>,
    queue_used_high: WriteOnly<u32>,

    /// Reserved
    __r9: [ReadOnly<u32>; 21],

    config_generation: ReadOnly<u32>,
}
```

这里我们对其比较重要的接口来进行解释：

```rust
impl VirtIOHeader{
	     /// Notify device.
    pub fn notify(&mut self, queue: u32) {    //这个接口提示设备去取queue_idx为queue的VirtQueue中的命令去执行
        self.queue_notify.write(queue);
    }
    
    /// Acknowledge interrupt and return true if success.
    pub fn ack_interrupt(&mut self) -> bool {
        let interrupt = self.interrupt_status.read();
        if interrupt != 0 {
            self.interrupt_ack.write(interrupt);
            true
        } else {
            false
        }
    }
    /// Begin initializing the device.
    ///
    /// Ref: virtio 3.1.1 Device Initialization
    pub fn begin_init(&mut self, negotiate_features: impl FnOnce(u64) -> u64) {
        self.status.write(DeviceStatus::ACKNOWLEDGE);
        self.status.write(DeviceStatus::DRIVER);

        let features = self.read_device_features();
        self.write_driver_features(negotiate_features(features));
        self.status.write(DeviceStatus::FEATURES_OK);

        self.guest_page_size.write(PAGE_SIZE as u32);
    }

    /// Finish initializing the device.
    pub fn finish_init(&mut self) {
        self.status.write(DeviceStatus::DRIVER_OK);
    }
    
}
```

**VirtQueue结构**

```rust
pub struct VirtQueue<'a> {
    /// DMA guard
    dma: DMA,
    /// Descriptor table
    desc: &'a mut [Descriptor],
    /// Available ring
    avail: &'a mut AvailRing,
    /// Used ring
    used: &'a mut UsedRing,

    /// The index of queue
    queue_idx: u32,
    /// The size of queue
    queue_size: u16,
    /// The number of used queues.
    num_used: u16,
    /// The head desc index of the free list.
    free_head: u16,
    avail_idx: u16,
    last_used_idx: u16,
}
```

VirtQueue是一个请求队列，可以用来缓存向设备发出的各种命令。其内部构造如何解析已经在SBI中做好了约定。

下面介绍VirtQuee的主要接口：

```rust
impl VirtQueue<'_> {
    /// Create a new VirtQueue.
    pub fn new(header: &mut VirtIOHeader, idx: usize, size: u16) -> Result<Self> {
        if header.queue_used(idx as u32) {
            return Err(Error::AlreadyUsed);
        }
        if !size.is_power_of_two() || header.max_queue_size() < size as u32 {
            return Err(Error::InvalidParam);
        }
        let layout = VirtQueueLayout::new(size);
        // alloc continuous pages
        let dma = DMA::new(layout.size / PAGE_SIZE)?;

        header.queue_set(idx as u32, size as u32, PAGE_SIZE as u32, dma.pfn());

        let desc =
            unsafe { slice::from_raw_parts_mut(dma.vaddr() as *mut Descriptor, size as usize) };
        let avail = unsafe { &mut *((dma.vaddr() + layout.avail_offset) as *mut AvailRing) };
        let used = unsafe { &mut *((dma.vaddr() + layout.used_offset) as *mut UsedRing) };

        // link descriptors together
        for i in 0..(size - 1) {
            desc[i as usize].next.write(i + 1);
        }

        Ok(VirtQueue {
            dma,
            desc,
            avail,
            used,
            queue_size: size,
            queue_idx: idx as u32,
            num_used: 0,
            free_head: 0,
            avail_idx: 0,
            last_used_idx: 0,
        })
    }

    /// Add buffers to the virtqueue, return a token.
    ///
    /// Ref: linux virtio_ring.c virtqueue_add
    pub fn add(&mut self, inputs: &[&[u8]], outputs: &[&mut [u8]]) -> Result<u16> {
        if inputs.is_empty() && outputs.is_empty() {
            return Err(Error::InvalidParam);
        }
        if inputs.len() + outputs.len() + self.num_used as usize > self.queue_size as usize {
            return Err(Error::BufferTooSmall);
        }

        // allocate descriptors from free list
        let head = self.free_head;
        let mut last = self.free_head;
        for input in inputs.iter() {
            let desc = &mut self.desc[self.free_head as usize];
            desc.set_buf(input);
            desc.flags.write(DescFlags::NEXT);
            last = self.free_head;
            self.free_head = desc.next.read();
        }
        for output in outputs.iter() {
            let desc = &mut self.desc[self.free_head as usize];
            desc.set_buf(output);
            desc.flags.write(DescFlags::NEXT | DescFlags::WRITE);
            last = self.free_head;
            self.free_head = desc.next.read();
        }
        // set last_elem.next = NULL
        {
            let desc = &mut self.desc[last as usize];
            let mut flags = desc.flags.read();
            flags.remove(DescFlags::NEXT);
            desc.flags.write(flags);
        }
        self.num_used += (inputs.len() + outputs.len()) as u16;

        let avail_slot = self.avail_idx & (self.queue_size - 1);
        self.avail.ring[avail_slot as usize].write(head);

        // write barrier
        fence(Ordering::SeqCst);

        // increase head of avail ring
        self.avail_idx = self.avail_idx.wrapping_add(1);
        self.avail.idx.write(self.avail_idx);
        Ok(head)
    }

    /// Whether there is a used element that can pop.
    pub fn can_pop(&self) -> bool {
        self.last_used_idx != self.used.idx.read()
    }


    /// Recycle descriptors in the list specified by head.
    ///
    /// This will push all linked descriptors at the front of the free list.
    fn recycle_descriptors(&mut self, mut head: u16) {
        let origin_free_head = self.free_head;
        self.free_head = head;
        loop {
            let desc = &mut self.desc[head as usize];
            let flags = desc.flags.read();
            self.num_used -= 1;
            if flags.contains(DescFlags::NEXT) {
                head = desc.next.read();
            } else {
                desc.next.write(origin_free_head);
                return;
            }
        }
    }

    /// Get a token from device used buffers, return (token, len).
    ///
    /// Ref: linux virtio_ring.c virtqueue_get_buf_ctx
    pub fn pop_used(&mut self) -> Result<(u16, u32)> {
        if !self.can_pop() {
            return Err(Error::NotReady);
        }
        // read barrier
        fence(Ordering::SeqCst);

        let last_used_slot = self.last_used_idx & (self.queue_size - 1);
        let index = self.used.ring[last_used_slot as usize].id.read() as u16;
        let len = self.used.ring[last_used_slot as usize].len.read();

        self.recycle_descriptors(index);
        self.last_used_idx = self.last_used_idx.wrapping_add(1);

        Ok((index, len))
    }

    /// Return size of the queue.
    pub fn size(&self) -> u16 {
        self.queue_size
    }
}
```



#### gpu的设备驱动程序

代表gpu设备的数据结构为：

```rust
pub struct VirtIOGpu<'a> {
    header: &'static mut VirtIOHeader,
    rect: Rect,
    /// DMA area of frame buffer.
    frame_buffer_dma: Option<DMA>,
    /// DMA area of cursor image buffer.
    cursor_buffer_dma: Option<DMA>,
    /// Queue for sending control commands.
    control_queue: VirtQueue<'a>,
    /// Queue for sending cursor commands.
    cursor_queue: VirtQueue<'a>,
    /// Queue buffer DMA
    queue_buf_dma: DMA,
    /// Send buffer for queue.
    queue_buf_send: &'a mut [u8],
    /// Recv buffer for queue.
    queue_buf_recv: &'a mut [u8],
}
```

这里使用了两个结构DMA，分析这个结构：



**DMA结构**

```rust
pub struct DMA {
    paddr: u32,
    pages: u32,
}

impl DMA {
    pub fn new(pages: usize) -> Result<Self> {
        let paddr = unsafe { virtio_dma_alloc(pages) };
        if paddr == 0 {
            return Err(Error::DmaError);
        }
        Ok(DMA {
            paddr: paddr as u32,
            pages: pages as u32,
        })
    }

    pub fn paddr(&self) -> usize {
        self.paddr as usize
    }

    pub fn vaddr(&self) -> usize {
        phys_to_virt(self.paddr as usize)
    }

    /// Page frame number
    pub fn pfn(&self) -> u32 {
        self.paddr >> 12
    }

    /// Convert to a buffer
    pub unsafe fn as_buf(&self) -> &'static mut [u8] {
        core::slice::from_raw_parts_mut(self.vaddr() as _, PAGE_SIZE * self.pages as usize)
    }
}
```

DMA是分配物理内存的结构，可以看到是以page为单位的，一个page占4KB，一个DMA表示一段连续的物理页面。



GPU的初始化程序为：

```rust
/// Create a new VirtIO-Gpu driver.
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {
        header.begin_init(|features| {
            let features = Features::from_bits_truncate(features);
            info!("Device features {:?}", features);
            let supported_features = Features::empty();
            (features & supported_features).bits()
        });

        // read configuration space
        let config = unsafe { &mut *(header.config_space() as *mut Config) };
        info!("Config: {:?}", config);

        let control_queue = VirtQueue::new(header, QUEUE_TRANSMIT, 2)?;
        let cursor_queue = VirtQueue::new(header, QUEUE_CURSOR, 2)?;

        let queue_buf_dma = DMA::new(2)?;
        let queue_buf_send = unsafe { &mut queue_buf_dma.as_buf()[..PAGE_SIZE] };
        let queue_buf_recv = unsafe { &mut queue_buf_dma.as_buf()[PAGE_SIZE..] };

        header.finish_init();

        Ok(VirtIOGpu {
            header,
            frame_buffer_dma: None,
            cursor_buffer_dma: None,
            rect: Rect::default(),
            control_queue,
            cursor_queue,
            queue_buf_dma,
            queue_buf_send,
            queue_buf_recv,
        })
    }
```

可以看到，两个VirtQueue的id分别设置为了QUEUE_TRANSMIT和QUEUE_CURSOR，因此，当需要对应的命令的时候，实际上只需要往对应的queue中放入命令，然后notify设备即可：

```rust
impl VirtIOGpu<'_> {
    /// Send a request to the device and block for a response.
    fn request<Req, Rsp>(&mut self, req: Req) -> Result<Rsp> {
        unsafe {
            (self.queue_buf_send.as_mut_ptr() as *mut Req).write(req);
        }
        self.control_queue
            .add(&[self.queue_buf_send], &[self.queue_buf_recv])?;
        self.header.notify(QUEUE_TRANSMIT as u32);
        while !self.control_queue.can_pop() {
            spin_loop();
        }
        self.control_queue.pop_used()?;
        Ok(unsafe { (self.queue_buf_recv.as_ptr() as *const Rsp).read() })
    }
    
/// Send a mouse cursor operation request to the device and block for a response.
    fn cursor_request<Req>(&mut self, req: Req) -> Result {
        unsafe {
            (self.queue_buf_send.as_mut_ptr() as *mut Req).write(req);
        }
        self.cursor_queue.add(&[self.queue_buf_send], &[])?;
        self.header.notify(QUEUE_CURSOR as u32);
        while !self.cursor_queue.can_pop() {
            spin_loop();
        }
        self.cursor_queue.pop_used()?;
        Ok(())
    }
}
```

Req是想要想GPU设备发出的命令，可以看到，先把命令写入queue_buf_send，再给出对应的通知self.header.notify(QUEUE_TRANSMIT as u32)，最后获取

self.queue_buf_recv中的返回值。至于其具体底层如何实现，实际上在SBI执行阶段就提供了这样的接口，我们只需要去读写对应的内存单元，自然就相当于去读写设备了。

在此基础上，我们给出了对设备的一系列命令：

```rust
impl VirtIOGpu<'_> {
    fn get_display_info(&mut self) -> Result<RespDisplayInfo>
    fn resource_create_2d(&mut self, resource_id: u32, width: u32, height: u32) -> Result
    fn set_scanout(&mut self, rect: Rect, scanout_id: u32, resource_id: u32) -> Result
    fn resource_flush(&mut self, rect: Rect, resource_id: u32) -> Result
    fn transfer_to_host_2d(&mut self, rect: Rect, offset: u64, resource_id: u32) -> Result
    fn resource_attach_backing(&mut self, resource_id: u32, paddr: u64, length: u32) -> Result
    fn update_cursor(
        &mut self,
        resource_id: u32,
        scanout_id: u32,
        pos_x: u32,
        pos_y: u32,
        hot_x: u32,
        hot_y: u32,
        is_move: bool,
    ) -> Result
    
}
```

下面我们来看为VirtIOGpu封装的其它方法：

```rust
impl VirtIOGpu<'_> {
    /// Create a new VirtIO-Gpu driver.
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {
        header.begin_init(|features| {
            let features = Features::from_bits_truncate(features);
            info!("Device features {:?}", features);
            let supported_features = Features::empty();
            (features & supported_features).bits()
        });

        // read configuration space
        let config = unsafe { &mut *(header.config_space() as *mut Config) };
        info!("Config: {:?}", config);

        let control_queue = VirtQueue::new(header, QUEUE_TRANSMIT, 2)?;
        let cursor_queue = VirtQueue::new(header, QUEUE_CURSOR, 2)?;

        let queue_buf_dma = DMA::new(2)?;
        let queue_buf_send = unsafe { &mut queue_buf_dma.as_buf()[..PAGE_SIZE] };
        let queue_buf_recv = unsafe { &mut queue_buf_dma.as_buf()[PAGE_SIZE..] };

        header.finish_init();

        Ok(VirtIOGpu {
            header,
            frame_buffer_dma: None,
            cursor_buffer_dma: None,
            rect: Rect::default(),
            control_queue,
            cursor_queue,
            queue_buf_dma,
            queue_buf_send,
            queue_buf_recv,
        })
    }

    /// Acknowledge interrupt.
    pub fn ack_interrupt(&mut self) -> bool {
        self.header.ack_interrupt()
    }

    /// Get the resolution (width, height).
    pub fn resolution(&self) -> (u32, u32) {
        (self.rect.width, self.rect.height)
    }

    /// Setup framebuffer
    pub fn setup_framebuffer(&mut self) -> Result<&mut [u8]> {
        // get display info
        let display_info = self.get_display_info()?;
        info!("=> {:?}", display_info);
        self.rect = display_info.rect;

        // create resource 2d
        self.resource_create_2d(
            RESOURCE_ID_FB,
            display_info.rect.width,
            display_info.rect.height,
        )?;

        // alloc continuous pages for the frame buffer
        let size = display_info.rect.width * display_info.rect.height * 4;
        let frame_buffer_dma = DMA::new(pages(size as usize))?;

        // resource_attach_backing
        self.resource_attach_backing(RESOURCE_ID_FB, frame_buffer_dma.paddr() as u64, size)?;

        // map frame buffer to screen
        self.set_scanout(display_info.rect, SCANOUT_ID, RESOURCE_ID_FB)?;

        let buf = unsafe { frame_buffer_dma.as_buf() };
        self.frame_buffer_dma = Some(frame_buffer_dma);
        Ok(buf)
    }

    /// Flush framebuffer to screen.
    pub fn flush(&mut self) -> Result {
        // copy data from guest to host
        self.transfer_to_host_2d(self.rect, 0, RESOURCE_ID_FB)?;
        // flush data to screen
        self.resource_flush(self.rect, RESOURCE_ID_FB)?;
        Ok(())
    }

    /// Set the pointer shape and position.
    pub fn setup_cursor(
        &mut self,
        cursor_image: &[u8],
        pos_x: u32,
        pos_y: u32,
        hot_x: u32,
        hot_y: u32,
    ) -> Result {
        let size = CURSOR_RECT.width * CURSOR_RECT.height * 4;
        if cursor_image.len() != size as usize {
            return Err(Error::InvalidParam);
        }
        let cursor_buffer_dma = DMA::new(pages(size as usize))?;
        let buf = unsafe { cursor_buffer_dma.as_buf() };
        buf.copy_from_slice(cursor_image);

        self.resource_create_2d(RESOURCE_ID_CURSOR, CURSOR_RECT.width, CURSOR_RECT.height)?;
        self.resource_attach_backing(RESOURCE_ID_CURSOR, cursor_buffer_dma.paddr() as u64, size)?;
        self.transfer_to_host_2d(CURSOR_RECT, 0, RESOURCE_ID_CURSOR)?;
        self.update_cursor(
            RESOURCE_ID_CURSOR,
            SCANOUT_ID,
            pos_x,
            pos_y,
            hot_x,
            hot_y,
            false,
        )?;
        self.cursor_buffer_dma = Some(cursor_buffer_dma);
        Ok(())
    }

    /// Move the pointer without updating the shape.
    pub fn move_cursor(&mut self, pos_x: u32, pos_y: u32) -> Result {
        self.update_cursor(RESOURCE_ID_CURSOR, SCANOUT_ID, pos_x, pos_y, 0, 0, true)?;
        Ok(())
    }
}
```





至此，我们获取了控制GPU的所有的接口，便可以回到example中去完成初始化工作了：

```rust
//主要是给gpu的各个像素赋值，以测试效果
fn virtio_gpu(header: &'static mut VirtIOHeader) {
    let mut gpu = VirtIOGpu::new(header).expect("failed to create gpu driver");
    let fb = gpu.setup_framebuffer().expect("failed to get fb");
    for y in 0..768 {
        for x in 0..1024 {
            let idx = (y * 1024 + x) * 4;
            fb[idx] = x as u8;
            fb[idx + 1] = y as u8;
            fb[idx + 2] = (x + y) as u8;
        }
    }
    gpu.flush().expect("failed to flush");
    info!("virtio-gpu test finished");
}
```



#### Block设备的设备驱动程序

```rust
pub struct VirtIOBlk<'a> {
    header: &'static mut VirtIOHeader,
    queue: VirtQueue<'a>,
    capacity: usize,
}
```

其主体函数的实现与GPU类似：

```rust
impl VirtIOBlk<'_> {
    /// Create a new VirtIO-Blk driver.
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {
        header.begin_init(|features| {
            let features = BlkFeature::from_bits_truncate(features);
            info!("device features: {:?}", features);
            // negotiate these flags only
            let supported_features = BlkFeature::empty();
            (features & supported_features).bits()
        });

        // read configuration space
        let config = unsafe { &mut *(header.config_space() as *mut BlkConfig) };
        info!("config: {:?}", config);
        info!(
            "found a block device of size {}KB",
            config.capacity.read() / 2
        );

        let queue = VirtQueue::new(header, 0, 16)?;
        header.finish_init();

        Ok(VirtIOBlk {
            header,
            queue,
            capacity: config.capacity.read() as usize,
        })
    }

    /// Acknowledge interrupt.
    pub fn ack_interrupt(&mut self) -> bool {
        self.header.ack_interrupt()
    }

    /// Read a block.
    pub fn read_block(&mut self, block_id: usize, buf: &mut [u8]) -> Result {
        assert_eq!(buf.len(), BLK_SIZE);
        let req = BlkReq {
            type_: ReqType::In,
            reserved: 0,
            sector: block_id as u64,
        };
        let mut resp = BlkResp::default();
        self.queue.add(&[req.as_buf()], &[buf, resp.as_buf_mut()])?;
        self.header.notify(0);
        while !self.queue.can_pop() {
            spin_loop();
        }
        self.queue.pop_used()?;
        match resp.status {
            RespStatus::Ok => Ok(()),
            _ => Err(Error::IoError),
        }
    }

    /// Read a block in a non-blocking way which means that it returns immediately.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The identifier of the block to read.
    /// * `buf` - The buffer in the memory which the block is read into.
    /// * `resp` - A mutable reference to a variable provided by the caller
    ///   which contains the status of the requests. The caller can safely
    ///   read the variable only after the request is ready.
    ///
    /// # Usage
    ///
    /// It will submit request to the virtio block device and return a token identifying
    /// the position of the first Descriptor in the chain. If there are not enough
    /// Descriptors to allocate, then it returns [Error::BufferTooSmall].
    ///
    /// After the request is ready, `resp` will be updated and the caller can get the
    /// status of the request(e.g. succeed or failed) through it. However, the caller
    /// **must not** spin on `resp` to wait for it to change. A safe way is to read it
    /// after the same token as this method returns is fetched through [VirtIOBlk::pop_used()],
    /// which means that the request has been ready.
    ///
    /// # Safety
    ///
    /// `buf` is still borrowed by the underlying virtio block device even if this
    /// method returns. Thus, it is the caller's responsibility to guarantee that
    /// `buf` is not accessed before the request is completed in order to avoid
    /// data races.
    pub unsafe fn read_block_nb(
        &mut self,
        block_id: usize,
        buf: &mut [u8],
        resp: &mut BlkResp,
    ) -> Result<u16> {
        assert_eq!(buf.len(), BLK_SIZE);
        let req = BlkReq {
            type_: ReqType::In,
            reserved: 0,
            sector: block_id as u64,
        };
        let token = self.queue.add(&[req.as_buf()], &[buf, resp.as_buf_mut()])?;
        self.header.notify(0);
        Ok(token)
    }

    /// Write a block.
    pub fn write_block(&mut self, block_id: usize, buf: &[u8]) -> Result {
        assert_eq!(buf.len(), BLK_SIZE);
        let req = BlkReq {
            type_: ReqType::Out,
            reserved: 0,
            sector: block_id as u64,
        };
        let mut resp = BlkResp::default();
        self.queue.add(&[req.as_buf(), buf], &[resp.as_buf_mut()])?;
        self.header.notify(0);
        while !self.queue.can_pop() {
            spin_loop();
        }
        self.queue.pop_used()?;
        match resp.status {
            RespStatus::Ok => Ok(()),
            _ => Err(Error::IoError),
        }
    }

    //// Write a block in a non-blocking way which means that it returns immediately.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The identifier of the block to write.
    /// * `buf` - The buffer in the memory containing the data to write to the block.
    /// * `resp` - A mutable reference to a variable provided by the caller
    ///   which contains the status of the requests. The caller can safely
    ///   read the variable only after the request is ready.
    ///
    /// # Usage
    ///
    /// See also [VirtIOBlk::read_block_nb()].
    ///
    /// # Safety
    ///
    /// See also [VirtIOBlk::read_block_nb()].
    pub unsafe fn write_block_nb(
        &mut self,
        block_id: usize,
        buf: &[u8],
        resp: &mut BlkResp,
    ) -> Result<u16> {
        assert_eq!(buf.len(), BLK_SIZE);
        let req = BlkReq {
            type_: ReqType::Out,
            reserved: 0,
            sector: block_id as u64,
        };
        let token = self.queue.add(&[req.as_buf(), buf], &[resp.as_buf_mut()])?;
        self.header.notify(0);
        Ok(token)
    }

    
}
```



### 内核部分

virtio-drivers提供了对部分virtIO设备的驱动，但是它还没有与内核进行联动，与此同时，接受输入输出的串口等设备的驱动还没有得到支持，因为我们继续对内核部分进行编写。

```
[dependencies]
......
virtio-drivers = { git = "https://github.com/rcore-os/virtio-drivers" }
......
```

在cargo.toml中将virtio-drivers crate引入，这样我们就能直接使用上面介绍的各种机制。



#### 设备初始化

```rust
#[no_mangle]
pub fn rust_main() -> ! {
    ......
    board::device_init();
    ......
}
```

```rust
pub fn device_init() {
    use riscv::register::sie;
    let mut plic = unsafe { PLIC::new(VIRT_PLIC) };
    let hart_id: usize = 0;
    let supervisor = IntrTargetPriority::Supervisor;
    let machine = IntrTargetPriority::Machine;
    plic.set_threshold(hart_id, supervisor, 0);
    plic.set_threshold(hart_id, machine, 1);
    //irq nums: 5 keyboard, 6 mouse, 8 block, 10 uart
    for intr_src_id in [5usize, 6, 8 , 10] {
        plic.enable(hart_id, supervisor, intr_src_id);
        plic.set_priority(intr_src_id, 1);
    }
    unsafe {
        sie::set_sext();
    }
}
```

这部分的操作主要完成 PLIC 的初始化，设置好外设中断优先级、外设中断的阈值，激活外设中断。



```rust
pub fn irq_handler() {
    let mut plic = unsafe { PLIC::new(VIRT_PLIC) };
    let intr_src_id = plic.claim(0, IntrTargetPriority::Supervisor);
    match intr_src_id {
        5 => KEYBOARD_DEVICE.handle_irq(),
        6 => MOUSE_DEVICE.handle_irq(),
        8 => BLOCK_DEVICE.handle_irq(),
        10 => UART.handle_irq(),
        _ => panic!("unsupported IRQ {}", intr_src_id),
    }
    plic.complete(0, IntrTargetPriority::Supervisor, intr_src_id);
}
```

irq_handler() 实现对各种外设中断的分发。外设产生中断之后，scause 寄存器中会记录中断的类型，PLIC 的 Claim 寄存器会保存具体的外设中断类型（5  keyboard, 6 mouse, 8 block, 10 uart），然后进入到 irq_handler() 外设中断处理函数，根据  PLIC 的 Claim 寄存器保存的信息，进入到具体的外设处理函数

操作系统处理完毕之后，将 PLIC 的 Complete 寄存器设置为对应的中断源 id 来告知 PLIC 已经处理完毕。



#### 接受来自内核态下的外设中断

```rust
#[no_mangle]
pub fn trap_from_kernel(_trap_cx: &TrapContext) {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            crate::board::irq_handler();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            check_timer();
            // do not schedule now
        }
        _ => {
            panic!(
                "Unsupported trap from kernel: {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
}
```



#### 串口的中断处理函数

**串口寄存器中的一些定义**

```rust
#[repr(C)]
#[allow(dead_code)]
struct ReadWithoutDLAB {
    /// receiver buffer register
    pub rbr: ReadOnly<u8>,
    /// interrupt enable register
    pub ier: Volatile<IER>,
    /// interrupt identification register
    pub iir: ReadOnly<u8>,
    /// line control register
    pub lcr: Volatile<u8>,
    /// model control register
    pub mcr: Volatile<MCR>,
    /// line status register
    pub lsr: ReadOnly<LSR>,
    /// ignore MSR
    _padding1: ReadOnly<u8>,
    /// ignore SCR
    _padding2: ReadOnly<u8>,
}


#[repr(C)]
#[allow(dead_code)]
struct WriteWithoutDLAB {
    /// transmitter holding register
    pub thr: WriteOnly<u8>,
    /// interrupt enable register
    pub ier: Volatile<IER>,
    /// ignore FCR
    _padding0: ReadOnly<u8>,
    /// line control register
    pub lcr: Volatile<u8>,
    /// modem control register
    pub mcr: Volatile<MCR>,
    /// line status register
    pub lsr: ReadOnly<LSR>,
    /// ignore other registers
    _padding1: ReadOnly<u16>,
}


```

**串口定义**

```rust
pub struct NS16550aRaw {
    base_addr: usize,
}

impl NS16550aRaw {
    fn read_end(&mut self) -> &mut ReadWithoutDLAB {
        unsafe { &mut *(self.base_addr as *mut ReadWithoutDLAB) }
    }

    fn write_end(&mut self) -> &mut WriteWithoutDLAB {
        unsafe { &mut *(self.base_addr as *mut WriteWithoutDLAB) }
    }

    pub fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }

    pub fn init(&mut self) {
        let read_end = self.read_end();
        let mut mcr = MCR::empty();
        mcr |= MCR::DATA_TERMINAL_READY;
        mcr |= MCR::REQUEST_TO_SEND;
        mcr |= MCR::AUX_OUTPUT2;
        read_end.mcr.write(mcr);
        let ier = IER::RX_AVAILABLE;
        read_end.ier.write(ier);
    }

    pub fn read(&mut self) -> Option<u8> {
        let read_end = self.read_end();
        let lsr = read_end.lsr.read();
        if lsr.contains(LSR::DATA_AVAILABLE) {
            Some(read_end.rbr.read())
        } else {
            None
        }
    }

    pub fn write(&mut self, ch: u8) {
        let write_end = self.write_end();
        loop {
            if write_end.lsr.read().contains(LSR::THR_EMPTY) {
                write_end.thr.write(ch);
                break;
            }
        }
    }
}
```

**对串口的进一步封装**

```rust
struct NS16550aInner {
    ns16550a: NS16550aRaw,                                   //串口本体
    read_buffer: VecDeque<u8>,                               //串口数据的缓冲
}

pub struct NS16550a<const BASE_ADDR: usize> {
    inner: UPIntrFreeCell<NS16550aInner>,
    condvar: Condvar,                                       //用来阻塞无法获取数据的线程
}
```

```rust
impl<const BASE_ADDR: usize> CharDevice for NS16550a<BASE_ADDR> {
    fn read(&self) -> u8 {
        loop {
            let mut inner = self.inner.exclusive_access();
            if let Some(ch) = inner.read_buffer.pop_front() {
                return ch;
            } else {
                let task_cx_ptr = self.condvar.wait_no_sched();
                drop(inner);
                schedule(task_cx_ptr);
            }
        }
    }
    fn write(&self, ch: u8) {
        let mut inner = self.inner.exclusive_access();
        inner.ns16550a.write(ch);
    }
    fn handle_irq(&self) {
        let mut count = 0;
        self.inner.exclusive_session(|inner| {
            while let Some(ch) = inner.ns16550a.read() {
                count += 1;
                inner.read_buffer.push_back(ch);
            }
        });
        if count > 0 {
            self.condvar.signal();
        }
    }
}
```

这里其它部分比较平凡，但是注意在handle_irq中有一步特殊的处理，即每次对串口处理中断时会将串口数据读入缓冲区。



#### OS对输入设备中断的支持

```rust
impl INPUTDevice for VirtIOINPUT {
    fn handle_irq(&self) {
        let mut input = self.0.exclusive_access();
        input.ack_interrupt();
        let event = input.pop_pending_event().unwrap();
        let dtype = match Decoder::decode(
            event.event_type as usize,
            event.code as usize,
            event.value as usize,
        ) {
            Ok(dtype) => dtype,
            Err(_) => return,
        };
        match dtype {
            virtio_input_decoder::DecodeType::Key(key, r#type) => {
                println!("{:?} {:?}", key, r#type);
                if r#type == KeyType::Press {
                    let mut inner = PAD.exclusive_access();
                    let a = inner.as_ref().unwrap();
                    match key.to_char() {
                        Ok(mut k) => {
                            if k == '\r' {
                                a.repaint(k.to_string() + "\n")
                            } else {
                                a.repaint(k.to_string())
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            virtio_input_decoder::DecodeType::Mouse(mouse) => println!("{:?}", mouse),
        }
    }
}
```

可以看到，当输入来自于键盘时，会对输入进行解析，并且在图形界面中体现出来，PAD来自于图形界面模块。而输入为鼠标暂时不做处理。



#### OS对图形界面模块的支持

在前面已经获取对VirtGPU的支持的情况下，可以进一步做出对图形界面的支持。

图形界面的组建比较多，这里选择其中几个来进行分析。

**组件特性**

```rust
pub trait Component: Send + Sync + Any {
    fn paint(&self);                                                     //根据组建绘制图形
    fn add(&self, comp: Arc<dyn Component>);                             //在组件下加入子组件
    fn bound(&self) -> (Size, Point);                                    //获取组件的大小和位置
}
```

每个gui中的组件都拥有以上三个特性。



**图形绘制**

```rust
#[derive(Clone)]
pub struct Graphics {
    pub size: Size,
    pub point: Point,
    pub drv: Arc<dyn GPUDevice>,
}

impl Graphics {
    pub fn new(size: Size, point: Point) -> Self {
        Self {
            size,
            point,
            drv: GPU_DEVICE.clone(),
        }
    }
}


impl DrawTarget for Graphics {
    type Color = Rgb888;

    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let fb = self.drv.getfreambuffer();

        pixels.into_iter().for_each(|px| {
            let idx = ((self.point.y + px.0.y) * VIRTGPU_XRES as i32 + self.point.x + px.0.x) as usize * 4;
            if idx + 2 >= fb.len() {
                return;
            }
            fb[idx] = px.1.b();
            fb[idx + 1] = px.1.g();
            fb[idx + 2] = px.1.r();
        });
        self.drv.flush();
        Ok(())
    }
}
```

**组件绘制**

```rust
pub struct Panel {
    inner: UPIntrFreeCell<PanelInner>,
}
struct PanelInner {
    graphic: Graphics,
    comps: VecDeque<Arc<dyn Component>>,
}


impl Component for Panel {
    fn paint(&self) {
        let mut inner = self.inner.exclusive_access();

        Rectangle::new(Point::new(0, 0), inner.graphic.size)
            .into_styled(PrimitiveStyle::with_fill(Rgb888::WHITE))
            .draw(&mut inner.graphic)
            .unwrap();

        let len = inner.comps.len();
        drop(inner);
        for i in 0..len {
            let mut inner = self.inner.exclusive_access();
            let comp = Arc::downgrade(&inner.comps[i]);
            drop(inner);
            comp.upgrade().unwrap().paint();
        }
    }
}
```

可以看到这是一个递归的过程，每个组件除了绘制自己，也会绘制自己的子组件。具体的绘制过程这里不做复杂的阐述。



#### OS对blk设备的支持

```rust

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut resp = BlkResp::default();
            let task_cx_ptr = self.virtio_blk.exclusive_session(|blk| {
                let token = unsafe { blk.read_block_nb(block_id, buf, &mut resp).unwrap() };
                self.condvars.get(&token).unwrap().wait_no_sched()
            });
            schedule(task_cx_ptr);
            assert_eq!(
                resp.status(),
                RespStatus::Ok,
                "Error when reading VirtIOBlk"
            );
        } else {
            self.virtio_blk
                .exclusive_access()
                .read_block(block_id, buf)
                .expect("Error when reading VirtIOBlk");
        }
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut resp = BlkResp::default();
            let task_cx_ptr = self.virtio_blk.exclusive_session(|blk| {
                let token = unsafe { blk.write_block_nb(block_id, buf, &mut resp).unwrap() };
                self.condvars.get(&token).unwrap().wait_no_sched()
            });
            schedule(task_cx_ptr);
            assert_eq!(
                resp.status(),
                RespStatus::Ok,
                "Error when writing VirtIOBlk"
            );
        } else {
            self.virtio_blk
                .exclusive_access()
                .write_block(block_id, buf)
                .expect("Error when writing VirtIOBlk");
        }
    }
    fn handle_irq(&self) {
        self.virtio_blk.exclusive_session(|blk| {
            while let Ok(token) = blk.pop_used() {
                self.condvars.get(&token).unwrap().signal();
            }
        });
    }
}
```

这里直接调用virtio-drivers的方法即可。不过在virtio-drivers中我们遗留了一个问题，就是对dma-alloc相关的实现，在os中，我们发现可以用前面章节实现的物理页帧分配器来实现：

```rust
#[no_mangle]
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let mut ppn_base = PhysPageNum(0);
    for i in 0..pages {
        let frame = frame_alloc().unwrap();
        if i == 0 {
            ppn_base = frame.ppn;
        }
        assert_eq!(frame.ppn.0, ppn_base.0 + i);
        QUEUE_FRAMES.exclusive_access().push(frame);
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
        frame_dealloc(ppn_base);
        ppn_base.step();
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr(paddr.0)
}

#[no_mangle]
pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    PageTable::from_token(kernel_token())
        .translate_va(vaddr)
        .unwrap()
}

```



### 实验：贪吃蛇游戏

创建 desktop 对象，记录当前的 main_window，以及所有打开的 window，输入设备的中断最终是交给 main_window 来处理，进而再传递给具体的组件来进行响应：

```rust
lazy_static::lazy_static!{
    pub static ref DESKTOP: Arc<Desktop> = Arc::new(Desktop::new());
}
  
pub struct Desktop {
    pub main_window: Arc<Window>,
    inner: UPIntrFreeCell<DesktopInner>,
}
  
pub struct DesktopInner {
    windows: VecDeque<Arc<Window>>,
}


```

基于 window 来管理各个组件以及消息传递等（实现了 component 特性）

```rust
pub struct Window {
    inner: UPIntrFreeCell<WindowInner>,
}
  
pub struct WindowInner {  
    size: Size,
    point: Point,  
    titel: Option<String>,
    comps: VecDeque<Arc<dyn Component>>,
}


```

**游戏的具体实现**

创建 SnakeGame 对象，包括一个 panel、food 以及 snake，其中 food 用 Button  来表示，snake 则用 VecDeque 来表示，直接利用 button 的 paint 方法来画图，不用再额外写一个直接操作 point  画图的函数，在创建 SnakeGame 时，food 以及 snake 的位置直接固定了

```rust
pub struct SnakeGame {
    inner: UPIntrFreeCell<SnakeInner>,
}
    
pub struct SnakeInner {
    graphic: Graphics,
    pub panel: Arc<Panel>,
    pub food: Button,
    pub snake: VecDeque<Button>,
}


```

细节方面的问题属于绘制细节，不属于OS的问题，便不再这里讨论。