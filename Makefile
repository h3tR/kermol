.PHONY: all run clean libkermol_objects grub_cfg check_multiboot2

all: kermol.iso

libkermol.a:
	cargo build

libkermol_objects: libkermol.a
	mkdir -p target/x86_64-kermol/debug/objects
	ar x target/x86_64-kermol/debug/libkermol.a --output target/x86_64-kermol/debug/objects

kermol: libkermol_objects
	gcc -T linker.ld -o target/x86_64-kermol/debug/kermol target/x86_64-kermol/debug/objects/*.o -ffreestanding -nostdlib

kermol.bin: kermol
	mkdir -p target/x86_64-kermol/iso/boot
	objcopy -O binary -S target/x86_64-kermol/debug/kermol target/x86_64-kermol/iso/boot/kermol.bin

check_multiboot2: kermol.bin
	grub2-file --is-x86-multiboot2 target/x86_64-kermol/iso/boot/kermol.bin

grub_cfg:
	mkdir -p target/x86_64-kermol/iso/boot/grub/
	cp grub.cfg target/x86_64-kermol/iso/boot/grub/grub.cfg

kermol.iso: check_multiboot2 grub_cfg
	grub2-mkrescue -v -o target/x86_64-kermol/kermol.iso target/x86_64-kermol/iso

run: kermol.iso
	 qemu-system-x86_64 -drive format=raw,file=target/x86_64-kermol/kermol.iso \
          -m 256M \
          -no-reboot \
          -serial stdio
          #-drive id=nvme0,file=disk.img,if=none \
          #-device nvme,drive=nvme0,serial=deadbeef \

clean:
	cargo clean
