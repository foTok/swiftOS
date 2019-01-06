# swiftOS
This project has servel different components.
## 1. boot_loader
This component implemented a boot_loader to initialize Pi,
receive kernel and start the kernel.

It uses some driver from component pi and basic elements from std.
(not the std offered by Rust)
## 2. firmware
Firmware from official site.

## 3. kernel
The kernel of the OS. This component uses pi and std, like the boot_loder.

## 4. pi
The driver of hardware. Now we have {gpio, timer, uart}. This component
use std.

## 5. std
Our own std lib. Some containers and protocals.

## 6. ttywrite
The software which operates on linux host and send the kernel to Pi by uart.

```
+-------------+        +--------+    +----------+    +----------+
| boot_loader |        | kernel |    | firmware |    | ttywrite |
+-+-----------+        +------+-+    +----------+    +----------+
  ^                           ^
  |                           |
  |                           |
  |                           |
  |  +----------------------+ |
  |  | +----+       +-----+ | |
  +--+ | Pi +<------+ std | +-+
     | +----+       +-----+ |
     +----------------------+
```
