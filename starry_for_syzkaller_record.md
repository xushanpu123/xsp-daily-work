工作仓库：https://gitee.com/xushanpu/starry_for-_syzkaller



工作日志：

为了支持 sshd。



第一步搞到 sshd 的二进制应用文件，因为 Starry 目前对于 musl 的支持比较好，所以我们需要想办法拿到使用 musl 编译的 sshd，由于 openssh 依赖于 openssl 和 zlib，所以要么去交叉编译获取一个编译后的文件，但是编译需要同时编译 openssl 和 zlib，且编译后会出现其他错误，导致无法运行，就使用一个 docker-compose 创建一个 alpine linux，然后从 alpine linux 中取程序，alpine linux 是一个基于 musl 库的轻量级 linux，有包管理工具 apk ，通过包管理工具下载的程序都是使用 musl 编译的。



在运行的时候又拉了 libcrypto.so.3 和 libz.so.1，放在 lib 目录下。



sshd 文件需要按照 alpine 中的路径，放在 /usr/sbin/ 目录下且运行时需要使用绝对路径，否则会报错

```
sshd re-exec requires execution with an absolute path，

/
```

支持 sshd 的时候遇到的问题：

```
Error relocating /usr/sbin/sshd: inflateEnd: symbol not found
ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQCFH/Iqx5ov7aYUbJZOnb5+oWclMmSXlYoCjiQB3lIYQ0n3cY85MnCD/VEdTfXxKtGINg7ndo0a7UrT3F79ntDaHpp55ac1sG6U9zR1QqlvMRvpYtrG/Xccx91d7IicyY3OJfs+mQySa/iRvGo8matCN2r9cGlTco4nh1NuSBU4Q8b9toDlHamrsRH+DFhBP5uW1LB/RrManb0Rsi84XKxqZCso0IA+q5zexdrUy4DtFJdU+bGET+ADhXU7jEV2eSrHDFJDsxVm8BU4CDVFzMiYGzUvLLY71joq8PAuo+y9lzWBu94lkisOdqdGDz5tD0pa0XXevYXe85YGYA0ssePEf3Bc0PHrYKQjTkOsiYARFwTPBKaihTKemv6UpMV4vRFPCRZ4hVC4IK8dq2S8p8f49gGKW1uidVYPjT+Is0J0Xvr4Sf+0szyBIoS9QtiDJ3o3bficeZ/AddY0xZsoQlygd7wKffS+K5znAuh/TeAiWGUtx2t6bhu9PilSIMNziz0= xsp@xsp-Legion-Y7000P-IAH7
Error relocating /usr/sbin/sshd: deflate: symbol not found

Error relocating /usr/sbin/sshd: deflateInit_: symbol not found

Error relocating /usr/sbin/sshd: inflate: symbol not found

Error relocating /usr/sbin/sshd: deflateEnd: symbol not found

Error relocating /usr/sbin/sshd: inflateInit_: symbol not found
```

musl 根据 stat 的 st_dev 和 st_ino 作为标识识别动态库，如果 st_dev 和 st_ino 与另一个的对应字段相同，则会被识别位同一个动态库，导致异常，代码如下：

```C
for (p=head->next; p; p=p->next) {
	if (p->dev == st.st_dev && p->ino == st.st_ino) {
		/* If this library was previously loaded with a
		 * pathname but a search found the same inode,ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQCFH/Iqx5ov7aYUbJZOnb5+oWclMmSXlYoCjiQB3lIYQ0n3cY85MnCD/VEdTfXxKtGINg7ndo0a7UrT3F79ntDaHpp55ac1sG6U9zR1QqlvMRvpYtrG/Xccx91d7IicyY3OJfs+mQySa/iRvGo8matCN2r9cGlTco4nh1NuSBU4Q8b9toDlHamrsRH+DFhBP5uW1LB/RrManb0Rsi84XKxqZCso0IA+q5zexdrUy4DtFJdU+bGET+ADhXU7jEV2eSrHDFJDsxVm8BU4CDVFzMiYGzUvLLY71joq8PAuo+y9lzWBu94lkisOdqdGDz5tD0pa0XXevYXe85YGYA0ssePEf3Bc0PHrYKQjTkOsiYARFwTPBKaihTKemv6UpMV4vRFPCRZ4hVC4IK8dq2S8p8f49gGKW1uidVYPjT+Is0J0Xvr4Sf+0szyBIoS9QtiDJ3o3bficeZ/AddY0xZsoQlygd7wKffS+K5znAuh/TeAiWGUtx2t6bhu9PilSIMNziz0= xsp@xsp-Legion-Y7000P-IAH7
		 * setup its shortname so it can be found by name. */
		if (!p->shortname && pathname != name)
			p->shortname = strrchr(p->name, '/')+1;
		close(fd);
		return p;
	}
}
```

所以对于不同的链接库 需要指定不同的st_ino 和 st_dev,但是starry对动态链接库的支持不完善，对所有的.so文件的st_dev和st_ino都置为0，所以在加载完 libcrypto.so.3后再加载libz.so.1后，由于二者的st_dev和st_ino都一致，所以starry会误认为libz.so.1已经加载过了，然而实际上这个文件并没有加载，所以在搜索符号表的时候会出现问题，又查询了相关文档知道st_dev和st_ino分别对应设备号和inode编号，但是starry只有一个FAT32文件系统，不存在这两个参数，所以这里我修改为了将文件路径按字符串hash的方法来判定。

至此能够完成链接部分，出现缺少syscall的问题：x86_64 116 号系统调用，补充一个 假的 syscall: setgroups。这里是因为sshd在网络通信的时候需要对对接用户分组，但是在支持syzkaller的工作中只需要跟sys_manager通信，所以这里直接返回了OK(0)。

Starry 的页表权限和 mprotect 并不完善，导致了页表映射和权限出了问题，直接给所有的映射的页开启了RWX 权限。

sshd 在运行的时候会读取文件 /etc/passwd 和文件用户，所以从 linux 中拉过来了一个 /etc/passwd 文件并做了简单的修改，只留下了 root 和 ssh 用户。

还有 sshd 在运行的时候会读取 /etc/ssh/sshd_config 文件，然后根据配置文件来选择相应的功能。然后又补充了这个文件。



sshd 运行需要 /var/empty 文件夹，所有从远程连接的 shell 都会用这个文件夹，但是 ArceOS 会使用 var 文件夹作为 ramfs 的一个入口。没办法从 testcase 里直接填充。所以将 ramfs 的代码直接给取消掉，然后使用文件系统的 empty 文件夹。

sshd 在运行的时候需要有相应的 private key 和 public key，这些 key 是由 ssh-keygen 去生成，但是使用ssh-keygen 去生成可能会造成一些未知的问题，因此在 alpine linux去使用ssh-keygen 在/etc/ssh文件夹下生成相应的key，然后复制到 Starry 里面。

由于 openssh 对于 key的权限有比较高的要求，要求只能由当前用户去读，但是Starry使用了fat32，并不支持文件权限，stat系统调用只返回777，所以会导致sshd由于权限问题无法继续去运行。所以需要去修改，目前使用临时的方案，在stat获取权限的时候直接给key文件的权限变为600，

补充了一个新的假SYSCALL getsid(124号系统调用)。

还有sshd 会使用 /dev/log 文件作为日志输出，这个日志将被systemd 读取和管理，但是Starry不支持这些东西，所以会导致无法运行，而且这个文件是作为unix socket 文件被读取，使用socket相关的系统调用去处理，目前Starry也缺少相关的处理能力，所以修改了socket_address_from能够返回一个Result结构，能够处理错误，如果是Unix socket文件，那么返回文件未找到。

最后将运行命令改为 /usr/sbin/sshd -d。-d 将显示调试信息。