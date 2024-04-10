## **2024/04/09**

​    在alpine linux上编译了syzkaller，重新捋了一下syzkaller在linux宿主机和linux vm中跑的流程，晚上尝试改一下syz-manager让它能控制skzkaller在byteOS上生成测例并执行一下测例。



## 2024/04/10

​	否决了4月9号的方案，准备修改syz_manager的配置来自动fuzzing ByteOS，目前看需要修改target os的编译器到musl-gcc，需要修改fuzzing内核路径的相关配置。

​	截止中午，成功利用syzkaller自带的makefile编译成功了需要的x86-musl-linux的syz-fuzzer，syz-excutor，syz-execprog和syz-stress，其中除了syz-excutor都是go源程序编译的，syz-excutor是C++写的，需要将对应的编译器修改了musl的。下午继续修改syz_manager将这几个程序放到ByteOS里面跑一下。

​	分析了一下syz_manager，配置了my.cfg的目标os和架构，修改了instance.Copy的代码来适配ByteOS，但是目前还是没能自启动ByteOS，明天继续尝试。

