# chapter5实验笔记

## chapter4已经实现的核心功能和核心数据结构：

##### TaskManager：任务管理器

1、管理各任务的核心数据结构，主要记录了当前任务current_task和系统中存在各task的TaskContorlBlock（其中包含了每个task的上下文、地址空间、status和调度信息等)；

2、调度和切换功能：选择一个合适的处于Ready的task，将当前正在运行的task切换过去并把当前task的TaskContorlBlock保存在TaskManager中，再切换到选择的task上；



##### 内核和各用户task的地址空间隔离

1、每个task有各自的地址空间，内核也有自己的地址空间，中断时各自的用户地址空间切换到内核的地址空间，返回时从TaskManager中选择对应的task的地址空间并切换过去；

2、用户和内核均可以给自己分配新的物理页帧并入自己的虚拟地址，也可以释放自己的某些虚拟页；

3、构建os的时候可以同时装入多个用户task，让每个task执行不同的二进制文件，占用不同的地址空间并且相互调度。



## chapter5新增



#### 为每个task编号，即pid：

用chapter4类似于分配物理页帧的方法为每个task分配pid，确保当前存在的每个task都拥有唯一的pid，再在TaskControlBlock中存入task所分配的pid，就可以通过pid找到对应的TaskControlBlock了。每个task在new的时候通过pid分配器来分配新的pid，task在被完全回收的时候再将自己所占用的pid返还给pid分配器即可。



#### task具有自我复制的能力：（主要代码位于os/src/task/TaskControlBlock::fork())

引入系统调用fork()，本质上就是将自己的TaskControlBlock再复制一份，但是给新的task不同的地址空间和pid，其具体过程为：复制原task的TaskControlBlock，给这个新的task分配同样多的物理页帧和新的pid，再复制原Task地址空间中的所有数据，则此时新task和原task同一个虚拟地址所对应的数据安全相同，但是数据所存放的实际物理页帧不同。不过新旧TaskContorlBlock存放返回值的a0寄存器值不同，原task赋值为新task的pid，新task赋值为0。



#### task具有执行一个特定path下的文件的功能：（主要代码位于os/src/task/TaskControlBlock::exec())

引入系统调用exec(),具体起作用的步骤为TaskControlBlock::exec()，该操作的基本过程为：先创建一个装入目标可执行文件的新的MemorySet，再将返回值赋值给该TaskControlBlock中对应的各参数（具体参数的获取过程详见代码）



#### task具有了新的状态，退出态exit，并因此引入了系统调用wait()和waitpid()：

task在完成所有功能或被终止时，理应释放掉所有的内存空间，pid（以及在TaskManager中所占有的位置等），但是由于需要退出的task还有一些信息存储在自身资源中（如退出码存放在寄存器中），因此只能先释放一部分资源，等父进程获取了残余信息，再回收退出进程的残余资源，因此，需要引入系统调用wait()和waitpid()，采用不定期查询的方式来检查自己的某个或者子进程是否已经进入了exit态。而对子进程来说，如果发现自己的父进程已经不存在了，就直接退出并且回收所有资源即可。因此，在TaskControlBlock中需要加入children和parent项来指向自己的父进程和子进程（此处使用智能指针来表示指向而不是拥有）



#### task的内核栈KernalStack通过自身的pid唯一定位：

KernalStack被定义为

{

pid: usize

}

因为内核中的空间已经规定好了堆栈空间和pid的对应关系，所以通过pid即可找到自身所对应的堆栈空间。



#### 按文件名创建运行对应文件的进程：

user/src/bin中的各文件build好的二进制文件被硬编码到内核空间中，将每个二进制文件的地址装入到一个数组_num_app中，再构建一个string型数组__app_name中存放各文件名，这样我们获得了文件名就可以知道其在app_name中的索引，再根据该索引访问num_app数组就能找到文件名对应的文件数据的具体地址，通过将该地址上的数据copy到对应task的pc指向的待执行地址再进行进程切换即可。



#### 将TaskManeger的current相关的功能剥离出来构建了Processor：

个人理解Processor这个词更能体现current_task所代表的意义，即当前的处理器状态。一个Processor其实就记录了一个cpu当前的所有状态。



#### user_shell和read()系统调用：

user_shell其实就是提供了一个用户接口，根据用户输入的文件名去创建进程执行对应的文件，具体逻辑比较简单，但是需要一个接受键盘输入的系统调用read()。具体调用流程为：

user::user_shell::getchar()  -> user::console::read() -> os::fs::sys_read() -> os::sbi::console_getchar() -> os::sbi::sbi_call()

最终利用RUSTSBI提供的接口完成单个字符的数据读取。



#### 练习题任务：

实现spawn（）系统调用，即不经过fork，直接复制一个进程，执行path中的文件

实现过程：

fork（）中完成对原TaskControlBlock的复制后，不进行地址空间数据的全复制，而是直接接入exec()部分的代码，即可完成spawn()的功能。

一个问题：需要自己写测例验证spwan()的执行情况，但是对于封装好的spwan()我无法分辨它是直接给新创建进程载入了新的文件数据还是使用了fork+exec的方式完成了该功能。

