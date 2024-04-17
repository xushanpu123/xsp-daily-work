## 2024/04/17

分离了driver_net,driver_display,driver_virtio,slab_allocator,allocator,axfs_vfs,memory_addr ,kernel_guard,spinlock,page_table_entry ,axlog,crate_interface,axalloc,lazy_init

尝试拆axconfig失败了，疑似build.rs中包含路径依赖。

下面是crates和modules目录下已完成拆分和尚未完成拆分的crates的统计，其中已完成19个，未完成的有24个：

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
| handler_table    | ✘        |
| linked_list      | ✘        |
| of               | ✘        |
| page_table       | ✘        |
| percpu           | ✘        |
| percpu_macros    | ✘        |
| ratio            | ✘        |
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
