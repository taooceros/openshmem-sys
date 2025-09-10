use std::process::Command;
use std::{env, path::PathBuf};
use std::error::Error;
use std::fmt;
/// splits a command line by space and collects all arguments that start with `prefix`
fn collect_args_with_prefix(cmd: &str, prefix: &str) -> Vec<String> {
    shell_words::split(cmd)
        .unwrap()
        .iter()
        .filter_map(|arg| {
            if arg.starts_with(prefix) {
                Some(arg[2..].to_owned())
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, PartialEq)]
struct UnquoteError {
    quote: char,
}


impl UnquoteError {
    fn new(quote: char) -> UnquoteError {
        UnquoteError { quote }
    }
}

impl fmt::Display for UnquoteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Quotes '{}' not closed.", self.quote)
    }
}

impl Error for UnquoteError {}

fn unquote(s: &str) -> Result<String, UnquoteError> {
    if s.chars().count() < 2 {
        return Ok(String::from(s));
    }

    let quote = s.chars().next().unwrap();

    if quote != '"' && quote != '\'' && quote != '`' {
        return Ok(String::from(s));
    }

    if s.chars().last().unwrap() != quote {
        return Err(UnquoteError::new(quote));
    }

    let s = &s[1..s.len() - 1];
    Ok(String::from(s))
}

fn probe_via_oshcc(oshcc: &str) -> std::io::Result<Library> {
    // Capture the output of `mpicc -show`. This usually gives the actual compiler command line
    // invoked by the `mpicc` compiler wrapper.
    Command::new(oshcc).arg("-show").output().map(|cmd| {
        let output = String::from_utf8(cmd.stdout).expect("mpicc output is not valid UTF-8");
        // Collect the libraries that an MPI C program should be linked to...
        let libs = collect_args_with_prefix(output.as_ref(), "-l");
        // ... and the library search directories...
        let libdirs = collect_args_with_prefix(output.as_ref(), "-L")
            .into_iter()
            .filter_map(|x| unquote(&x).ok())
            .map(PathBuf::from)
            .collect();
        // ... and the header search directories.
        let headerdirs = collect_args_with_prefix(output.as_ref(), "-I")
            .into_iter()
            .filter_map(|x| unquote(&x).ok())
            .map(PathBuf::from)
            .collect();

            Library {
                oshcc: Some(oshcc.to_string()),
                libs,
                lib_paths: libdirs,
                include_paths: headerdirs,
                version: String::from("unknown"),
                _priv: (),
            }
    })
}

/// Result of a successfull probe
#[allow(clippy::manual_non_exhaustive)]
#[derive(Clone, Debug)]
pub struct Library {
    /// Path to compiler capable of building MPI programs
    pub oshcc: Option<String>,
    /// Names of the native MPI libraries that need to be linked
    pub libs: Vec<String>,
    /// Search path for native MPI libraries
    pub lib_paths: Vec<PathBuf>,
    /// Search path for C header files
    pub include_paths: Vec<PathBuf>,
    /// The version of the MPI library
    pub version: String,
    _priv: (),
}


fn main() {
    let oshmem = probe_via_oshcc("oshcc").unwrap();

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .clang_args(
            oshmem
                .include_paths
                .iter()
                .map(|p| format!("-I{}", p.to_string_lossy())),
        )
        .header("include/wrapper.h")
        // Tell cargo to invalidate the built ucx_sys whenever any of the
        // included header files changed.
        .prepend_enum_name(false)
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // let cargo knows if wrapper.h is changed
    println!("cargo:rerun-if-changed=include/wrapper.h");

    for path in oshmem.lib_paths {
        println!("cargo:rustc-link-search={}", path.to_string_lossy());
    }
    for path in oshmem.include_paths {
        println!("cargo:include={}", path.to_string_lossy());
    }
    for lib in oshmem.libs {
        println!("cargo:rustc-link-lib={}", lib);
    }

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
