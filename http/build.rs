use rustc_version::{Result,version};

fn main() -> Result<()> {
    println!("cargo:rustc-env=RUSTC_VERSION={}", version()?);
    Ok(())
}
