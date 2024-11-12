bin:	dump attrib fsck label mkfs modfs ctl
dump:
	cargo build --release --bin dumpexfat
attrib:
	cargo build --release --bin exfatattrib
fsck:
	cargo build --release --bin exfatfsck
label:
	cargo build --release --bin exfatlabel
mkfs:
	cargo build --release --bin mkexfatfs
modfs:
	cargo build --release --bin modexfatfs
ctl:
	cargo build --release --bin exfatctl
clean:
	cargo clean --release -p exfat-utils
clean_all:
	cargo clean
fmt:
	cargo fmt
	git status
lint:
	cargo clippy --release --fix --all
	git status
plint:
	cargo clippy --release --fix --all -- -W clippy::pedantic
	git status
test:
	cargo test --release
test_debug:
	cargo test --release -- --nocapture
install:
	cargo install --path .
uninstall:
	cargo uninstall

xxx:	fmt lint test
