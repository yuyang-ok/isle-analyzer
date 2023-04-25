# isle-analyzer

This is a `IDE` support for ISLE language.
<https://github.com/bytecodealliance/wasmtime/tree/main/cranelift/isle>

## Right now below features has supported

* go to definition
* go to references
* hover
* highlight
* rename
* inlay hints
* auto completion
* diagnose information

## How to use


install plugin from marketplace.

<https://marketplace.visualstudio.com/items?itemName=isle-analyzer.isle-analyzer>

install LSP server via
~~~
cargo install --git  https://github.com/yuyang-ok/isle-analyzer isle-analyzer
~~~


## RoadMap
 + formatter


## changelog 2023-4-22

Modify the way that `isle-analyzer` load project.
you specify list of file `isle-analyzer` should load like blow.
~~~
in .../.vscode/settings.json

{
    ...
    "isle-analyzer.files": [
        "/home/yuyang/projects/wasmtime/cranelift/codegen/src/isa/riscv64/inst.isle",
        "/home/yuyang/projects/wasmtime/cranelift/codegen/src/isa/riscv64/lower.isle",
        "/home/yuyang/projects/wasmtime/cranelift/codegen/src/prelude.isle",
        "/home/yuyang/projects/wasmtime/cranelift/codegen/src/prelude_lower.isle",
    ]
    ...
}
~~~