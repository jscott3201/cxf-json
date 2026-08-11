use std::env;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(fuzzing)");
    println!("cargo::rustc-check-cfg=cfg(cxf_json_semantic_harness)");
    println!("cargo::rerun-if-env-changed=CXF_JSON_SEMANTIC_HARNESS");

    match env::var("CXF_JSON_SEMANTIC_HARNESS") {
        Err(env::VarError::NotPresent) => {}
        Ok(value) if value == "1" => println!("cargo::rustc-cfg=cxf_json_semantic_harness"),
        Ok(_) | Err(env::VarError::NotUnicode(_)) => {
            panic!("CXF_JSON_SEMANTIC_HARNESS must be unset or equal to 1")
        }
    }
}
