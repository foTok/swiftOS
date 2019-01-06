#! /bin/bash
ttywrite="./ttywrite/target/release/ttywrite"
kernel="./kernel/build/kernel.bin"
tty_path="/dev/ttyUSB0"

$ttywrite "-i" $kernel $tty_path
