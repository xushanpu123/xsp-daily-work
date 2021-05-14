.section .text.entry
    .globl _start
_start:
    la sp, bootstacktop
    call rust_main

    .section .bss.stack
    .align 12
    .global bootstack
bootstack:
    .space 4096 * 4
    .global bootstacktop
bootstacktop:
.global bootstack1
bootstack1:
    .space 4096 * 4
    .global bootstacktop1
bootstacktop1:
.global bootstack2
bootstack2:
    .space 4096 * 4
    .global bootstacktop2
bootstacktop2:
.global bootstack3
bootstack3:
    .space 4096 * 4
    .global bootstacktop3
bootstacktop3: