use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("src/definitions")?;
    tonic_prost_build::configure()
        .out_dir("src/definitions")
        .compile_protos(&["../../proto/parser.proto"], &["../../proto"])?;
    Ok(())
}
