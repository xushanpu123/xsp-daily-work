# 		chapter1：构建裸机下的程序执行环境

## 实验目的

​            在裸机（即target = riscv64gc-unknown-none-elf)环境下执行最简单的rust程序"helloworld"。

## 实验过程

####  **1、交叉编译：将编译的目标平台修改为“riscv64gc-unknown-none-elf“**

 **方式**：在工程目录下建立"$\textcolor{red}{.cargo/config} $"文件，并在其中写入如下内容：

```
[build]
target = "riscv64gc-unknown-none-elf"  
```

   此时，在主目录下执行<font color='redorange'>cargo build</font>,即可在目标平台上编译构建项目了。



#### 2、解决交叉编译的各种问题

**问题一：编译默认依赖标准库$\textcolor{red}{std} $，而<font color='blue'>riscv64gc-unknown-none-elf</font>平台无标准库**

​        **解决方法**：在 `main.rs` 的开头加上一行 `#![no_std]`， 告诉 Rust 编译器不使用 Rust 标准库 std 转而使用核心库 core。



**问题二：没有提供编译所必须的panic语义**

​       **解决方法**：增加下面的代码：

```rust
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

```

**问题三：由于没有std标准库，无法完成以`main`为起点的一些初始化操作**

**解决方案**：在 `main.rs` 的开头加入设置 `#![no_main]` 告诉编译器我们没有一般意义上的 `main` 函数， 并将原来的 `main` 函数删除。这样编译器也就不需要考虑初始化工作了。



**遗留问题：此时虽然可以编译成功，但是我们的可执行程序没有了起点位置，因此运行不了什么东西。**

**解决方案**：默认的起点位置为_start，因此创建一个__start函数即可。

#### 3、支持裸机运行的过程

**裸机运行的make命令：make run**

```makefile
run: build
	 @qemu-system-riscv64 \
         -machine virt \
         -nographic \
         -bios $(BOOTLOADER) \
         -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)
```

- `-bios $(BOOTLOADER)` 意味着硬件加载了一个 BootLoader 程序，即 RustSBI
- `-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)` 表示硬件内存中的特定位置 `$(KERNEL_ENTRY_PA)` 放置了操作系统的二进制代码 `$(KERNEL_BIN)` 。 `$(KERNEL_ENTRY_PA)` 的值是 `0x80200000` 。



**make build的主要内容**

```makefile
# Building
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
build: env $(KERNEL_BIN)

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
	@cargo build --release
```

`可以看到，cargo build形成的elf文件被objcopy工具处理后，形成了可执行文件，被make run命令 所执行。且执行的起始位置为0x80200000。



**设置正确的内存布局**

​        内核程序想要在裸机上正常运行，必须合理设置其布局，为程序运行分配各个分段。这个内核程序保存在src/linker.ld中，为使其发挥作用，需要在$\textcolor{red}{.cargo/config} $  中增加如下内容：

```
[target.riscv64gc-unknown-none-elf]
rustflags = [
     "-Clink-arg=-Tsrc/linker.ld", "-Cforce-frame-pointers=yes"
]
```

​        此后，我们设计 $\textcolor{red}{linker.ld} $:

```
OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;


SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }


    . = ALIGN(4K);
    ebss = .;
    ekernel = .;


    /DISCARD/ : {
        *(.eh_frame)
    }

}
```

 **对linker.ld做一些必要的解释**：**.**   代表当前的内存位置，初始时为BASE_ADDRESS = 0x80200000，即为上面规定的内核代码开始执行的内存位置。可以看到，从BASE_ADDRESS 开始，.text，.rodata，.data，.bss分别顺序占用4K的内存空间，且每个段的起点和终点均有标记。



**初始化栈空间布局**

```.asm
 	.section .text.entry
	.globl _start
_start:
    la sp, boot_stack_top
    call rust_main
	.section .bss.stack
	.globl boot_stack
boot_stack:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:
```

此为src/entry.asm中的内容，用来开辟栈空间，该空间大小为4096*16B，且栈顶为boot_stack_top，栈底为boot_stack。同时，在全局入口地址_start中，把boot_stack_top设置为了sp的初始值，再跳转到函数rust_main的位置。



**清空.bss段**

```rust
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}
```

这段代码功能比较简单，就是把.bss段全部置0。至此，代码可以在裸机上运行了。



#### **4、支持必要的程序接口**

**获取RUSTSBI的服务**

​		当计算机处于内核态时，可以获取RUSTSBI提供的一些基本服务，其基本格式为：

```rust
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x17") which,
        );
    }
    ret
}
```

其中，whtch为SBI调用的编号，arg0~arg2用来传递sbi调用的返回值。至此，我们可以封装一些必要的程序接口：

```rust
const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_SHUTDOWN: usize = 8;
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    panic!("It should shutdown!");
}

```

#### 5、裸机打印"helloworld"并退出

​        实验代码通过上面提供的程序接口实现了print和println以及其它的相关宏定义，从而通过如下代码即可完成功能：

```rust
#[no_mangle]
pub fn rust_main() -> ! {
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
        fn sbss();
        fn ebss();
        fn boot_stack();
        fn boot_stack_top();
    }
    clear_bss();
    logging::init();
    println!("Hello, world!");
    trace!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    debug!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    info!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    warn!(
        "boot_stack [{:#x}, {:#x})",
        boot_stack as usize, boot_stack_top as usize
    );
    error!(".bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    panic!("Shutdown machine!");
}
```



下面做重要代码的解析：

```rust
//通过这种形式的声明，可以获取其它文件中声明的全局tag或函数的物理地址，从而能够在后面打印它，这种方式在后面做栈相关的操作的时候会经常使用。
extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
        fn sbss();
        fn ebss();
        fn boot_stack();
        fn boot_stack_top();
    }
```

至此，chapter1的功能：在裸机下跑通helloworld实现。