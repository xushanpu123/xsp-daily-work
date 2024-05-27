## 2024/05/27

学习x86_64中断控制器相关的支持，尝试了一些blog中的用法，但是没有启动起来中断。

## 2024/05/26

模仿virtio中的examples实现了x86_64从pci总线上获取各个设备的流程

## 2024/05/25

实现了对aarch64的gpu，net设备的驱动

## 2024/05/24

通过了x86_64的编译，包括启动配置，对pci总线的支持

## 2024/05/23

修复了两处bug：

1、由于没有设置user/.cargo/config.toml中的x86_64编译设置，所以并没有让DISCARD debug相关段的设定生效，导致initproc过大无法在文件系统中装入；

2、把启动参数设计成：

```
-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
```

会在init gpu时卡死，但是去掉bus=virtio-mmio-bus.0后恢复正常，原因未知。



学习了virtio中使用pci总线和控制各设备的方法：

首先需要在qemu启动时使用参数：

```
-device virtio-gpu-pci -vga none \
-device virtio-blk-pci,drive=x0 -drive file=$(img),if=none,format=raw,id=x0 \
-device virtio-net-pci,netdev=net0 -netdev user,id=net0,hostfwd=tcp::5555-:5555
```

接下来分析了控制设备的方法：

```rust
fn enumerate_pci(mmconfig_base: *mut u8) {
    info!("mmconfig_base = {:#x}", mmconfig_base as usize);

    let mut pci_root = unsafe { PciRoot::new(mmconfig_base, Cam::Ecam) };
    for (device_function, info) in pci_root.enumerate_bus(0) {
        let (status, command) = pci_root.get_status_command(device_function);
        info!(
            "Found {} at {}, status {:?} command {:?}",
            info, device_function, status, command
        );
        if let Some(virtio_type) = virtio_device_type(&info) {
            info!("  VirtIO {:?}", virtio_type);

            // Enable the device to use its BARs.
            pci_root.set_command(
                device_function,
                Command::IO_SPACE | Command::MEMORY_SPACE | Command::BUS_MASTER,
            );
            dump_bar_contents(&mut pci_root, device_function, 4);

            let mut transport =
                PciTransport::new::<HalImpl>(&mut pci_root, device_function).unwrap();
            info!(
                "Detected virtio PCI device with device type {:?}, features {:#018x}",
                transport.device_type(),
                transport.read_device_features(),
            );
            virtio_device(transport);
        }
    }
}
```

重点是利用virtio_type的比对可以获取到pci总线上对应类型的物理设备，然后获取到transport值，利用这个值可以新建一个对应的抽象设备来管理对应的物理设备：

```rust
let mut blk = VirtIOBlk::<HalImpl, T>::new(transport).expect("failed to create blk driver");

let mut gpu = VirtIOGpu::<HalImpl, T>::new(transport).expect("failed to create gpu driver"); 

let mut net =virtio_drivers::device::net::VirtIONetRaw::<HalImpl, T,NET_QUEUE_SIZE>::new(transport) .expect("failed to create net driver");
```

随后就可以用virtio-drivers提供的抽象设备接口控制各种设备了。



## 2024/05/22

补齐了前面的日志，修复了makefile中找不到gpu等设备的bug

## 2024/05/21

继续做ch9的四种架构支持，未完成，找杨金博讨论了一下去除riscv的方法，但是还没除完。

## 2024/05/20

继续做ch9的四种架构支持，未完成

## 2024/05/19

在杨金博的帮助下发现了x86_64报segmentfault的原因，发现了rcore_tutorial关于mmap映射的bug，解决该问题。

## 2024/05/18

尝试解决x86_64报segmentfault的问题失败

## 2024/05/17

做ch9的多架构支持，但是x86_64一直报segmentfault，其它架构可以出usershell。

## 2024/05/16

继续支持基于polyhal的ch8的四种架构，其中x86_64报segmentfault,loongaich64和aarch64的adder测例无法通过

## 2024/05/15

支持基于polyhal的ch8的四种架构，未完成

## 2024/05/14

想不起来具体做了啥

## 2024/05/13

写整个工作的相关文档

## 2024/05/12

和胡志文一起完成了基于polyhal的ch9基于riscv64的支持，我完成其中的关于地址空间的修改

## 2024/05/11

继续做ch9的支持，想不起来具体做了什么工作

## 2024/05/10

开始做ch9的支持，没有做完，想不起来具体做了啥

## 2024/05/09

啥也没干

## 2024/05/08

想不起来干了啥

## 2024/05/07

移植了基于polyhal的rcore-tutorial ch6，四种架构

## 2024/05/06

移植了基于polyhal的rcore-tutorial ch3，riscv

## 2024/05/05

移植了基于polyhal的rcore-tutorial ch2，四种架构

## 2024/05/04

移植了基于polyhal的rcore-tutorial ch2，支持三种架构riscv64,aarch64,loongarch64，x86有segmentfault bug

## 2024/05/03

移植了基于polyhal的rcore-tutorial ch2，riscv64

## 2024/05/02

移植了基于polyhal的rcore-tutorial ch1，四种架构

## 2024/05/01

移植了基于polyhal的rcore-tutorial ch1，riscv

## 2024/04/31

完成了udpserver、bwbench、httpclient、echoserver、tls、priority、parellel、sleep的CI和模块化。

## 2024/04/30

完成了memtest，exception，task/yield的CI，其中出现了与log有关的bug，因为log的“0.4.21”版本支持macro_use的宏定义时会出现未知bug，所以修改了axtask模块中的macro_use,将所有的宏都加了log::，更好的解决方案应该是锁定log为低版本，但没有找到方法。
## 2024/04/23

上午研究了一下贾爷写的CI，下午痛风发作了请了个假，没什么进度，明天开始动手写。

## 2024/04/22

拆分出了一个基于arceos的最小可运行模块hellotest,链接为https://github.com/Arceos-crates/hellotest.git，运行方法见readme。

## 2024/04/21

完成了所有模块的拆分，只留下helloworld作为初始启动的模块。

starry链接：https://github.com/xushanpu123/Starry

分离出的crate链接：https://github.com/Arceos-crates

## 2024/04/20

研究拆分含有路径依赖的build.rs的crate如何拆分，没解决。

## 2024/04/19

分离了除包含build.rs的crates和modules下的所有的crates：

| Crate 名称                                     | 是否完成 |
| ---------------------------------------------- | -------- |
| axerror                                        | ✓        |
| axio                                           | ✓        |
| driver_pci                                     | ✓        |
| driver_common                                  | ✓        |
| driver_block                                   | ✓        |
| driver_net                                     | ✓        |
| driver_display                                 | ✓        |
| driver_virtio                                  | ✓        |
| slab_allocator                                 | ✓        |
| allocator                                      | ✓        |
| axfs_vfs                                       | ✓        |
| memory_addr                                    | ✓        |
| kernel_guard                                   | ✓        |
| spinlock                                       | ✓        |
| page_table_entry                               | ✓        |
| axlog                                          | ✓        |
| crate_interface                                | ✓        |
| axalloc                                        | ✓        |
| lazy_init                                      | ✓        |
| arm_gic                                        | √        |
| arm_pl011                                      | √        |
| axfs_devfs                                     | √        |
| axfs_ramfs                                     | √        |
| capability                                     | √        |
| dw_apb_uart                                    | √        |
| flatten_objects                                | √        |
| handler_table                                  | √        |
| linked_list                                    | √        |
| of                                             | √        |
| page_table                                     | √        |
| percpu                                         | √        |
| percpu_macros                                  | √        |
| ratio                                          | √        |
| scheduler                                      | √        |
| timer_list                                     | √        |
| tuple_for_each                                 | √        |
| axconfig                                       | ✘        |
| axdisplay                                      | √        |
| axdriver                                       | ✘        |
| axfs                                           | √        |
| axhal                                          | √        |
| axmem                                          | √        |
| axnet                                          | √        |
| axprocesshttps://github.com/xushanpu123/Starry | √        |
| axruntime                                      | √        |
| axsignal                                       | √        |
| axsync                                         | √        |
| axtask                                         | √        |

## 2024/04/18

分离了linked_list，axfs_ramfs，scheduler，arm_gic，arm_pl011，axfs_devfs，capability，dw_apb_uart，flatten_objects,of，timer_list，tuple_for_each，至此crates目录下的crates全部分离完毕，剩下的都与axconfig有关，开始修axconfig。

目前进度：



| Crate 名称                                | 是否完成 |
| ----------------------------------------- | -------- |
| axerror                                   | ✓        |
| axiohttps://github.com/xushanpu123/Starry | ✓        |
| driver_pci                                | ✓        |
| driver_common                             | ✓        |
| driver_block                              | ✓        |
| driver_net                                | ✓        |
| driver_display                            | ✓        |
| driver_virtio                             | ✓        |
| slab_allocator                            | ✓        |
| allocator                                 | ✓        |
| axfs_vfs                                  | ✓        |
| memory_addr                               | ✓        |
| kernel_guard                              | ✓        |
| spinlock                                  | ✓        |
| page_table_entry                          | ✓        |
| axlog                                     | ✓        |
| crate_interface                           | ✓        |
| axalloc                                   | ✓        |
| lazy_init                                 | ✓        |
| arm_gic                                   | √        |
| arm_pl011                                 | √        |
| axfs_devfs                                | √        |
| axfs_ramfs                                | √        |
| capability                                | √        |
| dw_apb_uart                               | √        |
| flatten_objects                           | √        |
| handler_table                             | √        |
| linked_list                               | √        |
| of                                        | √        |
| page_table                                | √        |
| percpu                                    | √        |
| percpu_macros                             | √        |
| ratio                                     | √        |
| scheduler                                 | √        |
| timer_list                                | √        |
| tuple_for_each                            | √        |
| axconfig                                  | ✘        |
| axdisplay                                 | ✘        |
| axdriver                                  | ✘        |
| axfs                                      | ✘        |
| axhal                                     | ✘        |
| axmem                                     | ✘        |
| axnet                                     | ✘        |
| axprocess                                 | ✘        |
| axruntime                                 | ✘        |
| axsignal                                  | ✘        |
| axsync                                    | ✘        |
| axtask                                    | ✘        |

## 2024/04/17

分离了driver_net,driver_display,driver_virtio,slab_allocator,allocator,axfs_vfs,memory_addr ,kernel_guard,spinlock,page_table_entry ,axlog,crate_interface,axalloc,lazy_init，percpu_macros,percpu,page_table,ratio，handler_table

尝试拆axconfig失败了，疑似build.rs中包含路径依赖。

与axconfig有关的模块有：axhal，

下面是crates和modules目录下已完成拆分和尚未完成拆分的crates的统计，其中已完成21个，未完成的有22个：

| Crate 名称       | 是否完成 |
| ---------------- | -------- |
| axerror          | ✓        |
| axio             | ✓        |
| driver_pci       | ✓        |
| driver_common    | ✓        |
| driver_block     | ✓        |
| driver_net       | ✓        |
| driver_display   | ✓        |
| driver_virtio    | ✓        |
| slab_allocator   | ✓        |
| allocator        | ✓        |
| axfs_vfs         | ✓        |
| memory_addr      | ✓        |
| kernel_guard     | ✓        |
| spinlock         | ✓        |
| page_table_entry | ✓        |
| axlog            | ✓        |
| crate_interface  | ✓        |
| axalloc          | ✓        |
| lazy_init        | ✓        |
| arm_gic          | ✘        |
| arm_pl011        | ✘        |
| axfs_devfs       | ✘        |
| axfs_ramfs       | ✘        |
| capability       | ✘        |
| dw_apb_uart      | ✘        |
| flatten_objects  | ✘        |
| handler_table    | √        |
| linked_list      | ✘        |
| of               | ✘        |
| page_table       | √        |
| percpu           | √        |
| percpu_macros    | √        |
| ratio            | √        |
| scheduler        | ✘        |
| timer_list       | ✘        |
| tuple_for_each   | ✘        |
| axconfig         | ✘        |
| axdisplay        | ✘        |
| axdriver         | ✘        |
| axfs             | ✘        |
| axhal            | ✘        |
| axmem            | ✘        |
| axnet            | ✘        |
| axprocess        | ✘        |
| axruntime        | ✘        |
| axsignal         | ✘        |
| axsync           | ✘        |
| axtask           | ✘        |



## 2024/04/16

分离了starry的crates axerror，axio，driver_pci,driver_common,driver_block分离后代码位于https://github.com/xushanpu123/Starry，分离出的crates位于https://github.com/Arceos-crates 下的对应同名仓库



## 2024/04/15

分离crates后的starry仓库：https://github.com/xushanpu123/Starry

分离出的crates的organization：https://github.com/Arceos-crates

初步确定了后续的工作是分离一些starry和ByteOS中的crates。

## 2024/04/14

​	主干代码全部学习完毕，跟杨金博商量了一下，明天做一下excutor的模块拆分

## 2024/04/13

​	继续分析文件系统模块，基本研究清楚了ByteOS的目录系统的设计方式，理清了一个应用程序依据文件路径获取文件访问的FCB或找到对应设备结构的方式。明天准备完结文件系统模块并且继续研究内存模块和task模块。

## 2024/04/12

​	在杨金博的要求下分析了ByteOS的文件系统模块：

​	分析了vfs文件系统挂载的代码，分析了FAT32，devfs，procfs和Ramfs的逻辑初始化过程和FAT32、Ramfs转化用户访问文件的地址到磁盘块地址的流程。

## 2024/04/11

​	继续分析syz_manager，manager复制文件到qemu虚拟机中的命令为：

```go
//   syzkaller/vm/qemu/qemu.go
func (inst *instance) Copy(hostSrc string) (string, error) {
...
_, err := osutil.RunCmd(10*time.Minute*inst.timeouts.Scale, "", "scp", args...)
...
}
```

​	在创建虚拟机实例的时候，发现使用了长管道进行了主机和VM的通信：

```go
inst.rpipe, inst.wpipe, err = osutil.LongPipe()
```

​	对此机制，暂时没研究，chatgpt的解释如下：

```
在一般情况下，虚拟机和主机之间是无法直接使用管道进行通信的。虚拟机通常是由虚拟化软件（如 VirtualBox、VMware、QEMU 等）创建的，它们提供了虚拟机和主机之间的虚拟化接口，通过这个接口可以实现虚拟机和主机之间的通信。

然而，在某些情况下，可以通过一些特殊的技术来实现虚拟机和主机之间的通信，比如 QEMU 的用户模式网络堆栈（user-mode networking stack）。这种技术可以将虚拟机中的网络流量转发到主机上，从而实现虚拟机和主机之间的通信。但这种方式并不是使用管道直接通信，而是通过网络协议来进行通信的。

在代码中所提到的长管道（LongPipe）可能是一种特殊的技术，用于模拟虚拟机和主机之间的通信。但具体实现细节需要参考代码的其他部分来确定。
```

​	所以目前猜测是做了一层封装，本质还是网络通信。

​	启动qemu的方案代码是：

```go
	qemu := osutil.Command(inst.cfg.Qemu, args...)
	qemu.Stdout = inst.wpipe
	qemu.Stderr = inst.wpipe
	if err := qemu.Start(); err != nil {
		return fmt.Errorf("failed to start %v %+v: %v", inst.cfg.Qemu, args, err)
	}
```

​	因此如果想用syz_excutor启动ByteOS也是类似的。

​	syz_manager启动fuzzer的命令代码是：

```go
	args := &instance.FuzzerCmdArgs{
		Fuzzer:    fuzzerBin,
		Executor:  executorBin,
		Name:      instanceName,
		OS:        mgr.cfg.TargetOS,
		Arch:      mgr.cfg.TargetArch,
		FwdAddr:   fwdAddr,
		Sandbox:   mgr.cfg.Sandbox,
		Procs:     procs,
		Verbosity: fuzzerV,
		Cover:     mgr.cfg.Cover,
		Debug:     *flagDebug,
		Test:      false,
		Runtest:   false,
		Optional: &instance.OptionalFuzzerArgs{
			Slowdown:   mgr.cfg.Timeouts.Slowdown,
			RawCover:   mgr.cfg.RawCover,
			SandboxArg: mgr.cfg.SandboxArg,
		},
	}
	cmd := instance.FuzzerCmd(args)
	outc, errc, err := inst.Run(mgr.cfg.Timeouts.VMRunningTime, mgr.vmStop, cmd)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to run fuzzer: %v", err)
	}
```

## 2024/04/10

​	否决了4月9号的方案，准备修改syz_manager的配置来自动fuzzing ByteOS，目前看需要修改target os的编译器到musl-gcc，需要修改fuzzing内核路径的相关配置。

​	截止中午，成功利用syzkaller自带的makefile编译成功了需要的x86-musl-linux的syz-fuzzer，syz-excutor，syz-execprog和syz-stress，其中除了syz-excutor都是go源程序编译的，syz-excutor是C++写的，需要将对应的编译器修改了musl的。下午继续修改syz_manager将这几个程序放到ByteOS里面跑一下。

​	分析了一下syz_manager，配置了my.cfg的目标os和架构，修改了instance.Copy的代码来适配ByteOS，但是目前还是没能自启动ByteOS，明天继续尝试。

## **2024/04/09**

​    在alpine linux上编译了syzkaller，重新捋了一下syzkaller在linux宿主机和linux vm中跑的流程，晚上尝试改一下syz-manager让它能控制skzkaller在byteOS上生成测例并执行一下测例。