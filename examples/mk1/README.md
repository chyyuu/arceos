```
cd arceos
## RISC-V64
make clean
make ARCH=riscv64 defconfig
make A=examples/mk1 ARCH=riscv64 run

## x86_64
make ARCH=x86_64 defconfig
make A=examples/mk1 ARCH=x86_64 run

## AArch64
make ARCH=aarch64 defconfig
make A=examples/mk1 ARCH=aarch64 run

## LoongArch64
make ARCH=loongarch64 defconfig
make A=examples/mk1 ARCH=loongarch64 run
```