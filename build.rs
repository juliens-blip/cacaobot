// Build script for compiling Protobuf definitions
// This generates Rust code from cTrader Open API .proto files

use std::io::Result;

fn main() -> Result<()> {
    // Compile cTrader Protobuf definitions
    // Note: proto files should be placed in proto/ directory
    
    let proto_files = &[
        "proto/OpenApiCommonMessages.proto",
        "proto/OpenApiCommonModelMessages.proto",
        "proto/OpenApiMessages.proto",
        "proto/OpenApiModelMessages.proto",
    ];
    
    // Compile all cTrader Open API protobuf files
    prost_build::compile_protos(proto_files, &["proto/"])?;
    
    // Re-run build if any proto file changes
    for file in proto_files {
        println!("cargo:rerun-if-changed={}", file);
    }
    
    Ok(())
}
