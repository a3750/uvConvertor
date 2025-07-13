# uvconvertor

## 介绍

这个项目是用 Rust 语言重新实现的 uvconvertor，源项目来自 [vankubo/uvConvertor](https://github.com/vankubo/uvConvertor.git)，并且此项目增加了许多额外功能。它可以解析一个或多个 Keil MDK 项目的工程文件 .uvprojx 以及该项目生成的 .dep 文件，读取其中源文件的编译命令，然后将每个项目的编译命令组合起来，生成一个 compile_commands.json 文件。这个文件可以让 Keil MDK 的项目被 clangd 索引，从而帮助支持语言服务协议的编辑器（如 VSCode、NeoVim、CLion）实现代码提示和代码错误诊断等功能。

## 安装

从 cargo 安装
```bash
$ cargo install uvconvertor
```

从 homebrew 安装
```bash
$ brew tap J3750/homebrew-tools git@gitee.com:J3750/homebrew-tools.git
$ brew update
$ brew install uvconvertor
```

从 scoop 安装
```bash
$ scoop bucket add J3750 git@gitee.com:J3750/scoop-bucket.git
$ scoop update
$ scoop install uvconvertor
```

## 从源码构建

```bash
$ git clone git@gitee.com:J3750/uvconvertor.git
$ cd uvconvertor
$ cargo build
$ cargo run -- -f <path to .uvprojx>[:<target name>] -o <output directory of compile_commands.json>
```

## 用法

```bash
$ uvconvertor --file <FILE_WITH_TARGETS>... \
              [--output <OUTPUT_DIRECTORY>] \
              [--extopts <ARGS>...] \
              [--rmopts <ARGS>...] \
              [--pattern <PATTERN>] \
              [--no-sysinc]
```

## 参数选项

```
-f, --file <FILE_WITH_TARGETS>...
    One or more input uvprojx files with names of targets, such as "--file 1.uvprojx 2.uvprojx,t1,t2"

-o, --output <OUTPUT_DIRECTORY>
    The directory where the output file compile_commands.json is located, default export to stdout

-e, --extopts [<ARG>...]
    Additional arguments to include, e.g. "--extopts=-I/path/to/include,-std=c11"

-r, --rmopts [<ARG>...]
    arguments to remove

-p, --pattern <PATTERN>
    This pattern will be used to replace disk icon of all absolute paths
    e.g. pattern "/mnt/$disk" can replace "C:/.../incldue" to "/mnt/c/.../incldue"

-n, --no-sysinc
    Remove sysroot include path from compile commands
```
