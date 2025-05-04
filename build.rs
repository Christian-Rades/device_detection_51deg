use cc::Build;
use std::env;
use std::fs;
use std::path::PathBuf;

fn scan(build: &mut Build, path: &str, suffix: &str) {
    let paths = fs::read_dir(path).unwrap();

    for path in paths {
        let p = path.unwrap().path();
        let str = p.to_str().unwrap();

        if str.ends_with(suffix) {
            build.file(str);
        }
    }
}

fn main() {
    let mut c = cc::Build::new();
    c.warnings(false);

    scan(&mut c, "./device-detection-cxx/src/", ".c");
    scan(&mut c, "./device-detection-cxx/src/hash/", ".c");
    scan(&mut c, "./device-detection-cxx/src/common-cxx/", ".c");

    c.compile("51degrees");
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .wrap_unsafe_ops(true)
        // The input header we would like to generate
        // bindings for.
        .header("device-detection-cxx/src/hash/fiftyone.h")
        .blocklist_var("FP_.*")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
