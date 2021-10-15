1、下载好rcore_tutorial-v3后在os目录下make run安装cargo包的时候出现报错：

```
error: failed to get `riscv` as a dependency of package `os v0.1.0 (/home/xsp/rCore-Tutorial-v3/os)`

Caused by:
  failed to load source for dependency `riscv`

Caused by:
  Unable to update https://github.com/rcore-os/riscv

Caused by:
  failed to fetch into: /home/xsp/.cargo/git/db/riscv-ab2abd16c438337b

Caused by:
  network failure seems to have happened
  if a proxy or similar is necessary `net.git-fetch-with-cli` may help here
  https://doc.rust-lang.org/cargo/reference/config.html#netgit-fetch-with-cli

Caused by:
  SSL error: received early EOF; class=Ssl (16); code=Eof (-20)
make: *** [Makefile:60：kernel] 错误 101
```

解决方案：只是网络不好而已，跟代理等等都没关系，换个wifi就解决了。



2、分析可执行elf文件的工具：rust-readobj -h target/riscv64gc-unknown-none-elf/debug/os

​      反汇编工具和shell： rust-objdump -S target/riscv64gc-unknown-none-elf/debug/os



##### 在裸机上输出"hello world！"所需要进行的操作

分为两个层面：

1、先运行不需要std支持（有底层os）的程序（个人理解为是rust本身运行所需要满足的条件）

​    1）rust程序要运行，必须能够找到入口位置，std标准库中提供的入口点在main处，移除了std后，程序无法找到入口，而rust默认的入口地址是_start，所以我们添加__start，便可以找到入口位置了（实际上，这里写入一个跳转指令就可以跳转到我们希望的任何真正的入口位置）

  2）rust程序运行，必须有panic处理函数，std被移除后，panic_handler必须自己添加上；

 3）输出和退出不再被std写好，必须自己去调用底层的系统调用。



2、移除底层操作系统后，即在裸机上运行（个人理解是为用户程序提供系统调用）

1）首先基础功能如打印和退出，已经由更底层的rustsbi写好了，我们只需要调用它的服务就可以了。需要把rustsbi放到0x80000000的位置（需要在../bootloader/rustqemu.bin下添加rustsbi，才能正常完成指导书的命令）

2）os想要运行起来，必须把其代码（即我们编译链接好的内核文件）放到rustsbi运行完毕后默认跳转到的内存位置0x80200000，这样开机后rustsbi运行后，就可以由操作系统来接管计算机。为了完成这个目标，我们借助ld文件来调整整个可执行文件的内存布局，把_start的位置放置到0x80200000，这样rustsbi一执行完就会去执行逻辑开端__start，后续的跳转和运行工作利用代码本身的内在逻辑即可顺利执行了。同时我们在ld文件中给整个可执行文件.text,.bss,.data.rodate等分配位置，并且把编译器编译好的对应的各个输入文件的段放入其中。

3）程序执行必须设置好堆栈，所以我们应该在entry.asm文件中设置好分配给堆栈的空间，并且把堆栈指针sp指向我们分配给堆栈的位置；

4）.bss段数据必须全是0，我们分配给.bss的内存空间不一定满足这个条件，所以必须全部清零。

至此，我们便可以在用户态下利用系统调用来进行打印和正确退出了



**与Write有关的宏的位置**：

```rust
use core::fmt::{self,Write};
```



**关于吴一凡指导书的一个与个人实验不符合的地方**

```rust
//吴一凡指导书gdb debug方式
cargo build --release

rust-objcopy --binary-architecture=riscv64 target/riscv64gc-unknown-none-elf/release/os --strip-all -O binary target/riscv64gc-unknown-none-elf/release/os.bin

qemu-system-riscv64 -machine virt -nographic -bios ../bootloader/rustsbi-qemu.bin -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 -S -s
```

然而无法实现debug，上网查阅资料说--release形式的cargo build无法形成gdb 的symbol，后使用cargo build使用debug目录下的可执行文件就可以正常使用gdb了。