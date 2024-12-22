build:
	cargo build --release

install: build
	sudo cp target/release/wksp /usr/local/bin/wksp

uninstall:
	sudo rm /usr/local/bin/wksp

clean:
	cargo clean
