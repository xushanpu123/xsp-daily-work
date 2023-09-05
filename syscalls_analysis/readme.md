这是一个获取linux所有系统调用并整理成表格的小work。

在linux源码中，复制

include/uapi/asm-generic/unistd.h到该目录下并改名为unistd2.h，复制

include/linux/syscalls.h到该目录下并改名为syscalls2.h。

然后启动catch.py脚本即可获取所有系统调用的简单信息，信息会以表格形式存入res.xlsx