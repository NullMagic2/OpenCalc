fn main() {
    println!("cargo:rerun-if-changed=calc95.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("calc95.ico")
        .set("FileDescription", "OpenCalc")
        .set("ProductName", "OpenCalc")
        .set("InternalName", "OpenCalc")
        .set("OriginalFilename", "OpenCalc.exe");
    resource
        .compile()
        .expect("failed to compile the OpenCalc Windows icon/version resource");
}
