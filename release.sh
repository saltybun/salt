# LINUX_64="x86_64-unknown-linux-gnu"
MAC_64="x86_64-apple-darwin"
MAC_M="aarch64-apple-darwin"
# WIN_64="x86_64-pc-windows-gnu"

# cleanup
rm salt-$MAC_64.zip
rm salt-$MAC_M.zip

# echo "Building for linux"
# TARGET_CC=x86_64-unknown-linux-gnu cargo build -r --target $LINUX_64
# strip target/$LINUX_64/release/salt

echo "Building for intel macs"
cargo build -r --target $MAC_64
strip target/$MAC_64/release/salt

echo "Building for M series macs"
cargo build -r --target $MAC_M
strip target/$MAC_M/release/salt

# echo "Building for Windows"
# cargo build -r --target $WIN_64

# zip ./salt-$LINUX_64.zip target/$LINUX_64/release/salt
zip ./salt-$MAC_64.zip target/$MAC_64/release/salt
zip ./salt-$MAC_M.zip target/$MAC_M/release/salt
# zip ./salt-$WIN_64.zip target/$WIN_64/release/salt