# Minimal user-space program for mk1
# This program writes "Hello, world!" to stdout and exits

.section .text
.globl _start
_start:
    # sys_write(fd=1, buf=msg, count=14)
    li      a0, 1           # fd = STDOUT_FILENO
    la      a1, msg         # buf = address of message
    li      a2, 14          # count = length of message
    li      a7, 64          # syscall number for sys_write (RISC-V Linux ABI)
    ecall

    # sys_exit(exit_code=0)
    li      a0, 0           # exit_code = 0 (success)
    li      a7, 93          # syscall number for sys_exit (RISC-V Linux ABI)
    ecall

    # Should never reach here, but just in case
    j       .

.section .rodata
.globl msg
msg:
    .ascii "Hello, world!\n"
