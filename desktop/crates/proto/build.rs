//! Build script compiling proto/tandem/v1/*.proto with prost-build into Rust
//! modules. The repo-root proto directory is the only schema source; no vendored
//! copies.

use std::path::PathBuf;

const PROTO_FILES: &[&str] = &[
    "tandem/v1/common.proto",
    "tandem/v1/call.proto",
    "tandem/v1/calllog.proto",
    "tandem/v1/pairing.proto",
    "tandem/v1/transport.proto",
];

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("crate sits at <repo>/desktop/crates/proto");
    let proto_root = repo_root.join("proto");
    assert!(
        proto_root.is_dir(),
        "repo-root proto directory not found at {}",
        proto_root.display()
    );

    std::env::set_var(
        "PROTOC",
        protoc_bin_vendored::protoc_bin_path().expect("vendored protoc must be available"),
    );

    let inputs: Vec<PathBuf> = PROTO_FILES.iter().map(|f| proto_root.join(f)).collect();
    for input in &inputs {
        println!("cargo:rerun-if-changed={}", input.display());
    }

    prost_build::compile_protos(&inputs, &[&proto_root]).expect("TLP schema must compile");
}
