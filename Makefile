test:
	cargo test --all -- --color always --nocapture

expensive-demo:
	RUST_TEST_THREADS=1 cargo test demo --release -- --color always --nocapture --ignored

expensive-256:
	cargo test demo_256 --release -- --color always --nocapture --ignored

expensive-512:
	cargo test demo_512 --release -- --color always --nocapture --ignored

expensive-256-32bit:
	cargo test demo_256_32bit --release -- --color always --nocapture --ignored

expensive-512-32bit:
	cargo test demo_512_32bit --release -- --color always --nocapture --ignored

expensive-32bit:
	RUST_TEST_THREADS=1 cargo test 32bit --release -- --color always --nocapture --ignored

expensive-games:
	cargo test games_com --release -- --color always --nocapture --ignored

expensive-fuzz:
	cargo test fuzz --package dustbox_fuzzer -- --color always --nocapture --ignored

expensive-encode:
	cargo test encode -- --color always --nocapture --ignored

bench:
	cargo bench --all

mips:
	cargo test --release mips -- --nocapture

run:
	cargo run --package dustbox_debugger

run-release:
	cargo run --release --package dustbox_gtk

lint:
	rustup run nightly cargo clippy --all

prober:
	cd utils/prober && make

typos:
	speller . > spell

glade:
	glade debugger/src/interface.glade

bindiff:
	vbindiff ~/dosbox-x/MEMDUMP.BIN emu_mem.bin
