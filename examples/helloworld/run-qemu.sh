#!/bin/bash
# Runner script for ArceOS helloworld
# This script converts the ELF binary to raw binary and runs it in QEMU

set -e

ELF_FILE="$1"

if [ -z "$ELF_FILE" ]; then
    echo "Usage: $0 <elf-file>"
    exit 1
fi

# Generate bin filename from elf filename
BIN_FILE="${ELF_FILE%.elf}"
if [ "$BIN_FILE" = "$ELF_FILE" ]; then
    # No .elf extension, just append .bin
    BIN_FILE="${ELF_FILE}.bin"
else
    BIN_FILE="${BIN_FILE}.bin"
fi

# Convert ELF to raw binary
echo "[Runner] Converting ELF to binary: $ELF_FILE -> $BIN_FILE"
rust-objcopy --binary-architecture=riscv64 "$ELF_FILE" --strip-all -O binary "$BIN_FILE"

# Run in QEMU
echo "[Runner] Starting QEMU..."
exec qemu-system-riscv64 \
    -machine virt \
    -bios default \
    -kernel "$BIN_FILE" \
    -m 128M \
    -smp 1 \
    -nographic
