// Embeds the SixNG icon into the Windows executable.
fn main() {
    println!("cargo:rerun-if-changed=../../assets/logo/sixng.ico");
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("../../assets/logo/sixng.ico");
            if let Err(e) = res.compile() {
                // A missing resource compiler only costs the icon, not the build.
                println!("cargo:warning=exe icon not embedded: {e}");
            }
        }
    }
}
