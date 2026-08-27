use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_SHA");
    println!("cargo:rerun-if-changed=.git/HEAD");

    let build_sha = env::var("BUILD_SHA")
        .ok()
        .filter(|value| !value.is_empty() && value != "development")
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|value| value.trim().to_owned())
        })
        .unwrap_or_else(|| "unversioned-build".to_owned());

    println!("cargo:rustc-env=BUILD_SHA={build_sha}");
}
