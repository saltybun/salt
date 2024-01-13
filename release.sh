VERSION=v0.1.2
# LINUX_64="x86_64-unknown-linux-gnu"
# MAC_64="x86_64-apple-darwin"
# MAC_M="aarch64-apple-darwin"
WIN_64="x86_64-pc-windows-gnu"

# cleanup
rm salt-*.zip

# echo "Building for linux"
# CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc \
# cargo build --target=x86_64-unknown-linux-gnu
# strip target/$LINUX_64/release/salt

# echo "Building for intel macs"
# cargo build -r --target $MAC_64
# strip target/$MAC_64/release/salt

# echo "Building for M series macs"
# cargo build -r --target $MAC_M
# strip target/$MAC_M/release/salt

# echo "Building for Windows"
cargo build -r --target $WIN_64

# zip ./salt-$LINUX_64.zip target/$LINUX_64/release/salt
# zip ./salt-$MAC_64-$VERSION.zip target/$MAC_64/release/salt
# zip ./salt-$MAC_M-$VERSION.zip target/$MAC_M/release/salt
# zip ./salt-$WIN_64.zip target/$WIN_64/release/salt