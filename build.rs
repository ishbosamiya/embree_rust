extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn pre_compiled_lib_exists() -> bool {
    let embree_loc = PathBuf::from("./deps/embree3");
    embree_loc.exists()
}

fn get_target_os() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "macos")]
    {
        "macosx"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        panic!("Precompiled Embree available only for linux, macos and windows")
    }
}

fn get_embree_precompiled_url(version: &str, target_os: &str) -> String {
    if target_os == "linux" {
        format!("https://github.com/embree/embree/releases/download/v{version}/embree-{version}.x86_64.{target_os}.tar.gz", version = version, target_os = target_os)
    } else if target_os == "macosx" {
        format!("https://github.com/embree/embree/releases/download/v{version}/embree-{version}.x86_64.{target_os}.zip", version = version, target_os = target_os)
    } else if target_os == "windows" {
        format!("https://github.com/embree/embree/releases/download/v{version}/embree-{version}.x64.vc14.{target_os}.zip", version = version, target_os = target_os)
    } else {
        unreachable!("unsupported OS")
    }
}

fn get_embree_lib(version: &str) {
    let temp_dir_path = PathBuf::from("./deps/temp");

    std::fs::create_dir_all(&temp_dir_path).unwrap_or(());

    let target_os = get_target_os();

    // get the compiled library from github if it does not exist
    // already
    if temp_dir_path.read_dir().unwrap().count() == 0 {
        let url = get_embree_precompiled_url(version, target_os);
        println!(
            "precompiled zipped library not found, downloading from {}",
            url
        );
        std::process::Command::new("wget")
            .current_dir(&temp_dir_path)
            .arg(url)
            .output()
            .expect("enable to spawn wget");
    }

    assert_eq!(temp_dir_path.read_dir().unwrap().count(), 1);

    // extract the downloaded file
    temp_dir_path.read_dir().unwrap().for_each(|zipped_file| {
        let path = zipped_file.unwrap().path();
        if path.extension().unwrap() == "gz" {
            println!("trying to extract .tar.gz file");
        } else if path.extension().unwrap() == "zip" {
            println!("trying to extract .zip file");
        } else {
            unreachable!("the downloaded file should be .tar.gz or .zip")
        }
        std::process::Command::new("tar")
            .arg("-xf")
            .arg(path.to_str().unwrap())
            .arg("--directory")
            .arg("./deps")
            .output()
            .expect("enable to spawn tar");
    });

    let deps_path = temp_dir_path.parent().unwrap();

    assert_eq!(deps_path.read_dir().unwrap().count(), 2);

    // rename the extracted directory to embree3
    deps_path
        .read_dir()
        .unwrap()
        .filter(|path| path.as_ref().unwrap().file_name() != "temp")
        .for_each(|path| {
            let path = path.unwrap().path();
            let mut embree3_path = deps_path.to_path_buf();
            embree3_path.push("embree3");
            std::fs::rename(path, embree3_path).unwrap();
        });

    // delete the temp directory
    std::fs::remove_dir_all(temp_dir_path).unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    if pre_compiled_lib_exists() {
        println!("pre compiled embree already exists at deps/embree3");
    } else {
        get_embree_lib("3.13.2");
    }

    println!("cargo:rustc-link-lib=embree3");
    println!("cargo:rustc-link-lib=tbb");
    println!("cargo:rustc-link-search=deps/embree3/lib");
    // TODO: need to test for cross compilation, it may work only on
    // linux, see
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#dynamic-library-paths
    // for more info
    //
    // TODO: need to also make sure that it does not overwrite the
    // existing environment variable
    //
    // TODO: if crate is built and then run using
    // `./target/debug/xyz`, it will fail because the environment
    // variable is not set. There does not seem to be a good way to
    // handle this, see https://github.com/rust-lang/cargo/issues/4895
    // for more details.
    println!("cargo:rustc-env=LD_LIBRARY_PATH=deps/embree3/lib");

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
