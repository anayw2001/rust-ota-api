use protobuf_codegen::Customize;

fn main() {
    protobuf_codegen::Codegen::new()
        .protoc()
        // All inputs and imports from the inputs must reside in `includes` directories.
        .includes(["proto"])
        // Inputs must reside in some of include paths.
        .input("proto/ota_metadata.proto")
        .customize(Customize::default().gen_mod_rs(true))
        .cargo_out_dir("proto")
        .run_from_script()
}