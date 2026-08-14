fn main() {
    println!("cargo:rerun-if-changed=assets/tibetan-ewts-keyboard.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("assets/tibetan-ewts-keyboard.ico")
        .set("FileDescription", "Tibetan EWTS Keyboard")
        .set("ProductName", "Tibetan EWTS Keyboard")
        .set("OriginalFilename", "tibetan-ewts-keyboard.exe");
    resource
        .compile()
        .expect("failed to embed Windows executable resources");
}
