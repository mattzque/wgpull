#!/bin/bash
# Builds IPK package for OpenWRT

set -e

PACKAGE="wgpull"
VERSION=$(grep '^version' Cargo.toml | sed -E 's/version = "(.+)"/\1/')
# TODO figure out what arch to use for armv7-unknown-linux-musleabihf rust target
ARCH="all"

BUILD_DIR="target/armv7-unknown-linux-musleabihf/minsize"

# The directory where the .ipk will be created
DEST_DIR="ipk-build"
DATA_DIR="$DEST_DIR/data"
CONTROL_DIR="$DEST_DIR/control"

# Create directories
mkdir -p $DATA_DIR
mkdir -p $CONTROL_DIR

# Copy static files
cp -R package/openwrt/* $DATA_DIR/

# Copy the binaries
cp $BUILD_DIR/wgpull-lighthouse $DATA_DIR/usr/bin/
cp $BUILD_DIR/wgpull-node $DATA_DIR/usr/bin/

# Create the control file
echo "Package: $PACKAGE" > $CONTROL_DIR/control
echo "Version: $VERSION" >> $CONTROL_DIR/control
echo "Architecture: $ARCH" >> $CONTROL_DIR/control
echo "Maintainer: Matthias Hecker <mail@mattzq.com>" >> $CONTROL_DIR/control
echo "Section: base" >> $CONTROL_DIR/control
echo "Priority: optional" >> $CONTROL_DIR/control
echo "Description: Wireguard Configuration Management" >> $CONTROL_DIR/control

pushd $DEST_DIR
echo $(pwd)
tar --owner=root --group=root -czf data.tar.gz -C data/ .
tar --owner=root --group=root -czf control.tar.gz -C control/ .
echo 2.0 > debian-binary
tar -czf $PACKAGE-$VERSION.ipk debian-binary './control.tar.gz' './data.tar.gz'
popd

cp $DEST_DIR/$PACKAGE-$VERSION.ipk package/

rm -Rf ipk-build
