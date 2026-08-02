//! Embeds the brand mark and version strings into `curio.exe` as Windows resources.
//!
//! Without this the binary carries no icon at all, and Windows substitutes its generic
//! executable glyph in four places a user actually looks: Explorer, the taskbar, the Start
//! Menu shortcut the installer creates, and the Add/Remove Programs entry whose `DisplayIcon`
//! points here. The installer having a nice icon does not help any of them — an installer is
//! seen once, and the app is seen every day.
//!
//! The icon is the same `packaging/windows/curio.ico` the installer uses (R-DEL-9), so the
//! mark has one source and cannot drift between the two surfaces (R-OV-2).

fn main() {
    // Cargo re-runs a build script whenever *any* tracked input changes, and declaring the
    // icon keeps a re-rasterised mark from silently not making it into the next build.
    println!("cargo:rerun-if-changed=../../packaging/windows/curio.ico");
    println!("cargo:rerun-if-changed=build.rs");

    // Build scripts compile for the host, so `cfg(windows)` here is the host — which is what
    // gates whether `winresource` was resolved at all. The target check inside is separate
    // and load-bearing: a Windows host cross-compiling to macOS must not staple a PE
    // resource onto a Mach-O binary.
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
            return;
        }

        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../packaging/windows/curio.ico");
        res.set("ProductName", "Curio");
        res.set("FileDescription", "Curio");
        res.set("LegalCopyright", "MIT");

        if let Err(err) = res.compile() {
            // A missing icon is a cosmetic defect, not a reason nobody can build the app.
            // Failing the build here would turn "the .ico was not regenerated" into a hard
            // stop for a contributor who only wanted to run the tests.
            println!("cargo:warning=could not embed Windows resources: {err}");
        }
    }
}
