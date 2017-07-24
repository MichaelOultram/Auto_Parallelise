#!/usr/bin/bash
cd "${0%/*}" # cd into current repo

# Download redox
#git submodule update --init
cd redox

# Remove kernel from redox
git rm kernel

# Make a link to kernel-distro
mkdir kernel
sudo mount --bind $(pwd)/../kernel-distro $(pwd)/kernel

# Download the rest of redox
git submodule update --init --recursive
