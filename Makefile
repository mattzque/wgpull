all: package

.PHONY: package-ipk package-deb

package-ipk:
	rm -f package/*.ipk
	rustup target add armv7-unknown-linux-musleabihf
	CC=/opt/musl/bin/arm-linux-musleabihf-gcc \
	AR=/opt/musl/bin/arm-linux-musleabihf-ar \
	CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER=rust-lld \
		cargo build \
		--target armv7-unknown-linux-musleabihf \
	    --profile minsize
	bash ./create_ipk_package.sh lighthouse
	bash ./create_ipk_package.sh node

package-deb:
	rm -f package/*.deb
	rustup target add x86_64-unknown-linux-musl
	cargo install cargo-deb
	cargo deb --target x86_64-unknown-linux-musl -p wgpull_lighthouse
	cargo deb --target x86_64-unknown-linux-musl -p wgpull_node
	cp target/x86_64-unknown-linux-musl/debian/*.deb package/

package: package-ipk package-deb

clean:
	rm -f package/*.ipk
	rm -f package/*.deb
	cargo clean
