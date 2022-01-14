
fn main() {
    use std::path::PathBuf;
    use std::env;
    println!("cargo:rerun-if-changed=gui_files/gui_v1.fl");
    let g = fl2rust::Generator::default();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    g.in_out("gui_files/gui_v1.fl", out_path.join("gui_v1.rs").to_str().unwrap()).expect("Failed to generate rust from fl file!");
}