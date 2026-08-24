fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Zenith Patcher");
        res.set("FileDescription", "Zenith Patcher - Traduction FR");
        res.set("CompanyName", "Zenith Team");
        res.set("LegalCopyright", "Zenith Team");
        res.set("OriginalFilename", "Zenith-Patcher.exe");
        let _ = res.compile();
    }
}
