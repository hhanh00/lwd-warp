fn main() {
    tonic_build::configure()
        .out_dir("src")
        .file_descriptor_set_path("src/cash.z.wallet.sdk.rpc.bin")
        .compile(
            &["proto/service.proto", "proto/compact_formats.proto"],
            &["proto"],
        )
        .unwrap();
}
