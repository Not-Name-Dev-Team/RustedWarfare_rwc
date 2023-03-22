# RustedWarfare_rwc

适用于Rusted Warfare的ini混淆器

## Build

编译前需要配置好rust语言环境

编译：

```sh
make
```

或者

```sh
cargo build --release
```

## Use

```sh
rwc [参数]
```

- 参数列表

  - h (help) 使用帮助
    ``` rwc -h ```

  - i (input) 输入文件路径

  - o (output) 输出路径

  - r (root) 指定rwmod的ROOT路径

  - v (version) 查询rwc版本

- 用例

  - ```rwc -i 输入文件夹路径 -o 输出文件夹路径```
