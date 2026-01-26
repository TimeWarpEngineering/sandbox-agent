use std::{fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=../universal-agent-schema/src/lib.rs");

    let schema = schemars::schema_for!(sandbox_agent_universal_agent_schema::UniversalEvent);

    let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(|dir| {
            Path::new(&dir)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .unwrap();
    let out_dir = workspace_root.join("spec");
    fs::create_dir_all(&out_dir).unwrap();

    let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize JSON schema");
    fs::write(out_dir.join("universal-schema.json"), json)
        .expect("Failed to write universal-schema.json");
}
