fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_path = std::path::Path::new("proto/erp.proto");

    // Try to find well-known types include path
    let include_paths = vec![
        std::path::PathBuf::from("/tmp/protoc/include"),
        std::path::PathBuf::from("/usr/local/include"),
        std::path::PathBuf::from("/usr/include"),
    ];

    let mut found_include = None;
    for p in &include_paths {
        if p.join("google").join("protobuf").join("timestamp.proto").exists() {
            found_include = Some(p.clone());
            break;
        }
    }

    if let Some(include) = found_include {
        tonic_build::configure()
            .compile_protos(&[proto_path], &[include, std::path::PathBuf::from("proto")])?;
    } else {
        tonic_build::compile_protos(proto_path)?;
    }

    Ok(())
}
