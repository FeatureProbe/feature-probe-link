use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = ["protos/channel_service.proto", "protos/conn_packet.proto"];
    let out_dir = env::var("OUT_DIR").unwrap();
    for proto_file in proto_files.iter() {
        tonic_build::compile_protos(proto_file)?;
    }

    let mod_file_content = proto_files
        .iter()
        .map(|proto_file| {
            let proto_path = Path::new(proto_file);
            let mod_name = proto_path
                .file_stem()
                .expect("Unable to extract stem")
                .to_str()
                .expect("Unable to extract filename");
            format!(
                "pub mod {} {{ tonic::include_proto!(\"halo.{}\"); }}",
                mod_name, mod_name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut mod_file = File::create(Path::new(&out_dir).join("mod.rs")).unwrap();
    mod_file
        .write_all(mod_file_content.as_bytes())
        .expect("Unable to write mod file");

    Ok(())
}
