fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os != "windows" {
        return;
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icons/app.ico")
        .set("FileDescription", "poem-rs")
        .set("ProductName", "poem-rs")
        .set("InternalName", "poem-rs.exe")
        .set("OriginalFilename", "poem-rs.exe");

    if let Err(error) = res.compile() {
        panic!("failed to compile Windows resources: {error}");
    }
}
