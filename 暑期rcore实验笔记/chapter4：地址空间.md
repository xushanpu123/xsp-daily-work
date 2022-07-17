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



**在三级页表结构中访问和创建虚拟页的页表项**

第一个相关功能函数为 ：

```rust
 fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry>
```

它能够返回vpn所对应的PTE,若在查询过程中发现其一级页号或二级页号在对应的页表中的页表项并不valid（该上级索引还没有被分配下级页表），则为其分配下级页表以便得到对应的PTE,其基本实现如下：

```rust
impl PageTable{
fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let mut idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter_mut().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
}
```



另一个类似的函数为：

```rust
fn find_pte(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> 
```

其实现为：

```rust
impl PageTable{
fn find_pte(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
}
```

实际上它的含义同样为查找vpn对应的PTE，但是如果该PTE还没有在页表中创建，则返回None。



配合以下两个函数，我们可以不通过MMU而手动查询页表内容：

```rust
impl PageTable{
pub fn from_token(satp: usize) -> Self {          
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).copied()
    }
}
```



实现了这些功能后，我们提供了两个很重要的函数：

```rust
//为页表的虚拟页vpn增加一个对应的页表项，物理页号为ppn，标记为为flags。 
pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

//释放掉vpn对应的页表项
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
```





最后，提供了一个辅助函数用来获取页表的根页表地址：

```rust
pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
```

至此，页表相关的结构和方法均已经实现，我们对能被外层调用的接口做一个总结：

```rust
impl PageTable {
     pub fn new() -> Self；        					 //创建空页表
     pub fn from_token(satp: usize) -> Self；         //创建根目录地址为satp的空页表
     pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags)；
     pub fn unmap(&mut self, vpn: VirtPageNum)；
     pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry>  //获取页表项的copy
     pub fn token(&self) -> usize                     //获取页表的satp
}
```



#### 2、逻辑段与地址空间的构建

**逻辑段的结构与方法**

```rust
/// map area structure, controls a contiguous piece of virtual memory
pub struct MapArea {
    vpn_range: VPNRange,                                     //迭代器，元素为所有的虚拟页            
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,        //记录映射关系
    map_type: MapType,                                       
    map_perm: MapPermission,                                 //逻辑段的访问权限
}
```

逻辑段代表了一段连续的虚拟页所构成的虚拟地址空间，其中：

```rust
pub enum MapType {
    Identical,                                       //表示对等映射
    Framed,                                          //表示对于每个虚拟页面都需要映射到一个新分配的物理页帧
}

bitflags! {
    pub struct MapPermission: u8 {                  //可以看到与PTEflags是同样的结构
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

```

MapArea与页表映射有关的方法为：

```rust
impl MapArea {
    
//创建一个新的MapArea
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }
    
//为MapArea中的一个虚拟页映射物理页，若为恒等映射，则直接映射到vpn同号的ppn，否则重新分配一个物理页帧使之映射。映射完成后需要在页表中也完成映射操作。
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }
    #[allow(unused)]

//解除映射
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        #[allow(clippy::single_match)]
        match self.map_type {
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }
    
//完成对MapArea中所有虚拟页的映射
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

//解除对MapArea中所有虚拟页的映射
    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
}
```



在此基础上，可以实现在MapArea中存入数组数据的方法：

```rust
impl MapArea{
/// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}
```

该函数将数据放入范围内的每个页中，一个页存满了则进入下一个页存放，直到全部数据存放完毕或者MapArea中的虚拟页全部被用完。

至此，我们可以以此为基础完成对进程地址空间的定义。



**地址空间的结构与方法**

地址空间是与进程一一对应的，其结构为：

```rust
/// memory set structure, controls virtual-memory space
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}
```

它包含一个页表和一系列逻辑段，其基本方法包括：

```rust
pub fn new_bare() -> Self {                                                   //建立一个空的MemorySet
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }
    
pub fn token(&self) -> usize {                                                //获取内置页表的satp
        self.page_table.token()
    }
    
pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {        //获取内置页表对应vpn的PTE
        self.page_table.translate(vpn)
    }
```



```rust
//push方法将data写入map_area中，再将map_area插入MemorySet中
fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);                   //将map_area与page_table绑定并分配物理页帧
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }

//MemorySet中根据参数插入一个没有存放数据的Frame类型的MapArea。
pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }
```



下面介绍一个特殊的函数：

```rust
 /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }
```

我们注意到，这里出现了两个参数TRAMPOLINE和strampoline，我们分别看一下它们的定义：

```rust
//   os/src/config.rs
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
```



```
#     os/src/linker.ld

.text : {
        *(.text.entry)
        . = ALIGN(4K);
        strampoline = .;
        *(.text.trampoline);
        . = ALIGN(4K);
        *(.text .text.*)
    }
    
#    os/src/trap/trap.S

 .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
.......
```

可以看到，strampoline对应了__alltraps所对应的物理页的起始地址。



因此，我们就可以理解该函数的含义了：

```rust
//在内置页表中将虚拟地址TRAMPOLINE所对应的虚拟页映射到__alltraps所对应的页。
fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }
```



接下来，我们来实现内核载入时映射地址空间的函数：

```rust
pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        info!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        info!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        info!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        info!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        info!("mapping .text section");
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        info!("mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        info!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        info!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        info!("mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set
    }
```

可以看到，内核的地址空间对应了多个恒等映射的逻辑段。



下面是载入elf文件的用户态程序的地址空间建立过程：

```rust
 pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }
        // map user stack with U flags
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // guard page
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
        // map TrapContext
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }
```

返回了一个三元组(进程的MemorySet，用户栈地址，入口地址)

这里对几个比较重要的部分进行分析：

```rust
//应用程序在链接的时候就已经确定了每个数据的虚拟地址，在载入系统的时候，数据在程序中的虚拟地址和在虚拟内存中的虚拟地址是一致的，这样才能够保证程序在进入虚拟内存系统后依然可以正常的运行。注意flags的设置，一定是MapPermission::U的，这样才能在用户态下执行。
let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }
```



```rust
// max_end_vpn是用户程序部分所占用的最后一个虚拟页，我们从它后面的一个页开始用作用户栈，需要设置MapPermission::R | MapPermission::W ||MapPermission::U

        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // guard page
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
```



```rust
//   os/src/config.rs
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

// os/mm/memory_set.rs
这一步是预留了TRAMPOLINE前面的一个虚拟页来放置TRAP_CONTEXT
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
```

至此，与MemorySet相关的方法均实现了，我们用activate()函数来使其生效：

```rust
pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);                                    //切换页表
            core::arch::asm!("sfence.vma");                       //刷新TLB
        }
    }
```

实例化一个内核的地址空间：

```rust
lazy_static! {
    /// a memory set instance through lazy_static! managing kernel space
    pub static ref KERNEL_SPACE: Arc<Mutex<MemorySet>> =
        Arc::new(Mutex::new(MemorySet::new_kernel()));
}

pub fn init() {
    .......
    KERNEL_SPACE.lock().activate();
}
```



#### 3、引入地址空间后的task设计

​      在chapter2和chapter3中，任务想要运行必须手动将其载入特定的内存位置中，并且通过设计task的TrapContext来跳转到对应的内存位置去执行。在本章节中，task可以通过虚拟页来访问对应的内容,因此只需要在载入的时候将存放程序的位置作为参数建立task的地址空间就可以了：

```rust
///   os/src/loader.rs

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

```rust
/// task control block structure
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,             //增加地址空间的设置
    pub trap_cx_ppn: PhysPageNum,          //TrapContext的在自身地址空间中的物理页帧，根据恒等映射原则，内核可以用该地址访问相应的Trapcontext
    pub base_size: usize,                  //user_sp位置
}

impl TaskControlBlock{
  	pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    』
```

```rust

impl TaskControlBlock{
//根据elf_data构建TaskControlBlock
	pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);    //获取内核堆栈地址，相关实现在os/src/config.rs
        KERNEL_SPACE.lock().insert_framed_area(                                         //为用户程序的内核堆栈分配页帧，采用Frame模式
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(                                       //初始化TrapContext，以便_switch
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
}
```



值得注意的是，Task_Manager结构中存放TaskControlBlock的结构从本来的全局数组变更为了Vec：

```rust
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

```

这是因为我们给出了动态分配的策略：

```rust
// os/heap_alloc.rs
#[global_allocator]
/// heap allocator instance
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// heap space ([u8; KERNEL_HEAP_SIZE])
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// initiate heap allocator
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}
```

其它关于task切换的方法基本上大同小异了。



**中断进入与中断恢复的设计**

本实验采用双页表设计，这也就意味着用户态和内核态使用不一样的地址空间。当中断发生时，计算机会做以下工作：

- `sstatus` 的 `SPP` 字段会被修改为 CPU 当前的特权级（U/S）。
- `sepc` 会被修改为 Trap 处理完成后默认会执行的下一条指令的地址。
- `scause/stval` 分别会被修改成这次 Trap 的原因以及相关的附加信息。
- CPU 会跳转到 `stvec` 所设置的 Trap 处理入口地址，并将当前特权级设置为 S ，然后从Trap 处理入口地址处开始执行。

注意到，此时我们虽然位于内核态，但是依然处于用户进程的地址空间中，因此，我们需要把Trap处理的入口地址放入到用户地址空间中，即前面提到的：

```rust
pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize){
	memory_set.map_trampoline();
}
```

再设置用户程序中断的入口地址：

```rust
//  os/trap/mod.rs

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_handler() -> ! {
    ......
    trap_return();
}

pub fn trap_return() -> ! {
    set_user_trap_entry();
    ......
}
```

中断处理入口处的代码：

```
_alltraps:
    csrrw sp, sscratch, sp
    # now sp->*TrapContext in user space, sscratch->user stack
    # save other general purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they have been saved in TrapContext
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # read user stack from sscratch and save it in TrapContext
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # load kernel_satp into t0
    ld t0, 34*8(sp)
    # load trap_handler into t1
    ld t1, 36*8(sp)
    # move to kernel_sp
    ld sp, 35*8(sp)
    # switch to kernel space
    csrw satp, t0
    sfence.vma
    # jump to trap_handler
    jr t1
```

保存完现场后，TrapContext中存放了kernel_satp，trap_handler，kernel_sp，在切换到内核堆栈中后，直接做对应的切换即可。这里使用了jr指令，这是一个绝对寻址的指令，可以确保跳转到目标位置。





处理完中断后，会调用trap_return()：

```rust
#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        core::arch::asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}
```

执行上述代码，则会跳转到__restore继续执行，由于内核态和用户态下TRAMPOLINE都是一样的，所以页表切换后，这个执行过程并不会产生任何差错。

下面是__restore部分：

```
__restore:
    # a0: *TrapContext in user space(Constant); a1: user space token
    # switch to user space
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0
    mv sp, a0
    # now sp points to TrapContext in user space, start restoring based on it
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # restore general purpose registers except x0/sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # back to user stack
    ld sp, 2*8(sp)
    sret
```

包含了地址空间恢复和现场恢复。

值得注意的是，switch模块并没有太大的变化，因为switch并不涉及地址空间的切换，当我们修改current_task的时候，__restore函数自然会把地址空间切换到next_task。至此，chapter4的框架部分解析完成。

## 编程练习

### 问题一：重写 sys_get_time 和 sys_task_info

引入虚存机制后，原来内核的 sys_get_time 和 sys_task_info 函数实现就无效了。请你重写这个函数，恢复其正常功能。



#### 需要重写的原因

以sys_get_time为例，它的实现为：

```rust
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}
```

它传入的是一个TimeVal类型的指针，发起系统调用时是在用户态下传入的地址，在内核态下，该地址显然不再是传入的数据的位置，因此，我们只需要找到真实的地址位置即可。因此，我们只需要实现一个当前进程虚拟地址到物理地址的转换方法即可,一种实现方式为：

```rust
pub fn translated_physical_address(token: usize, ptr: *const u8) -> usize{
    let page_table = PageTable::from_token(token);
    let mut va = VirtAddr::from(ptr as usize);
    let ppn = page_table.find_pte(va.floor()).unwrap().ppn();
    super::PhysAddr::from(ppn).0 + va.page_offset()

}
```

由此，可以得到答案方法：

```rust
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    let _us = get_time_us();
    let ts = current_translated_physical_address(_ts as *const u8 ) as *mut TimeVal;
    unsafe {
         *ts = TimeVal {
             sec: _us / 1_000_000,
            usec: _us % 1_000_000,
        };
     }
    0
}
```

```rust
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let _ti =  current_translated_physical_address(ti as *const u8 ) as *mut TaskInfo;
    unsafe{
    *_ti = TaskInfo{
        status:get_current_status(),
        syscall_times:get_syscall_times(),
        time : (get_time_us() - get_current_start_time())/1000

    };
}
    0
}
```



### 问题二：mmap 和 munmap 匿名映射

mmap在 Linux 中主要用于在内存中映射文件， 本次实验简化它的功能，仅用于申请内存。

请实现 mmap 和 munmap 系统调用，mmap 定义如下：

```rust
fn sys_mmap(start: usize, len: usize, port: usize) -> isize
```

- syscall ID：222

- 申请长度为 len 字节的物理内存（不要求实际物理内存位置，可以随便找一块），将其映射到 start 开始的虚存，内存页属性为 port

- - 参数：

    start 需要映射的虚存起始地址，要求按页对齐 len 映射字节长度，可以为 0 port：第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0

- 返回值：执行成功则返回 0，错误返回 -1

- - 说明：

    为了简单，目标虚存区间要求按页对齐，len 可直接按页向上取整，不考虑分配失败时的页回收。

- - 可能的错误：

    start 没有按页大小对齐 port & !0x7 != 0 (port 其余位必须为0) port & 0x7 = 0 (这样的内存无意义) [start, start + len) 中存在已经被映射的页 物理内存不足

munmap 定义如下：

```rust
fn sys_munmap(start: usize, len: usize) -> isize
```

- syscall ID：215

- 取消到 [start, start + len) 虚存的映射

- 参数和返回值请参考 mmap

- - 说明：

    为了简单，参数错误时不考虑内存的恢复和回收。

- - 可能的错误：

    [start, start + len) 中存在未被映射的虚存。

tips:

- 一定要注意 mmap 时的页表项，注意 riscv 页表项的格式与 port 的区别。
- 你增加 PTE_U 了吗？



#### 增加映射函数

映射函数的定义为：

```rust
fn sys_mmap(start: usize, len: usize, port: usize) -> isize
```

具体步骤可以为：

1、构建一个对应的MapArea；

2、将该MapArea push到进程的地址空间中即可。

3、必须进行映射检查，不可重复映射。