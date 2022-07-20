use rustc_version::{version, Result};

fn main() -> Result<()> {
    println!("cargo:rustc-env=RUSTC_VERSION={}", version()?);
    Ok(())
}
