#!/bin/bash
set -euo pipefail

# Script to sign and repackage binaries
# Usage: ./scripts/sign-and-repackage.sh <tarball> <platform>

if [ $# -ne 2 ]; then
	echo "Usage: $0 <tarball> <platform>"
	echo "Supported platforms: windows, macos"
	exit 1
fi

tarball=$1
platform=$2

# Map platform to binary name and signer command
case "$platform" in
"windows")
	binary_name="gkg.exe"
	signer_cmd="sign-windows-binaries"
	;;
"macos")
	binary_name="gkg"
	signer_cmd="sign-macos-binaries"
	;;
*)
	echo "Error: Unsupported platform '$platform'"
	echo "Supported platforms: windows, macos"
	exit 1
	;;
esac

mkdir -p work

# Extract, sign, and repackage
echo "Extracting ${tarball}..."
tar -xzvf "${tarball}" -C work
"${signer_cmd}" "work/${binary_name}"
rm -f work/${binary_name}.unsigned

echo "Repackaging ${tarball}..."
tar -czvf "${tarball}" -C work .

rm -rf work

echo "Completed signing ${tarball}"
