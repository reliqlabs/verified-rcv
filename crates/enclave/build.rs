fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true) // smoke test in src/server.rs uses the generated client
        .compile_protos(&["proto/tally.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/tally.proto");
    Ok(())
}
