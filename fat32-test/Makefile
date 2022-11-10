all:umount fat32


fat32:
	@sudo dd if=/dev/zero of=test.img bs=512 count=102400
	@sudo mkfs.vfat -F 32 ./test.img
	@sudo mount ./test.img /fat

umount:
	@sudo umount /fat

hex:
	@hexdump test.img > test.hex

mount:
	@sudo mount ./test.img /fat