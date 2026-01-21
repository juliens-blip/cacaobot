// Build script for compiling Protobuf definitions
// This generates Rust code from cTrader Open API .proto files

use std::io::Result;

fn main() -> Result<()> {
    // Compile cTrader Protobuf definitions
    // Note: proto files should be placed in proto/ directory
    
    let proto_files = &["proto/ctrader.proto"];
    
    // Check if proto files exist before compiling
    let proto_exists = std::path::Path::new("proto/ctrader.proto").exists();
    
    if proto_exists {
        prost_build::compile_protos(proto_files, &["proto/"])?;
        println!("cargo:rerun-if-changed=proto/ctrader.proto");
    } else {
        // If proto doesn't exist, create a stub to allow compilation
        println!("cargo:warning=proto/ctrader.proto not found - using stub implementation");
        println!("cargo:warning=Download cTrader proto files from: https://github.com/spotware/OpenApiProto");
    }
    
    Ok(())
}
