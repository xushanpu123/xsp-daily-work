### 基于polyhal的rcore tutorial多架构支持

#### 1、支持多架构的硬件抽象层polyhal

主要介绍什么是硬件抽象层，硬件无关，polyhal实现了哪些硬件抽象，提供了哪些接口，支持了哪几种架构，利用了rust的语言什么机制来实现封装等





##### 2、基于polyhal的rcore tutorial支持

描述对rcore tutorial的代码部分做了哪些更改，将哪些涉及到底层架构的操作换成了利用polyhal支持从而实现硬件无关

##### 3、多架构的编译和运行环境支持

主要介绍对编译和运行脚本的修改，主要分为支持文件系统前和支持文件系统后，以及最后增加设备后的一些修改，会顺带介绍一下因为高半核设计和x86_64的特点产生的一些bug和解决方法

##### 4、对于多架构中断和设备驱动的支持

主要介绍ch9不同架构下的中断控制器，x86下pci总线的支持，以及不同获取设备对象的方式，这里可以结合rust的trait特征来分析使用trait作为参数的便利性。

##### 5、遇到的问题和挑战及总结

总结一下过程中遇到了哪些bug，用了什么方法去排查bug和最终怎么解决的。说一下这个工作的整体工作流程和在别的操作系统上的可拓展性。