cargo build --release -Zbuild-std="panic_abort,std" --target wasm32-unknown-unknown
D:\MyPrograms\binaryen\wasm-opt.exe .\target\wasm32-unknown-unknown\release\dim_lo_process.wasm -O3 -o dim_lo_process.wasm
Copy-Item -Force dim_lo_process.wasm "D:\DIM\src\app\loadout-builder\process"