all:
	nasm -f bin -o prober.com prober.asm
	cp prober.com ~/dosbox-x
	cp prober.com ~/vm-share
	ndisasm -o 0x100 prober.com | head -n 10
