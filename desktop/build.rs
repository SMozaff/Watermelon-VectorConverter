// Embeds the Watermelon icon into wvgc-desktop.exe itself (what Explorer
// and the taskbar show before the window opens), separate from the iced
// window icon set at runtime in src/app/mod.rs.
fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../icons/watermelon.ico");
        res.compile()
            .expect("failed to embed Windows icon resource");
    }
}
