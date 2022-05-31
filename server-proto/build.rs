fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = ["../protos/client.proto", "../protos/service.proto"];

    for proto_file in proto_files.iter() {
        tonic_build::compile_protos(proto_file)?;
    }

    Ok(())
}
