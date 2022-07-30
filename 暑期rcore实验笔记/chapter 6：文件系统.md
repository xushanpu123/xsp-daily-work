# chapter 6：文件系统

## 实验目的

本章我们将实现一个简单的文件系统 – easyfs，能够对 **持久存储设备** (Persistent Storage) I/O 资源进行管理。具体来说就是：

1、支持以文件为单位来组织磁盘中的一系列数据信息。

2、支持以文件名来访问文件，并能够根据文件地址来访问到对应的数据。



## 实验过程分析

对文件系统的代码分析本人决定采用自顶向下的方式，先分析需要哪些功能，再考虑为了支撑这一功能需要做哪些底层的支持，直到完成了整体设计。

#### 1、需要提供的系统调用

文件系统部分我们需要对外提供的系统调用接口包括：

```rust
pub fn sys_open(path: *const u8, flags: u32) -> isize
```

这个syscall的含义为根据文件名检索到对应的文件，使当前进程获取对该文件的访问权限，并且用返回的整数文件描述符来作为它的访问标识。



```rust
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize
```

进程通过打开文件时获取的文件描述符fd来将文件内容读或者写到用户的buf数组中（涉及地址映射），读写长度为len，若长度不够，则尽可能读写，返回实际读写长度。



```rust
pub fn sys_close(fd: usize) -> isize
```

关闭进程对文件描述符为fd的文件的访问权限。



#### 2、磁盘、操作系统和进程对文件的读写访问结构与方法

我们假设在磁盘中存放着一种结构，我们只需要找到了这个结构并且把它读入内存来分析，就可以获取这个文件的全部信息，并且可以通过随机访问的方式去访问文件的任意一个位置的内容，这种结构我们假定它叫做DiskInode，DiskInode被组织在外存中一块叫DiskInode表的外存区域中，通过编号就可以唯一访问到，如何利用DiskInode来随机访问文件内容我们后面分析。DiskInode被存放在磁盘上，我们访问它需要先把它放入内存中，DiskInode在内存中的结构我们称为inode，通过Inode我们可以访问到其对应的DiskInode。因此进程想要获取对文件的访问能力实际上就是找到其对应的inode，若DiskInode还没有被放入内存中，则先作为inode缓存到内存中，否则直接返回其缓存在内存中的inode即可。因此，访问同一个DiskInode的进程可能有多个，而由此产生的Inode只会有一个，多进程都指向它，因此Inode使用rust的Arc特性来封装就可以利用好这个特性。而每个进程对文件访问时，都会有一些区别于其它进程的具体特性（例如进程的访问权限，进程当前的读写位置等），所以具体到进程，我们应该同样建立一个结构OSInode，给每一个进程使用，就此，我们可以分析出OSInode，Inode，DiskInode结构的大致结构和需要支持的接口：

```rust
impl DiskInode{
    //读取文件offset处的数据到buf处
     pub fn read_at(&self,
         offset: usize,
         buf: &mut [u8],
         block_device: &Arc<dyn BlockDevice>,
    ) -> usize
    
    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize
}
```

 

```rust
impl Inode{
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V
}
```

这两个函数可以直接通过Inode对其对应的DiskInode调用方法并获取其返回值，例如read_at()和write_at(),利用这个方法我们可以实现利用Inode读写的函数：

```rust
/// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            disk_inode.read_at(offset, buf, &self.block_device)
        })
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
```

而OSInode的结构也比较显然了：

```rust
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}

/// The OS inode inner in 'UPSafeCell'
pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}
```

OSInode包含读写权限，还包含inode指针以及offset，我们注意到读写函数的时候我们并没有给出文件的起点，实际上这个起点位置由各进程对应的OSInode的offset给出。OSInode应该支持的基本方法有：

```rust
impl OSInode {
    /// Construct an OS inode from a inode
    pub fn new(
        readable: bool,
        writable: bool,
        inode: Arc<Inode>,
    ) -> Self
    
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
}
```

显然，这个过程的实现访问了其内部对应的Inode。至此，只要获取了进程文件所对应的OSInode，我们就可以对文件进行读写了。而在进程的syscall中，我们查找OSIode的依据是一个自然数fd，这是因为我们为每个进程维护一个OSInode表，fd是对应的OSInode在这个表中的索引，这个表位于进程控制块中：

```rust
pub struct TaskControlBlock {
    // immutable
    /// Process identifier
    pub pid: PidHandle,
    /// Kernel stack corresponding to PID
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    ......
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
}
```

就此，我们给出了sys_read()和sys_write()的实现：

```rust
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(
            UserBuffer::new(translated_byte_buffer(token, buf, len))
        ) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(
            UserBuffer::new(translated_byte_buffer(token, buf, len))
        ) as isize
    } else {
        -1
    }
}
```



#### 3、通过文件名获取对文件的访问能力

根据以上的描述，我们知道，只要我们把文件的DiskInode最终化为OSInode并存放在进程的fd_table中，并且获取其fd，就可以实现对文件数据的随机访问了，而open()操作实际上就是实现的这个过程。

**目录与目录项**

​		想要实现open()的功能，我们就必须能够获取文件对应的DiskInode编号。这里，我们设计了一个特殊的结构：目录项DirEntry。DirEntry的定义为：

```rust
pub struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_number: u32,
}
```

可以看到，这个结构包括一个文件名name和一个inode_number，顾名思义，目录项中存放了name及其对应的inode编号，因此，我们只需要找到name为给定名称的DirEntry就可以了。然而，文件系统中的DirEntry可能有很多，而且需要可持久化的存储，我们不可能把它们放在内存中，因此，我们把这些DirEntry也存放在了一个文件中，并且将这个文件的DiskInode放在Inode表的0号位置，这个用来放置DirEntry的文件就叫做目录，在我们的实验中，只有一个目录文件，即根目录文件，它的DiskInode编号为0号，我们随时都可以访问到它。而通过遍历这个文件，我们就可以找到name所对应的DirEntry了。



DirEntry包含的方法有：

```rust
//  easy-fs/layout

impl DirEntry {
    /// Create an empty directory entry
    pub fn empty() -> Self
    
    /// Crate a directory entry from name and inode number
    pub fn new(name: &str, inode_number: u32) -> Self 
    /// Serialize into bytes
    pub fn as_bytes(&self) -> &[u8] 
    /// Serialize into mutable bytes
    pub fn as_bytes_mut(&mut self) -> &mut [u8] 
    /// Get name of the entry
    pub fn name(&self) -> &str 
    /// Get inode number of the entry
    pub fn inode_number(&self) -> u32 
}
```

大家可以去对应的目录中查阅其具体实现。至此，我们为Inode结构提供检索对应文件名的Inode编号的方法：

```rust
impl Inode{
	fn find_inode_id(
       	 &self,
       	 name: &str,
         disk_inode: &DiskInode,
   	 ) -> Option<u32> {
        	// assert it is a directory
         assert!(disk_inode.is_dir());
         let file_count = (disk_inode.size as usize) / DIRENT_SZ;
       	 let mut dirent = DirEntry::empty();
         for i in 0..file_count {
         assert_eq!(
                    disk_inode.read_at(
                    DIRENT_SZ * i,
                    dirent.as_bytes_mut(),
                    &self.block_device,
                ),
         DIRENT_SZ,
            );
         if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }
    
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode)
            .map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

}
```

通过在根目录下调用find()方法，我们就可以获取到对应文件的一个Inode了。而根目录的Inode编号已知，因此可以很简单的访问到。



#### 4、文件系统的磁盘访问接口

从这一部分开始，我们聚焦于如何利用DiskInode去随机访问文件信息。

**块设备接口层**

一个文件系统对应一个虚拟设备，而我们对磁盘的访问是以Block为基本单位的，我们通过块的编号去读写整个磁盘块，我们通过虚拟化来实现对磁盘的这种访问模式，其具体的细节在easy-fs-fuse中，我们利用一个linux文件来模拟磁盘区域，但是这里不是我们研究的重点，大家自行查看即可，现在我们只关注虚拟化为我们提供的接口：

```rust

pub trait BlockDevice : Send + Sync + Any {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);                    //将bolck_id号磁盘块数据写入buf
    fn write_block(&self, block_id: usize, buf: &[u8]);						  //将buf数据写入bolck_id号磁盘块
}

```



**块缓存层**

访问磁盘块时，我们有必要把最近访问的磁盘块在内存中缓存起来来减少I/O操作，从而提高系统性能，缓存层就是这样的结构：

```rust
pub struct BlockCache {
    /// cached block data
    cache: [u8; BLOCK_SZ],              
    /// underlying block id
    block_id: usize,
    /// underlying block device
    block_device: Arc<dyn BlockDevice>,
    /// whether the block is dirty
    modified: bool,
}
```

其包含的方法有：

```rust
impl BlockCache {
    /// Load a new BlockCache from disk.
    pub fn new(
        block_id: usize,
        block_device: Arc<dyn BlockDevice>
    ) -> Self {
        let mut cache = [0u8; BLOCK_SZ];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }
    /// Get the address of an offset inside the cached block data
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) } 
    }

    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    pub fn modify<T, V>(&mut self, offset:usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }
}

impl Drop for BlockCache {          //缓存释放时将块数据写回磁盘
    fn drop(&mut self) {
        self.sync()
    }
}

```

至此，对磁盘块的访问便完全转化为了对缓存块的访问。与前面的章节类似，我们设计一个结构来组织它们，并为其提供接口：

```rust
pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}
impl BlockCacheManager{
	pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>>
}
```

再实例化一个BlockCacheManager并提供对外的统一接口：

```rust
lazy_static! {
    /// The global block cache manager
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(
        BlockCacheManager::new()
    );
}

/// Get the block cache corresponding to the given block id and block device
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER.lock().get_block_cache(block_id, block_device)
}

/// Sync all block cache to block device
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
```

至此，我们通过get_block_cache()接口即可获取对应的缓存块并可以利用它暴露出来的接口来访问。



#### 5、文件系统外存布局

easy-fs 磁盘按照块编号从小到大顺序分成 5 个连续区域：

- 第一个区域只包括一个块，它是 **超级块** (Super Block)，用于定位其他连续区域的位置，检查文件系统合法性。
- 第二个区域是一个索引节点位图，长度为若干个块。它记录了索引节点区域中有哪些索引节点已经被分配出去使用了。
- 第三个区域是索引节点区域，长度为若干个块。其中的每个块都存储了若干个索引节点。
- 第四个区域是一个数据块位图，长度为若干个块。它记录了后面的数据块区域中有哪些已经被分配出去使用了。
- 最后的区域则是数据块区域，其中的每个被分配出去的块保存了文件或目录的具体内容。



**superblock**

```rust
#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,                     								//总块数                       
    pub inode_bitmap_blocks: u32,                                           
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

```



**Bitmap位图**

```rust
pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}
```

当然，定义Bitmap最主要的目的还是为其提供分配和回收对应数据块的方法：

```rust
impl Bitmap {
    /// A new bitmap from start block id and number of blocks
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }
    /// Allocate a new block from a block device
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            let pos = get_block_cache(
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            ).lock().modify(0, |bitmap_block: &mut BitmapBlock| {
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_, bits64)| **bits64 != u64::MAX)
                    .map(|(bits64_pos, bits64)| {
                        (bits64_pos, bits64.trailing_ones() as usize)
                    }) {
                    // modify cache
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize)
                } else {
                    None
                }
            });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }
    /// Deallocate a block
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        get_block_cache(
            block_pos + self.start_block_id,
            Arc::clone(block_device)
        ).lock().modify(0, |bitmap_block: &mut BitmapBlock| {
            assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
            bitmap_block[bits64_pos] -= 1u64 << inner_pos;
        });
    }
    /// Get the max number of allocatable blocks
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
```

```rust
pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize>

pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize)
```

通过这两个方法，可以进行Bitmap控制的Block的分配与回收。



**磁盘索引节点DiskInode**

​		DiskInode在磁盘区域中的索引节点区，可以用来获取文件的全部信息：

```rust
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    type_: DiskInodeType,
}


#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}
```

每个文件/目录在磁盘上均以一个 `DiskInode` 的形式存储。其中包含文件/目录的元数据： `size` 表示文件/目录内容的字节数， `type_` 表示索引节点的类型 `DiskInodeType` ，目前仅支持文件 `File` 和目录 `Directory` 两种类型。其余的 `direct/indirect1/indirect2` 都是存储文件内容/目录内容的数据块的索引。

根据前面所说，DiskInode应该具备基本访问文件随机地址的接口：

```rust
impl DiskInode{
     /// Get id of block given inner id
    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;
        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id]
        } else if inner_id < INDIRECT1_BOUND {
            get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else {
            let last = inner_id - INDIRECT1_BOUND;
            let indirect1 = get_block_cache(
                self.indirect2 as usize,
                Arc::clone(block_device)
            )
            .lock()
            .read(0, |indirect2: &IndirectBlock| {
                indirect2[last / INODE_INDIRECT1_COUNT]
            });
            get_block_cache(
                indirect1 as usize,
                Arc::clone(block_device)
            )
            .lock()
            .read(0, |indirect1: &IndirectBlock| {
                indirect1[last % INODE_INDIRECT1_COUNT]
            })
        }
    }
}
```

通过这个接口，可以获取文件逻辑块对应的物理块的地址，然后用类似于虚拟页表中的转化方法就可以访问到对应的物理地址了，下面看read_at和write_at的实现：

```rust
	pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = (offset + buf.len()).min(self.size as usize);
        if start >= end {
            return 0;
        }
        let mut start_block = start / BLOCK_SZ;
        let mut read_size = 0usize;
        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            end_current_block = end_current_block.min(end);
            // read and update read size
            let block_read_size = end_current_block - start;
            let dst = &mut buf[read_size..read_size + block_read_size];
            get_block_cache(
                self.get_block_id(start_block as u32, block_device) as usize,
                Arc::clone(block_device),
            )
            .lock()
            .read(0, |data_block: &DataBlock| {
                let src = &data_block[start % BLOCK_SZ..start % BLOCK_SZ + block_read_size];
                dst.copy_from_slice(src);
            });
            read_size += block_read_size;
            // move to next block
            if end_current_block == end { break; }
            start_block += 1;
            start = end_current_block;
        }
        read_size
    }

    /// Write data into current disk inode
    /// size must be adjusted properly beforehand
    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = (offset + buf.len()).min(self.size as usize);
        assert!(start <= end);
        let mut start_block = start / BLOCK_SZ;
        let mut write_size = 0usize;
        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            end_current_block = end_current_block.min(end);
            // write and update write size
            let block_write_size = end_current_block - start;
            get_block_cache(
                self.get_block_id(start_block as u32, block_device) as usize,
                Arc::clone(block_device)
            )
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                let src = &buf[write_size..write_size + block_write_size];
                let dst = &mut data_block[start % BLOCK_SZ..start % BLOCK_SZ + block_write_size];
                dst.copy_from_slice(src);
            });
            write_size += block_write_size;
            // move to next block
            if end_current_block == end { break; }
            start_block += 1;
            start = end_current_block;
        }
        write_size
    }
```

不过在write_at()之前需要对比缓冲区和文件容量的大小，如果容量不够，需要调用扩容函数increase_size()：

```rust
pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        let mut current_blocks = self.data_blocks();
        self.size = new_size;
        let mut total_blocks = self.data_blocks();
        let mut new_blocks = new_blocks.into_iter();
        // fill direct
        while current_blocks < total_blocks.min(INODE_DIRECT_COUNT as u32) {
            self.direct[current_blocks as usize] = new_blocks.next().unwrap();
            current_blocks += 1;
        }
        // alloc indirect1
        if total_blocks > INODE_DIRECT_COUNT as u32{
            if current_blocks == INODE_DIRECT_COUNT as u32 {
                self.indirect1 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_DIRECT_COUNT as u32;
            total_blocks -= INODE_DIRECT_COUNT as u32;
        } else {
            return;
        }
        // fill indirect1
        get_block_cache(
            self.indirect1 as usize,
            Arc::clone(block_device)
        )
        .lock()
        .modify(0, |indirect1: &mut IndirectBlock| {
            while current_blocks < total_blocks.min(INODE_INDIRECT1_COUNT as u32) {
                indirect1[current_blocks as usize] = new_blocks.next().unwrap();
                current_blocks += 1;
            }
        });
        // alloc indirect2
        if total_blocks > INODE_INDIRECT1_COUNT as u32 {
            if current_blocks == INODE_INDIRECT1_COUNT as u32 {
                self.indirect2 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_INDIRECT1_COUNT as u32;
            total_blocks -= INODE_INDIRECT1_COUNT as u32;
        } else {
            return;
        }
        // fill indirect2 from (a0, b0) -> (a1, b1)
        let mut a0 = current_blocks as usize / INODE_INDIRECT1_COUNT;
        let mut b0 = current_blocks as usize % INODE_INDIRECT1_COUNT;
        let a1 = total_blocks as usize / INODE_INDIRECT1_COUNT;
        let b1 = total_blocks as usize % INODE_INDIRECT1_COUNT;
        // alloc low-level indirect1
        get_block_cache(
            self.indirect2 as usize,
            Arc::clone(block_device)
        )
        .lock()
        .modify(0, |indirect2: &mut IndirectBlock| {
            while (a0 < a1) || (a0 == a1 && b0 < b1) {
                if b0 == 0 {
                    indirect2[a0] = new_blocks.next().unwrap();
                }
                // fill current
                get_block_cache(
                    indirect2[a0] as usize,
                    Arc::clone(block_device)
                )
                .lock()
                .modify(0, |indirect1: &mut IndirectBlock| {
                    indirect1[b0] = new_blocks.next().unwrap();
                });
                // move to next
                b0 += 1;
                if b0 == INODE_INDIRECT1_COUNT {
                    b0 = 0;
                    a0 += 1;
                }
            }
        });
    }
```

与之对应的还有clear_size():

```rust
 /// Clear size to zero and return blocks that should be deallocated
    /// and clear the block contents to zero later
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        let mut data_blocks = self.data_blocks() as usize;
        self.size = 0;
        let mut current_blocks = 0usize;
        // direct
        while current_blocks < data_blocks.min(INODE_DIRECT_COUNT) {
            v.push(self.direct[current_blocks]);
            self.direct[current_blocks] = 0;
            current_blocks += 1;
        }
        // indirect1 block
        if data_blocks > INODE_DIRECT_COUNT {
            v.push(self.indirect1);
            data_blocks -= INODE_DIRECT_COUNT;
            current_blocks = 0;
        } else {
            return v;
        }
        // indirect1
        get_block_cache(
            self.indirect1 as usize,
            Arc::clone(block_device),
        )
        .lock()
        .modify(0, |indirect1: &mut IndirectBlock| {
            while current_blocks < data_blocks.min(INODE_INDIRECT1_COUNT) {
                v.push(indirect1[current_blocks]);
                //indirect1[current_blocks] = 0;
                current_blocks += 1;
            }
        });
        self.indirect1 = 0;
        // indirect2 block
        if data_blocks > INODE_INDIRECT1_COUNT {
            v.push(self.indirect2);
            data_blocks -= INODE_INDIRECT1_COUNT;
        } else {
            return v;
        }
        // indirect2
        assert!(data_blocks <= INODE_INDIRECT2_COUNT);
        let a1 = data_blocks / INODE_INDIRECT1_COUNT;
        let b1 = data_blocks % INODE_INDIRECT1_COUNT;
        get_block_cache(
            self.indirect2 as usize,
            Arc::clone(block_device),
        )
        .lock()
        .modify(0, |indirect2: &mut IndirectBlock| {
            // full indirect1 blocks
            for i in 0..a1 {
                v.push(indirect2[i]);
                get_block_cache(
                    indirect2[i] as usize,
                    Arc::clone(block_device),
                )
                .lock()
                .modify(0, |indirect1: &mut IndirectBlock| {
                    for j in 0..INODE_INDIRECT1_COUNT {
                        v.push(indirect1[j]);
                        //indirect1[j] = 0;
                    }
                });
                //indirect2[i] = 0;
            }
            // last indirect1 block
            if b1 > 0 {
                v.push(indirect2[a1]);
                get_block_cache(
                    indirect2[a1] as usize,
                    Arc::clone(block_device),
                )
                .lock()
                .modify(0, |indirect1: &mut IndirectBlock| {
                    for j in 0..b1 {
                        v.push(indirect1[j]);
                        //indirect1[j] = 0;
                    }
                });
                //indirect2[a1] = 0;
            }
        });
        self.indirect2 = 0;
        v
    }
```



#### 6、用户read()和write()整体逻辑整理

**从OSInode到读取文件数据**

  以read()为例，read()的接口为：

```rust
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize
```

 它需要获取文件的OSInode，然后利用OSInode的接口read()来读取，而OSInode是借助其内部封装的Inode来read的：

```rust
impl File for OSInode {
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
}
```

我们继续追本溯源，可以看到Inode的read_at()是通过DiskInode的read来实现的：

```rust
pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            disk_inode.read_at(offset, buf, &self.block_device)
        })
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
```

最后，我们上面提到了具体利用DiskInode去读写文件的方法，至此，完成闭环。

**获取OSInode的方法**

   另一条主线是把文件对应的OSInode放入到进程的fd_table中，参数为文件的path，这里使用系统调用open():

```rust
pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(
        path.as_str(),
        OpenFlags::from_bits(flags).unwrap()
    ) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}
```

这里的主体函数是open_file()

```rust
/// Open a file by path
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            // clear size
            inode.clear();
            Some(Arc::new(OSInode::new(
                readable,
                writable,
                inode,
            )))
        } else {
            // create file
            ROOT_INODE.create(name)
                .map(|inode| {
                    Arc::new(OSInode::new(
                        readable,
                        writable,
                        inode,
                    ))
                })
        }
    } else {
        ROOT_INODE.find(name)
            .map(|inode| {
                if flags.contains(OpenFlags::TRUNC) {
                    inode.clear();
                }
                Arc::new(OSInode::new(
                    readable,
                    writable,
                    inode
                ))
            })
    }
}
```

主体函数为根节点下的find()和create()，这里先看find这条路线：

```rust
pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode)
            .map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
```

```rust
 /// Find inode under a disk inode by name
    fn find_inode_id(
        &self,
        name: &str,
        disk_inode: &DiskInode,
    ) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(
                    DIRENT_SZ * i,
                    dirent.as_bytes_mut(),
                    &self.block_device,
                ),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }
```

我们最终进入到了DiskInode中去寻找name对应的目录项，从而获取文件对应的Inode编号。



**创建文件的方法**

​        刚才提到了在inode代表的目录文件下面创建一个新文件的接口create()，创建一个文件，自然需要在DiskInode表中新分配一个DiskInode，获取其编号，然后创建它的目录项，使该目录项指向新分配的DiskInode，然后还需要在目录文件下面插入该目录项：

```rust
pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self.modify_disk_inode(|root_inode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        }).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) 
            = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(
            new_inode_block_id as usize,
            Arc::clone(&self.block_device)
        ).lock().modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
            new_inode.initialize(DiskInodeType::File);
        });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
```

#### 7、对文件系统的最外层封装

定义一个文件系统，实际上只需要给出一个虚拟设备名以及其基本磁盘布局就可以了：

```rust
pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    inode_area_start_block: u32,
    data_area_start_block: u32,
}

```

通过我们定义的文件系统接口以及设备访问的接口，我们可以将其进行进一步的封装处理：

```rust
impl EasyFileSystem {
    /// Create a filesystem from a block device
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        // calculate block size of areas & create bitmaps
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        let inode_num = inode_bitmap.maximum();
        let inode_area_blocks =
            ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        let data_bitmap = Bitmap::new(
            (1 + inode_bitmap_blocks + inode_area_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };
        // clear all blocks
        for i in 0..total_blocks {
            get_block_cache(
                i as usize,
                Arc::clone(&block_device)
            )
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                for byte in data_block.iter_mut() { *byte = 0; }
            });
        }
        // initialize SuperBlock
        get_block_cache(0, Arc::clone(&block_device))
        .lock()
        .modify(0, |super_block: &mut SuperBlock| {
            super_block.initialize(
                total_blocks,
                inode_bitmap_blocks,
                inode_area_blocks,
                data_bitmap_blocks,
                data_area_blocks,
            );
        });
        // write back immediately
        // create a inode for root node "/"
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(
            root_inode_block_id as usize,
            Arc::clone(&block_device)
        )
        .lock()
        .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
            disk_inode.initialize(DiskInodeType::Directory);
        });
        block_cache_sync_all();
        Arc::new(Mutex::new(efs))
    }
    /// Open a block device as a filesystem
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        // read SuperBlock
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(
                        1,
                        super_block.inode_bitmap_blocks as usize
                    ),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }
    /// Get the root inode of the filesystem
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        // acquire efs lock temporarily
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        // release efs lock
        Inode::new(
            block_id,
            block_offset,
            Arc::clone(efs),
            block_device,
        )
    }
    /// Get inode by id
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32;
        let block_id = self.inode_area_start_block + inode_id / inodes_per_block;
        (block_id, (inode_id % inodes_per_block) as usize * inode_size)
    }
    /// Get data block by id
    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }
    /// Allocate a new inode
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }
    /// Allocate a data block
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }
    /// Deallocate a data block
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(
            block_id as usize,
            Arc::clone(&self.block_device)
        )
        .lock()
        .modify(0, |data_block: &mut DataBlock| {
            data_block.iter_mut().for_each(|p| { *p = 0; })
        });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize
        )
    }
}
```



#### 8、将应用打包到文件系统中

这个过程实际上就是在宿主OS上创建一个f文件，将其封装为块设备并将read_block()和write_block()的接口暴露给OS，我们以该f为虚拟设备构建一个EasyFileSystem结构，此后，我们遍历每一个应用的文件名，用它作为name来创建文件，再在该文件中写入对应的程序数据。这样，在系统视角下，我们就可以通过文件名来访问所有的应用，而不必像之前的实验那样把所有的数据都硬编码在内核的数据段中。

```rust
fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
}

/// Pack a directory into a easy-fs disk image
fn easy_fs_pack() -> std::io::Result<()> {
    let matches = App::new("EasyFileSystem packer")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .help("Executable target dir(with backslash)"),
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len((BLOCK_NUM * BLOCK_SZ) as u64).unwrap();
        f
    })));
    let efs = EasyFileSystem::create(block_file.clone(), BLOCK_NUM as u32, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    let apps: Vec<_> = read_dir(src_path)
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    for app in apps {
        // load app data (elf) from host file system
        let mut host_file = File::open(format!("{}{}", target_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in easy-fs
        let inode = root_inode.create(app.as_str()).unwrap();
        // write data to easy-fs
        inode.write_at(0, all_data.as_slice());
    }
    // list apps
    for app in root_inode.ls() {
        println!("{}", app);
    }
    Ok(())
}
```

至此，除了少数函数，chapter 6 的基本框架已经介绍完毕。



## Lab4 编程作业

### 任务一：硬链接

硬链接要求两个不同的目录项指向同一个文件，在我们的文件系统中也就是两个不同名称目录项指向同一个磁盘块。

本节要求实现三个系统调用 `sys_linkat、sys_unlinkat、sys_stat` 。

**linkat**：

> - syscall ID: 37
>
> - 功能：创建一个文件的一个硬链接， [linkat标准接口](https://linux.die.net/man/2/linkat) 。
>
> - Ｃ接口： `int linkat(int olddirfd, char* oldpath, int newdirfd, char* newpath, unsigned int flags)`
>
> - Rust 接口： `fn linkat(olddirfd: i32, oldpath: *const u8, newdirfd: i32, newpath: *const u8, flags: u32) -> i32`
>
> - - 参数：
>
>     olddirfd，newdirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。 flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。 oldpath：原有文件路径 newpath: 新的链接文件路径。
>
> - - 说明：
>
>     为了方便，不考虑新文件路径已经存在的情况（属于未定义行为），除非链接同名文件。 返回值：如果出现了错误则返回 -1，否则返回 0。
>
> - - 可能的错误
>
>     链接同名文件。

**unlinkat**:

> - syscall ID: 35
>
> - 功能：取消一个文件路径到文件的链接, [unlinkat标准接口](https://linux.die.net/man/2/unlinkat) 。
>
> - Ｃ接口： `int unlinkat(int dirfd, char* path, unsigned int flags)`
>
> - Rust 接口： `fn unlinkat(dirfd: i32, path: *const u8, flags: u32) -> i32`
>
> - - 参数：
>
>     dirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。 flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。 path：文件路径。
>
> - - 说明：
>
>     注意考虑使用 unlink 彻底删除文件的情况，此时需要回收inode以及它对应的数据块。
>
> - 返回值：如果出现了错误则返回 -1，否则返回 0。
>
> - - 可能的错误
>
>     文件不存在。



#### 实现系统调用sys_linkat()

 sys_linkat()的接口为：

```rust
pub fn sys_linkat(oldpath: *const u8, newpath: *const u8) -> isize
```

语义为创建一个新的目录项，name为newpath，将其链接到oldpath对应的DiskInode上。

**基本思路**

先通过oldpath检索到对应的inode编号n，将对应的DiskInode的链接计数加1，然后创建一个新的目录项，其name为newpath，索引节点编号为n，即可完成任务。

**具体过程**

```rust
//     os/src/syscall/fs.rs
pub fn sys_linkat(oldpath: *const u8, newpath: *const u8) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let oldpath = translated_str(token, oldpath);
    let newpath = translated_str(token, newpath);
    if oldpath == newpath {
       return -1;
    }
    if link_file(&oldpath, &newpath).is_none() {
        return -1;
    }
    0
}
```



```rust
//   os/src/fs/inode.rs

pub fn link_file(oldname: &str, newname: &str) -> Option<()> {
    if oldname == newname {
        return None;
    }
    ROOT_INODE.link(oldname, newname)
}

//   easy-fs/src/vfs.rs
impl Inode{
    pub fn link(&self, oldname: &str, newname: &str) -> Option<()> {
        let mut fs = self.fs.lock();
        let old_inode_id =
            self.read_disk_inode(|root_inode| self.find_inode_id(oldname, root_inode));
        if old_inode_id.is_none() {
            return None;
        }
        let (block_id, block_offset) = fs.get_disk_inode_pos(old_inode_id.unwrap());
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(block_offset, |n: &mut DiskInode| n.nlink += 1);
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            self.increase_size(new_size as u32, root_inode, &mut fs);
            let dirent = DirEntry::new(newname, old_inode_id.unwrap());
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        block_cache_sync_all();
        Some(())
    }
}
```



#### 实现系统调用sys_unlinkat()

sys_unlinkat()的接口为：

```rust
pub fn sys_unlinkat(path: *const u8) -> isize
```

语义为将path对应的硬链接删除。

**基本思路**

检索到path所对应的DiskInode，将其所对应的链接计数减1。再从根目录中把名为path的目录项移除：

**具体实现**

```rust
//    os/syscall/fs.rs
pub fn sys_unlinkat(path: *const u8) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if unlink_file(&path).is_none() {
        return -1;
    }
    0
}
```



```rust
//   os/src/fs/inode.rs
pub fn unlink_file(name: &str) -> Option<()> {
    ROOT_INODE.unlink(name)
}

//   easy-fs/src/vfs.rs
impl Inode{
	pub fn unlink(&self, name: &str) -> Option<()> {
        let mut fs = self.fs.lock();
        let mut inid: Option<u32> = None;
        let mut v: Vec<DirEntry> = Vec::new();
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    root_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                if dirent.name() != name {
                    v.push(dirent);
                } else {
                    inid = Some(dirent.inode_number());
                }
            }
        });
        self.modify_disk_inode(|root_inode| {
            let size = root_inode.size;
            let data_blocks_dealloc = root_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
            self.increase_size((v.len() * DIRENT_SZ) as u32, root_inode, &mut fs);
            for (i, dirent) in v.iter().enumerate() {
                root_inode.write_at(i * DIRENT_SZ, dirent.as_bytes(), &self.block_device);
            }
        });
        if inid.is_none() {
            return None;
        }
        let (block_id, block_offset) = fs.get_disk_inode_pos(inid.unwrap());
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(block_offset, |n: &mut DiskInode| {
                n.nlink -= 1;
                if n.nlink == 0 {
                    let size = n.size;
                    let data_blocks_dealloc = n.clear_size(&self.block_device);
                    assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
                    for data_block in data_blocks_dealloc.into_iter() {
                        fs.dealloc_data(data_block);
                    }
                }
             });
        block_cache_sync_all();
        Some(())
    }
}
```



### 任务二：获取文件状态

**fstat**:

> - syscall ID: 80
>
> - 功能：获取文件状态。
>
> - Ｃ接口： `int fstat(int fd, struct Stat* st)`
>
> - Rust 接口： `fn fstat(fd: i32, st: *mut Stat) -> i32`
>
> - - 参数：
>
>     fd: 文件描述符 st: 文件状态结构体 `#[repr(C)] #[derive(Debug)] pub struct Stat {    /// 文件所在磁盘驱动器号，该实验中写死为 0 即可    pub dev: u64,    /// inode 文件所在 inode 编号    pub ino: u64,    /// 文件类型    pub mode: StatMode,    /// 硬链接数量，初始为1    pub nlink: u32,    /// 无需考虑，为了兼容性设计    pad: [u64; 7], } /// StatMode 定义： bitflags! {    pub struct StatMode: u32 {        const NULL  = 0;        /// directory        const DIR   = 0o040000;        /// ordinary regular file        const FILE  = 0o100000;    } } `



#### 实验实现

```rust
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    let va = VirtAddr::from(st as usize);
    let pa = usize::from(PhysAddr::from(
        inner.memory_set.translate(va.floor()).unwrap().ppn(),
    ));
    let st = (pa + va.page_offset()) as *mut Stat;
    let st = unsafe { &mut *st };
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        drop(inner);
        file.stat(st)
    } else {
        -1
    }
}
```

