use std::process::Command;

fn main() {
    let cmd = |command: &str, args: &[&str]| {
        let mut child = Command::new(command)
            .args(args)
            .spawn()
            .unwrap_or_else(|_| panic!("failed to call {command}"));
        let result = child
            .wait()
            .unwrap_or_else(|_| panic!("failed to wait for {command}"));
        if !result.success() {
            panic!("{command} failed");
        }
    };

    cmd(
        "cargo",
        &[
            "build",
            "--profile=wasm",
            "-Zbuild-std=panic_abort,std",
            "-Zbuild-std-features=panic_immediate_abort",
            "--target=wasm32-unknown-unknown",
            "-p=lo-web",
        ],
    );
    cmd(
        "wasm-opt",
        &[
            "./target/wasm32-unknown-unknown/wasm/lo_web.wasm",
            "-O3",
            "-o",
            "./target/wasm32-unknown-unknown/wasm/lo_web.opt.wasm",
        ],
    );

    println!("ok");
}
