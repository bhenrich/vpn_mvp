fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure a bundled `protoc` is available on all platforms
    if let Ok(protoc_path) = protoc_bin_vendored::protoc_bin_path() {
        std::env::set_var("PROTOC", protoc_path);
    }
    // Compile our local copy of the directory service proto for the client stubs.
    tonic_build::configure()
        .build_server(false)
        .compile(&["protos/directory.proto"], &["protos"])?;
    Ok(())
}
