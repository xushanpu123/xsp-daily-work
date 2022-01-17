# chapter6实验笔记

## chapter6的核心数据结构（从顶层到底层分析）

### 核心数据结构的组成元素（内存中的数据结构）

```rust
pub struct EasyFileSystem {//约定文件系统的第0块为superblock
    pub block_device: Arc<dyn BlockDevice>,//代表了文件系统挂载的块设备
    pub inode_bitmap: Bitmap,//管理inode数据块的bitmap
    pub data_bitmap: Bitmap,//管理data数据块的bitmap
    inode_area_start_block: u32,//inode区域的起始块
    data_area_start_block: u32,//data区域的起始块
}
```

**学习到的rust语法知识**：

```rust
pub block_device: Arc<dyn BlockDevice>
//表示所有实现了BlockDevice trait类型的struct
```



```rust
pub trait BlockDevice : Send + Sync + Any {
    
	fn read_block(&self, block_id: usize, buf: &mut [u8]);
//读取设备的第block_id个块并写入内存buf中
    
    fn write_block(&self, block_id: usize, buf: &[u8]);
//将buf中的内容写入设备的第block_id个块

}

```



```rust
pub struct Bitmap {
	start_block_id: usize,  //bitmap所占磁盘块的起始块
    blocks: usize,			//bitmap所占磁盘块的个数

}
```

上述结构为位于内存中定义的数据结构，一个Bitmap结构可以指向磁盘中连续的Bitmap.blocks个块，这些blocks中存放着的二进制数每一位都代表着一个磁盘块的占用情况，配合上Bitmap所表示的基地址（inode_area_start_block、 data_area_start_block），就可以定位表示范围内的每一个磁盘块。

在外存看来，自身存储的是一系列连续的二进制数串，所有支持trait BlockDevice的设备都可以依块访问，即将外存中的第block_id个块的所有数据与内存中的某个区域buf进行整块的数据交换。但是在文件系统看来，外存中不同位置存放的数据都具有一定的结构，由EasyFileSystem的结构可以看出文件系统将位于不同位置的块分为SuperBlock、inode_bitmap_block、inode_block、data_bitmap_block、data_block，当对文件系统进行操作时，内存读取对应位置的块并按照文件系统约定好的逻辑对各个部分的块数据进行解析，我们用来解析从外存中取出的块数据的数据结构即为外存中的数据结构。

### 外存中的数据结构（存储在磁盘中的数据）

```rust
pub struct SuperBlock { //超级块，放置在block_id=0的块中，取出block 0后解析成该数据结构
	magic: u32,         //魔数，验证是否是合法的elf文件
	pub inode_bitmap_blocks: u32,  
	pub inode_area_blocks: u32,
	pub data_bitmap_blocks: u32,
	pub data_area_blocks: u32,
}
```

**外存中的SuperBlock与内存中的EasyFileSystem的关系：**

直接控制整个文件系统的数据结构为EasyFileSystem，为block_device装入文件系统时，先在内存根据期望的文件系统布局在内存构建EasyFileSystem结构，再据此构建superblock，最后在实际设备的0号block装入此superblock并在特定的block依次分割出inode_bitmap、inode_area、data_bitmap、data_area区域；从block_device中获取文件系统控制权时，就从该blck_device中取出第0块的block，按照superblock数据结构来解析，即可构建出EasyFileSystem结构，进而对文件系统进行控制和访问。

整个磁盘空间的结构为：

**|**  super block **|**    inode_bitmap   **|**   inode_area  **|**   data_bitmap  **|**  data_area   **|**

超级块中记录了每个区域占用block的数量（super block默认占用第0块），EasyFileSystem中记录了两种bitmap的起始地址和比特数，两种area占用block的数量，因此这两种数据结构可以相互转换。



```rust
pub struct DiskInode { 
//文件在磁盘上的存储方式，根据DiskInode可以获取该文件在磁盘上的所有信息
    pub size: u32,        								//文件大小
    pub direct: [u32; INODE_DIRECT_COUNT],              //直接索引
    pub indirect1: u32,                                 //一级间接索引
    pub indirect2: u32,                                 //二级间接索引
    type_: DiskInodeType,                               //文件类型（普通文件or目录？）
}

pub enum DiskInodeType {
    File,
    Directory,
}
```

DiskInode存储在inode_area区域中，一个标号对应一个拟定好的位置，inode_bitmap区域的比特位对应了这些既定位置的使用情况，某个文件一旦被初次创建，就会被分配一个DiskInode并把自身的信息写入该DiskInode，此后需要对这个进行的任何操作都需要访问到其对应的DuskInode从而访问其本体。而由于DiskInode在磁盘中的位置跟其标号一一对应，因此只需要知道其标号，就可以找到对应的DiskInode了。



```rust
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}
```

内存中的一个Inode与磁盘中的一个DiskInode一一对应，只不过隐藏了其内部细节。



```rust
pub struct DirEntry {                                      //目录项结构，所有目录文件的内容均解析成目录项
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_number: u32,
}
```

上述数据结构均为存储在外存中的数据结构，其作用为提供给文件系统一个解析磁盘数据的方法。



### 磁盘块缓存数据结构

```rust
pub struct BlockCache {   //在内存中维护一个内存块来缓存一个磁盘block的数据结构，在上层看来等同于直接使用对应磁盘块
    cache: [u8; BLOCK_SZ],           							//缓存磁盘块的实际内存空间
    block_id: usize,                 							//缓存的磁盘块号
    block_device: Arc<dyn BlockDevice>,							//缓存的磁盘块所属的外设
    modified: bool,												//缓存区域是否被修改过，决定了该缓冲区被置换出时是否写入磁盘
}
```



```rust
pub struct BlockCacheManager {  //管理全部BlockChche的数据结构，在上层看来等同于直接使用文件系统中覆盖的整个磁盘
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}
```



## 核心数据结构的主要方法（自顶向下）

我们的目标是建立用户可用的文件目录系统，即

1、在根目录下查找文件名为 name:&str 的文件的Inode；

2、列举根目录下的所有文件的文件名；

3、在根目录下创建文件名为 name:&str 的文件；

4、清空文件名为 name:&str 的文件；

5、读写文件名为 name:&str 的文件。

这一切的前提为找到根目录所对应的Inode，然后为结构体Inode赋予2、3、4、5要求的方法。

```rust
impl EasyFileSystem {
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode  {
        let block_device = Arc::clone(&efs.lock().block_device);
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        Inode::new(
            block_id,
            block_offset,
            Arc::clone(efs),
            block_device,
        )
    }
}
}
//返回efs实体的根目录的Inode,基本思路为：root_DiskInode存放在第0号DiskInode处，因此需要一个方法get_disk_inode_pos()来获取inode_id号DiskInode所在的block_id和block_offset，还需要构建一个Inode::new()方法：

impl Inode {
    /// We should not acquire efs lock here.
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
}
```

**语法知识：**

```rust
let block_device = Arc::clone(&efs.lock().block_device);
//efs是Arc型数据的借用，如果直接使用efs，则efs.lock()后的数据在释放前其它程序都不能访问efs，但是使用clone后，block_device是efs中数据的一个clone，因此可以独立于efs使用。

```

如果获取了根目录的Inode，则根据文件名索引的步骤为:

```rust
//从根目录中找到对应文件名对应的DiskInode，遍历该DiskInode的内容，找到name相同的目录项，读取其inode_id，再根据inode_id找到对应的inode信息从而构建对应的Inode.
impl Inode {
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode|{
            self.find_inode_id(name,disk_inode)
            .map(|inode_id|{
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
     fn find_inode_id(     //根据文件名找到对应的inode_id
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
}
      }
    
```

此时，为了实现进一步的功能，我们必须实现如下接口：

```rust
EasyFileSystem::get_disk_inode_pos(inode_id:Option<u32>)->(usize,usize);
DirEntry::empty();
DiskInode::is_dir();
pub fn read_at(
		&self,
		offset: usize,
		buf: &mut [u8],
		block_device: &Arc<dyn BlockDevice>,
	) -> usize 
```

但是我们设计顶层的时候，可以先假设这些接口已经实现了，等我们完成了整个顶层设计的时候，再去考虑下层的数据结构需要使用哪些接口，再一一设计即可。

**语法知识：**

```rust
self.find_inode_id(name,disk_inode)
            .map(|inode_id|{
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })

//.map(f)方法：参数为闭包，即把调用者当成是参数，执行并返回闭包f的结果。
```

如果获取了根目录的Inode，则列出根目录下的所有的文件名的步骤为：

```rust
//找到对应的DiskInode,再找到DiskInode中指向的所有磁盘块，按顺序读出所有磁盘块中的目录项的name即可。
impl Inode{
    pub fn ls(&self) -> Vec<String>{
        self.read_disk_inode(|disk_inode|{
            
        })
        
    }
}
```



```rust
impl EasyFileSystem {          
        pub fn create(  //在block_device中装入新的EasyFileSystem，主要包括根据参数创建EasyFileSystem结构，构建superblock装入block 0，清空位图（实际操作时归零了所有磁盘块，但是个人认为清空位图部分即可），创建空的根目录
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>>
       
        pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> //读取superblock,获取文件系统布局，返回efs实例
        
        pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) //根据inode_id获取inode所在的磁盘块号和块内偏移（即块内第几个inode）
    
        pub fn get_data_block_id(&self, data_block_id: u32) -> u32      //根据data_block_id获取data所在的磁盘块id
        
     	pub fn alloc_inode(&mut self) -> u32                            //分配一个inode
    
        pub fn alloc_data(&mut self) -> u32                             //分配一个数据块
    
   		pub fn dealloc_data(&mut self, block_id: u32)                   //回收一个数据块
}
```







```rust
impl BlockCache {
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    pub fn modify<T, V>(&mut self, offset:usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }
}
//核心代码分析：该代码是获取某个blockchache（对应某个磁盘块）偏移量为offset处的指针，以&T或&mut T类型传入作为闭包f的参数，而f的返回值为V类型，结果为f的运行结果。
```



例如一段代码：

```rust
get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                    })
//该段代码的含义为：令indirect_block = &get_block_cache(self.indirect1 as usize, Arc::clone(block_device)).lock().cache[0],返回indirect_block[inner_id - INODE_DIRECT_COUNT]的值。
```