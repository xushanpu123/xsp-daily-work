#                                         chapter4：地址空间

## 实验逻辑和基本设计

#### 实验逻辑

1、实验的目的为：进程创建时，为每个进程分配一张页表（实际上还有更高级的抽象：地址空间和逻辑段，但是都是以页表结构为核心），能够在页表中实现为进程分配新的虚拟页以及释放任意虚拟页的功能。

2、进程的页表均在内核空间中，为了能够给新进程分配页表，需要提供分配页表的策略。

3、为了每个进程都能够自由的分配和回收自身任意虚拟页，需要有探测内存中的空闲物理页帧以及回收任意物理页帧的机制，并且必须有访问和修改随机页表项的接口。



#### 基本设计

1、rcore tutorial采用双页表的设计，即内核使用单独的地址空间，不同的用户进程在用户态时使用各自的用户地址空间，这也就意味着地址空间的切换发生在特权级切换时。

2、内核空间采用对等映射的方式，即内核的任一PageNum号虚拟页对应的物理页框号也是PageNum，这样的话，在内核中访问物理页时便不再会受到MMU的影响。内核页表在操作系统初始化的时候便一并完成对等映射的初始化了。

3、每个用户进程在初始化时，同步为其分配一个物理页作为其页表的根节点，此后所有的操作均在该页表上实现。

4、chapter4的核心内容在于对页表的支持，这里的页表被封装成了一个PageTable数据结构，这个结构实际上控制了所有被该页表占用的页框，需要实现的接口包括：

1）创建一个新的页表，且为其分配一个空闲的页帧作为其根页表；

2）进程分配或回收一个虚拟页时，需要去修改对应的页表项，因此页表必须能够随机访问和修改任意页表项。

3）为了满足1）、2）的要求，必须能探测和分配空闲物理页帧以及回收物理页帧。



#### 注意

本实验采用三级页表结构，所以任意一个虚拟页的访问和处理必须经过三次查表：先通过一级页号去访问根页表对应的页表项，获取对应的二级页表的地址，再通过二级页号去访问对应二级页表中的页表项，从中取出三级页表的地址，再通过三级页号去访问对应页表项，从中获取地址对应的物理页帧。



## 实验过程

#### 1、构建页表相关的数据结构和方法

**建立页与地址之间的转换关系**

```rust
// os/src/mm/address.rs

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);
```

这里的结构均是包裹了一个usize类型，是为了更好的增加类型方法来进行类型转换：

```rust
impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<PhysAddr> for usize {
    fn from(v: PhysAddr) -> Self {
        v.0
    }
}
impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self {
        v.0
    }
}
impl From<VirtAddr> for usize {
    fn from(v: VirtAddr) -> Self {
        v.0
    }
}
impl From<VirtPageNum> for usize {
    fn from(v: VirtPageNum) -> Self {
        v.0
    }
}
```

这里增加了各种结构与usize的from关系，通过这种方式我们可以用较为简单和通用的写法去获取结构包裹的usize值：

```rust
let p:usize = 10086;
let vp = VirtPageNum::from(p);
let p2:usize = vp.into();
//vp是p代表的虚拟页，p2是vp中包裹的usize。
```

```rust
//为各种结构构建方法，提供虚拟页与虚拟地址，物理页与物理地址间的转换方法。其中floor()返回该地址单元所在的页,若地址单元为某页的第一个单元，则ceil（）返回其所在的页，否则返回下一页。
impl VirtAddr {
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }
    pub fn ceil(&self) -> VirtPageNum {
        VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}
impl From<VirtAddr> for VirtPageNum {
    fn from(v: VirtAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
impl From<VirtPageNum> for VirtAddr {
    fn from(v: VirtPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}
impl PhysAddr {
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }
    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}
impl From<PhysAddr> for PhysPageNum {
    fn from(v: PhysAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
impl From<PhysPageNum> for PhysAddr {
    fn from(v: PhysPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}

impl VirtPageNum {                               //该函数用来把一个虚拟页号拆解成一级页号、二级页号和三级页号，返回一个三元数组 
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 511;
            vpn >>= 9;
        }
        idx
    }
}

```

为了更方便的表示一段连续的虚拟页并方便对它的处理，我们做了一些新的定义：

```rust
pub trait StepByOne {
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}
```

为VirtPageNum定义了自增函数step()，并加入到StepByOne这一trait中。



定义了结构体SimpleRange<T>来表示一段区间：

```rust
#[derive(Copy, Clone)]
/// a simple range structure for type T
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}
```



构建了创建SimpleRange<T>的函数，并提供了可以访问它边界的接口。

```rust
impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
}
```



创建了SimpleRange的迭代器：

```rust
/// iterator for the simple range structure
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}
impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}
impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}

impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}
```



最后我们对以上结构进行封装，就可以获取VirtPageNum类型的Range类型了：

```rust
/// a simple range structure for virtual page number
pub type VPNRange = SimpleRange<VirtPageNum>;
```

此后，我们便可以使用VPNRange来对一段虚拟页区间做操作了。



**构建页表项**

​       页表项结构是硬件决定的，在本实验中，页表项是64位二进制，其中[53:10]这44位是物理页号，最低的8位[7:0]

 则是标志位，它们的含义如下：

- 仅当 V(Valid) 位为 1 时，页表项才是合法的；

- R/W/X 分别控制索引到这个页表项的对应虚拟页面是否允许读/写/取指；

- U 控制索引到这个页表项的对应虚拟页面是否在 CPU 处于 U 特权级的情况下是否被允许访问；

- G 我们不理会；

- A(Accessed) 记录自从页表项上的这一位被清零之后，页表项的对应虚拟页面是否被访问过；

- D(Dirty) 则记录自从页表项上的这一位被清零之后，页表项的对应虚拟页表是否被修改过。

  页表存放在内存中，我们想要设置某个虚拟页的页表项，只需要在对应的内存位置装入我们写好的页表项就可以了。页表项的结构为：

  ```rust
  #[derive(Copy, Clone)]
  #[repr(C)]
  /// page table entry structure
  pub struct PageTableEntry {
      pub bits: usize,
  }
  ```

一个usize正好对应64位二进制，因此可以代表一个页表项。我们的重点是构建各种页表项的处理函数：

```rust
impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {      //根据物理页帧号和标记位构建一个PTE
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {                                     //创建一个空的PTE
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {                           //获取PTE中的物理页帧号
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {                            //获取PTE中的标记位
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {                             //判断PTE是否有效
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {                             //判断该页表项对应的物理页帧是否可读
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {                              //判断该页表项对应的物理页帧是否可写
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {                            //判断该页表项对应的物理页帧是否可执行
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}
```



此后，我们只需要查表就可以找到对应的物理页帧，当目标物理页帧存放的是页表的时候，我们希望以pte指针的形式来访问它；当目标页帧存放程序数据的时候，我们则希望能够按指定的类型来访问它，因此，为了方便起见，我们给PhysPageNum提供了如下方法：

```rust
impl PhysPageNum {
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
}

```

获取的指针都是*mut类型，这意味着我们可以通过指针来修改对应的数据。



**分配空闲物理页帧与释放已用页帧**

我们声明一个 `FrameAllocator` Trait 来描述一个物理页帧管理器需要提供哪些功能：

```rust
// os/src/mm/frame_allocator.rs

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}
```

真实的分配器使用一个简单的栈式分配器：

```rust
// os/src/mm/frame_allocator.rs

pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}
```

[current,end)表示从未使用过的且可用的物理页帧号，recycled用来存放被回收的物理页，当需要分配新的页帧的时候，优先从recycled中获取页帧，如果recycled为空，再从[current,end)区域分配可用帧。当页帧被回收的时候，将该栈帧推入recycled中。用代码描述其方式如下：

```rust
// os/src/mm/frame_allocator.rs

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}

// os/src/mm/frame_allocator.rs

impl FrameAllocator for StackFrameAllocator {
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else {
            if self.current == self.end {
                None
            } else {
                self.current += 1;
                Some((self.current - 1).into())
            }
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // validity check
        if ppn >= self.current || self.recycled
            .iter()
            .find(|&v| {*v == ppn})
            .is_some() {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }
}
```



与之前的方法类似，创建一个StackFrameAllocator的全局实例，然后给出各种操作的外部接口。注意，在全局实例中需要将其可用帧初始化为真实可用的页帧范围：

```rust
type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// frame allocator instance through lazy_static!
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// initiate the frame allocator using `ekernel` and `MEMORY_END`
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

/// allocate a frame
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

/// deallocate a frame
fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}
```

注意到frame_alloc()的返回只为Option<FrameTracker>，其定义为：

```rust
/// manage a frame which has the same lifecycle as the tracker
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // page cleaning
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}
```

这样做的好处是，在new()时，可以将对应的页帧清空，更重要的是，当FrameTracker超出生命周期时，将自动释放掉对应的页帧。这个过程的实现是非常巧妙的，我们让一个PageTable的FrameTracker都属于该PageTable，而每个PageTable又最终属于一个进程，只需要让进程结束时释放页表，而页表释放时释放自身的FrameTracker，则就可以实现进程释放的时候其所占用的所有物理页帧都被释放掉。



