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
t1stack:
    .space 4096 * 4
    .global t1stacktop
t1stacktop:
t2stack:
    .space 4096 * 4
    .global t2stacktop
t2stacktop:
t3stack:
    .space 4096 * 4
    .global t3stacktop
t3stacktop: