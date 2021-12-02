extern crate bindgen;

use std::env;
use std::path::{Path, PathBuf};

fn pre_compiled_lib_exists() -> bool {
    let embree_loc = PathBuf::from("./deps/embree3");
    embree_loc.exists()
}

/// [`source_dir`] is embree source code path
///
/// [`build_dir`] is path to which embree is compiled, generally
/// `source_dir/build`
///
/// [`to_dir`] is the path to which embree is installed
fn compile_embree(
    source_dir: impl AsRef<Path>,
    build_dir: impl AsRef<Path>,
    to_dir: impl AsRef<Path>,
) {
    std::process::Command::new("cmake")
        .current_dir(&build_dir)
        .arg("CMAKE_BUILD_TYPE=Release")
        .arg("-DEMBREE_ISPC_SUPPORT=false")
        .arg("-DEMBREE_TUTORIALS=false")
        .arg("-DEMBREE_STATIC_LIB=true")
        .arg(source_dir.as_ref())
        .output()
        .expect("cmake may not be available on system");

    // TODO: user customizable number of processes, embree is
    // expensive to compile, completely utilizes the CPU, RAM and SWAP
    // thus bringing the system to complete halt (at least on a XPS 15
    // 9570 with i7-8750H and 16GB RAM)
    std::process::Command::new("make")
        .current_dir(&build_dir)
        .arg("-j")
        .arg("6")
        .output()
        .expect("make may not be available on system");

    std::process::Command::new("cmake")
        .current_dir(&build_dir)
        .arg("--install")
        .arg(".")
        .arg("--prefix")
        .arg(to_dir.as_ref())
        .output()
        .expect("failed to install the library");
}

fn generate_embree_lib() {
    let root_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .canonicalize()
        .unwrap();

    let embree_source_dir = {
        let mut embree_source_dir = root_dir.clone();
        embree_source_dir.push("extern/embree");
        embree_source_dir.canonicalize().unwrap()
    };

    let embree_build_dir = {
        let mut embree_build_dir = embree_source_dir.clone();
        embree_build_dir.push("build");

        if !embree_build_dir.exists() {
            std::fs::create_dir_all(&embree_build_dir)
                .expect("could not create build dir for Embree");
        }

        embree_build_dir.canonicalize().unwrap()
    };

    let embree_deps_dir = {
        let mut embree_deps_dir = root_dir;
        embree_deps_dir.push("deps/embree3");

        if !embree_deps_dir.exists() {
            std::fs::create_dir_all(&embree_deps_dir)
                .expect("could not create deps dir for Embree");
        }

        embree_deps_dir.canonicalize().unwrap()
    };

    compile_embree(&embree_source_dir, &embree_build_dir, &embree_deps_dir);

    std::fs::remove_dir_all(embree_build_dir).unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=deps/embree3/");

    if pre_compiled_lib_exists() {
        println!("pre compiled embree already exists at deps/embree3");
    } else {
        generate_embree_lib();
    }

    println!("cargo:rustc-link-lib=dylib=stdc++");
    println!("cargo:rustc-link-lib=static=embree3");
    println!("cargo:rustc-link-lib=static=embree_sse42");
    println!("cargo:rustc-link-lib=static=embree_avx");
    println!("cargo:rustc-link-lib=static=embree_avx2");
    println!("cargo:rustc-link-lib=static=embree_avx512");
    println!("cargo:rustc-link-lib=static=lexers");
    println!("cargo:rustc-link-lib=static=math");
    println!("cargo:rustc-link-lib=static=simd");
    println!("cargo:rustc-link-lib=static=sys");
    println!("cargo:rustc-link-lib=static=tasking");
    println!("cargo:rustc-link-lib=dylib=tbb");

    let current_dir = std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let embree_lib_path = {
        let mut embree_lib_path = current_dir;
        embree_lib_path.push("deps/embree3/lib");
        embree_lib_path
    };
    println!(
        "cargo:rustc-link-search={}",
        embree_lib_path.canonicalize().unwrap().to_str().unwrap()
    );

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .allowlist_type("RTC.*")
        .allowlist_function("rtc.*")
        .no_copy("RTC.*")
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
