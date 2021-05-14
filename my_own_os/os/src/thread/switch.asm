# 我们将会用一个宏来用循环保存寄存器。这是必要的设置
.altmacro
# 寄存器宽度对应的字节数
.set    REG_SIZE, 8
# Context 的大小
.set    CONTEXT_SIZE, 34

# 宏：将寄存器存到栈上
.macro SAVE reg, offset
    sd  \reg, \offset*8(sp)
.endm

.macro SAVE_N n
    SAVE  x\n, \n
.endm


# 宏：将寄存器从栈中取出
.macro LOAD reg, offset
    ld  \reg, \offset*8(sp)
.endm

.macro LOAD_N n
    LOAD  x\n, \n
.endm

addi sp, sp, -13*8
SAVE s0, 0
SAVE s1, 1
SAVE s2, 2
SAVE s3, 3
SAVE s4, 4
SAVE s5, 5
SAVE s6, 6
SAVE s7, 7
SAVE s8, 8
SAVE s9, 9
SAVE s10,10
SAVE s11,11
SAVE ra, 12
sd sp, 0(a0)

ld sp, 0(a1)
LOAD s0, 0
LOAD s1, 1
LOAD s2, 2
LOAD s3, 3
LOAD s4, 4
LOAD s5, 5
LOAD s6, 6
LOAD s7, 7
LOAD s8, 8
LOAD s9, 9
LOAD s10,10
LOAD s11,11
LOAD ra, 12
addi sp,sp,13*8
sd x0, 0(a1)