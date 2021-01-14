global_asm!(include_str("trap.S"));
const SYS_READ: usize = 2;
const SYS_WRITE: usize = 3;
const SYS_EXIT: usize = 4;