# w600_rust

* **w600_rust_example** W600程序代码
* **websocket-server** WebSocket服务器代码
* **wx-app** 小程序代码

## 编译W600 SDK

* W600官网资料下载页面：[http://www.winnermicro.com/html/1/156/158/497.html](http://www.winnermicro.com/html/1/156/158/497.html)
* 下载Eclipse IDE 和 W600 SDK：[w60x_ide_setup_r1.2.zip](http://www.winnermicro.com/upload/1/editor/1558668629957.zip) & [WM_SDK_W60X_G3.04.00_2003.zip](http://www.winnermicro.com/upload/1/editor/1584693446529.zip)
* GNU Arm工具链下载(2019-q4)：[https://developer.arm.com/tools-and-software/open-source-software/developer-tools/gnu-toolchain/gnu-rm/downloads#panel2a](https://developer.arm.com/tools-and-software/open-source-software/developer-tools/gnu-toolchain/gnu-rm/downloads#panel2a) 下载后把bin目录配置到环境变量
* 打开W60X_IDE，选择“文件->导入->C->Existing Code as Makefile Project”
* 源码修改
    >wm_sockets2.0.3.h中，第472行，fcntl 改为 fcntl2，修改其他调用此函数的代码。

    >libemqtt.h中，第35行和第40行，修改mqtt用户名、密码长度，不小于实际使用的长度。

* 右键点击项目名称，选择“构建配置->全部构建”，在项目文件夹内会生成对应的.so文件和.a文件
* 使用命令 arm-none-eabi-gcc-ar crv .\startup_ARMCM3.a .\startup_ARMCM3.o 把startup_ARMCM3.o转换为startup_ARMCM3.a
* 对应的文件复制到w600_rust_example\w60x\lib文件夹中，SDK目录中的seboot.img放到 w600_rust_example\w60x\bin目录中
* 把GNU Tools Arm Embedded\9 2019-q4-major\arm-none-eabi\lib\thumb\v7-m\nofp\libnosys.a 也放到 lib中
* W60X_IDE启动以后在启动界面中选择最后一项“查看文档”，有更多说明。

## 烧写TB-01开发板程序

* 安装Rust，切换到nightly版本，添加编译目标
```bat
rustup default nightly
rustup target add
```
* 修改w600_rust_example\src\main.rs中的wifi名、密码、本机IP地址
* 进入w600_rust_example，执行 build.bat 进行编译
```bat
D:\w600_rust\w600_rust_example>build.bat

D:\w600_rust\w600_rust_example>cargo build --release
   Compiling w600_rust_example v0.0.1 (D:\w600_rust\w600_rust_example) 
    Finished release [optimized] target(s) in 3.15s
```
* 把TB-01 开发板连接到电脑，找到端口号，修改flash.bat中最后一行COM6
* 运行 flash.bat 烧写
```bat
D:\w600_rust\w600_rust_example>flash.bat

D:\w600_rust\w600_rust_example>echo off
secboot_len:6adc, app_imglen:721b4, total:801b4
need to download ...
opend COM6 !

start connect device
timeout, try to reset deviceCCC
sync success,
change baud to 2000000!
100% sent
All done.
```
* 用串口查看器打开相应端口，查看TB-01串口输出信息。

## 启动Websocket服务器

* 启动服务器后，看到日志 "clientx11 registered."说明TB-01已经连接了服务器。

```bat
D:\w600_rust\websocket-server>cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.28s
     Running `target\debug\websocket-server.exe`
Listening on: Ok(V4(0.0.0.0:9001))
Peer address: 192.168.1.3:49848
New WebSocket connection: 192.168.1.3:49848
Text("clientx11")
clientx11 registered.

```

## 运行微信小程序

* 下载微信开发者工具稳定版，启动以后导入项目，选择wx-app文件夹
* 打开项目后，打开右上角“详情”菜单，勾选“不校验和法域名”这一项
* 重新启动后，在预览界面中即可连接WebSocket服务给TB-01发送信息