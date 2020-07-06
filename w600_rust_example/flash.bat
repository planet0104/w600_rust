echo off

set NAME=w600_rust_example
set EXEC=target\thumbv7m-none-eabi\release\%NAME%
set BINARY=target\thumbv7m-none-eabi\release\%NAME%.bin
set BINARY_GZ=target\thumbv7m-none-eabi\release\%NAME%.bin.gz
set IMAGE=target\thumbv7m-none-eabi\release\%NAME%.img
set IMAGE_GZ=target\thumbv7m-none-eabi\release\%NAME%_gz.img
set DBG=target\thumbv7m-none-eabi\release\%NAME%.dbg
set FLS=target\thumbv7m-none-eabi\release\%NAME%.fls
set VERSION=target\thumbv7m-none-eabi\release\version.txt

REM make image file
arm-none-eabi-objcopy --output-target=binary -S -g -x -X -R .sbss -R .bss -R .reginfo -R .stack %EXEC% %BINARY%

echo "1.0.06" >> %VERSION%

.\tools\makeimg  %BINARY% %IMAGE% 0 0 %VERSION% 90000 10100

.\tools\wm_gzip.exe  %BINARY%

.\tools\makeimg.exe  %BINARY_GZ% %IMAGE_GZ% 0 1 %VERSION% 90000 10100 %BINARY%

.\tools\makeimg_all.exe ".\w60x\bin\secboot.img" %IMAGE% %FLS%
.\tools\makeimg_dbg.exe %IMAGE% %DBG%

REM flash
.\tools\wm_tools -p COM6 -b 2000000 write_flash %IMAGE_GZ%
