fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("src/icon.ico");
        res.set("ProductName", "Zenith Patcher");
        res.set("FileDescription", "Patcher de traduction française pour Undertale Yellow et Red & Yellow");
        res.set("LegalCopyright", "Zénith Team");
        res.set("OriginalFilename", "zenith-patcher-windows.exe");
        let _ = res.compile();
    }
}
