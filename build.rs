use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_SHA");
    // Container source archives do not contain .git. The factory supplies the
    // immutable source SHA as a build argument; `dev` keeps local builds useful.
    let build_sha = env::var("BUILD_SHA")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "dev".to_owned());

    println!("cargo:rustc-env=BUILD_SHA={build_sha}");
}
