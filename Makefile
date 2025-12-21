.PHONY: all run clean libkermol_objects limine limine_setup superclean

ISO_DIR=target/x86_64-kermol/iso
#this might differ
OVMF_PATH=/usr/share/ovmf/OVMF.fd


all: kermol.iso

libkermol.a:
	cargo build

libkermol_objects: libkermol.a
	mkdir -p target/x86_64-kermol/debug/objects
	ar x target/x86_64-kermol/debug/libkermol.a --output target/x86_64-kermol/debug/objects

kermol.elf: libkermol_objects
	gcc -T linker.ld -o $(ISO_DIR)/boot/kermol.elf target/x86_64-kermol/debug/objects/*.o -ffreestanding -nostdlib -static

kermol.bin: kermol.elf
	objcopy -O binary -S target/x86_64-kermol/debug/kermol.elf $(ISO_DIR)/boot/kermol.bin

#run ```make limine_setup``` before running this
limine:
	mkdir -p $(ISO_DIR)
	cp -r limine_isofiles/* $(ISO_DIR)
	cp limine.conf $(ISO_DIR)/boot

#limine must be installed on the system to use this
kermol.iso: limine kermol.elf
	xorriso -as mkisofs -R -r -J -b boot/limine-bios-cd.bin \
            -no-emul-boot -boot-load-size 4 -boot-info-table -hfsplus \
            -apm-block-size 2048 --efi-boot boot/limine-uefi-cd.bin \
            -efi-boot-part --efi-boot-image --protective-msdos-label \
            $(ISO_DIR) -o target/x86_64-kermol/kermol.iso

bios.bin:
	cp $(OVMF_PATH) bios.bin

run: kermol.iso bios.bin
	 qemu-system-x86_64 -drive format=raw,file=target/x86_64-kermol/kermol.iso \
		  	-pflash bios.bin \
		  	-net none \
          	-m 512M \
          	-no-reboot \
			-vga vmware \

          #-drive id=nvme0,file=disk.img,if=none \
          #-device nvme,drive=nvme0,serial=deadbeef \

#for more info read [https://codeberg.org/Limine/Limine/src/branch/v10.x/USAGE.md#bios-uefi-hybrid-iso-creation]
limine_setup:
	git clone https://codeberg.org/Limine/Limine.git --branch=v10.x-binary --depth=1
	mkdir -p limine_isofiles/boot/
	cp Limine/limine-bios.sys limine_isofiles/boot/
	cp Limine/limine-*-cd.bin limine_isofiles/boot/
	mkdir -p limine_isofiles/EFI/BOOT
	cp Limine/*.EFI limine_isofiles/EFI/BOOT
	rm -rf Limine


superclean: clean
	rm -rf limine_isofiles
	rm bios.bin

clean:
	cargo clean
