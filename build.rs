use std::{env, fs, path::PathBuf, process::Command};

fn git_dir() -> Option<PathBuf> {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| PathBuf::from(value.trim()))
}

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_SHA");
    if let Some(directory) = git_dir() {
        let head = directory.join("HEAD");
        println!("cargo:rerun-if-changed={}", head.display());
        // On a branch, HEAD itself only contains the ref name. Watch the ref file as
        // well so a normal `git commit` recompiles the embedded health identity.
        if let Ok(contents) = fs::read_to_string(&head) {
            if let Some(reference) = contents.trim().strip_prefix("ref: ") {
                println!("cargo:rerun-if-changed={}", directory.join(reference).display());
            }
        }
    }

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
