use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf}
};
use bindgen::{Bindings, BindgenError};

// SUNDIALS has a few non-negative constants that need to be parsed as an i32.
// This is an attempt at doing so generally.
#[derive(Debug)]
struct ParseSignedConstants;

impl bindgen::callbacks::ParseCallbacks for ParseSignedConstants {
    fn int_macro(&self, name: &str, _value: i64) -> Option<bindgen::callbacks::IntKind> {
        let prefix: String = name.chars().take_while(|c| *c != '_').collect();
        match prefix.as_ref() {
            "CV" | "IDA" | "KIN" | "SUN" => Some(bindgen::callbacks::IntKind::Int),
            _ => None,
        }
    }
}

// Ignore some macros (based on https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-1312298570)
#[derive(Debug)]
struct IgnoreMacros(HashSet<&'static str>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        use bindgen::callbacks::MacroParsingBehavior;
        if self.0.contains(name) {
            MacroParsingBehavior::Ignore
        } else {
            MacroParsingBehavior::Default
        }
    }
}

impl IgnoreMacros {
    const IGNORE_CONSTANTS: [&'static str; 19] = [
        "FE_DIVBYZERO",
        "FE_DOWNWARD",
        "FE_INEXACT",
        "FE_INVALID",
        "FE_OVERFLOW",
        "FE_TONEAREST",
        "FE_TOWARDZERO",
        "FE_UNDERFLOW",
        "FE_UPWARD",
        "FP_INFINITE",
        "FP_INT_DOWNWARD",
        "FP_INT_TONEAREST",
        "FP_INT_TONEARESTFROMZERO",
        "FP_INT_TOWARDZERO",
        "FP_INT_UPWARD",
        "FP_NAN",
        "FP_NORMAL",
        "FP_SUBNORMAL",
        "FP_ZERO",
    ];

    fn new() -> Self {
        Self(Self::IGNORE_CONSTANTS.iter().copied().collect())
    }
}

#[derive(Debug)]
struct Library {
    /// Location if the include files.
    inc: Option<String>,
    /// Location of the library.
    lib: Option<String>,
}

/// Build the Sundials code vendor with sundials-sys.
fn build_vendor_sundials(klu: &Library) -> (Library, &'static str) {
    macro_rules! feature {
        ($s:tt) => {
            if cfg!(feature = $s) {
                "ON"
            } else {
                "OFF"
            }
        };
    }

    let static_libraries = feature!("static_libraries");
    let (shared_libraries, library_type) = match static_libraries {
        "ON" => ("OFF", "static"),
        "OFF" => ("ON", "dylib"),
        _ => unreachable!(),
    };

    let mut config = cmake::Config::new("vendor");
    config
        .define("CMAKE_INSTALL_LIBDIR", "lib")
        .define("BUILD_STATIC_LIBS", static_libraries)
        .define("BUILD_SHARED_LIBS", shared_libraries)
        .define("BUILD_TESTING", "OFF")
        .define("EXAMPLES_INSTALL", "OFF")
        .define("EXAMPLES_ENABLE_C", "OFF")
        .define("BUILD_ARKODE", feature!("arkode"))
        .define("BUILD_CVODE", feature!("cvode"))
        .define("BUILD_CVODES", feature!("cvodes"))
        .define("BUILD_IDA", feature!("ida"))
        .define("BUILD_IDAS", feature!("idas"))
        .define("BUILD_KINSOL", feature!("kinsol"))
		.define("ENABLE_KLU", feature!("klu"))
        .define("OPENMP_ENABLE", feature!("nvecopenmp"))
        .define("PTHREAD_ENABLE", feature!("nvecpthreads"));
    if let Some(inc) = &klu.inc {
        config.define("KLU_INCLUDE_DIR", inc);
    }
    if let Some(lib) = &klu.lib {
        config.define("KLU_LIBRARY_DIR", lib);
    }

    let dst = config.build();
    let dst_disp = dst.display();
    let lib_loc = Some(format!("{}/lib", dst_disp));
    let inc_dir = Some(format!("{}/include", dst_disp));
    (Library { inc: inc_dir, lib: lib_loc }, library_type)
}

fn generate_bindings(inc_dirs: &[Option<String>]) -> Result<Bindings, BindgenError>
{
    macro_rules! define {
        ($a:tt, $b:tt) => {
            format!(
                "-DUSE_{}={}",
                stringify!($b),
                if cfg!(feature = $a) { 1 } else { 0 }
            )
        };
    }

    let mut builder = bindgen::Builder::default().header("wrapper.h");
    for dir in inc_dirs {
        if let Some(dir) = dir {
            builder = builder.clang_arg(format!("-I{}", dir))
        }
    }
    builder
        .clang_args(&[
            define!("arkode", ARKODE),
            define!("cvode", CVODE),
            define!("cvodes", CVODES),
            define!("ida", IDA),
            define!("idas", IDAS),
            define!("kinsol", KINSOL),
            define!("klu", KLU),
            define!("nvecopenmp", OPENMP),
            define!("nvecpthreads", PTHREADS),
        ])
        .parse_callbacks(Box::new(ParseSignedConstants))
        .parse_callbacks(Box::new(IgnoreMacros::new()))
        .generate()
}

fn get_sundials_version_major(bindings: impl AsRef<Path>) -> Option<u32> {
    let b = File::open(bindings).expect("Couldn't read file bindings.rs!");
    let mut b = BufReader::new(b).bytes();
    'version:
    while b.find(|c| c.as_ref().is_ok_and(|&c| c == b'S')).is_some() {
        for c0 in "UNDIALS_VERSION_MAJOR".bytes() {
            match b.next() {
                Some(Ok(c)) => {
                    if c != c0 {
                        continue 'version
                    }
                }
                Some(Err(_)) | None => return None
            }
        }
        // Match " : u32 = 6"
        if b.find(|c| c.as_ref().is_ok_and(|&c| c == b'=')).is_some() {
            let is_not_digit = |c: &u8| !c.is_ascii_digit();
            let b = b.skip_while(|c| c.as_ref().is_ok_and(is_not_digit));
            let v: Vec<_> =
                b.map_while(|c| c.ok().filter(|c| c.is_ascii_digit()))
                .collect();
            match String::from_utf8(v) {
                Ok(v) => return v.parse().ok(),
                Err(_) => return None
            }
        }
        return None
    }
    None
}

fn main() {
    // First, we build the SUNDIALS library, with requested modules with CMake

    let klu_inc = if let Ok(root) = env::var("DEP_SUITESPARSE_ROOT") {
        Some(format!("{}/include/suitesparse", root))
    } else {
        None
    };
    let klu_lib = if let Ok(root) = env::var("DEP_SUITESPARSE_ROOT") {
        Some(format!("{}/lib", root))
    } else {
        None
    };
    let klu = Library { inc: klu_inc, lib: klu_lib };
    let mut sundials = Library { inc: None, lib: None };
    let mut library_type = "dylib";
    if cfg!(any(feature = "build_libraries", target_family = "wasm")) {
        (sundials, library_type) = build_vendor_sundials(&klu);
    } else {
        sundials.inc = env::var("SUNDIALS_INCLUDE_DIR").ok();
        sundials.lib = env::var("SUNDIALS_LIBRARY_DIR").ok();
    }

    if sundials.lib.is_none() && sundials.inc.is_none() {
        #[cfg(target_family = "windows")] {
            let vcpkg = vcpkg::Config::new()
                .emit_includes(true)
                .find_package("sundials");
            if vcpkg.is_err() {
                (sundials, library_type) = build_vendor_sundials(&klu);
            }
        }
    }

    // Second, we use bindgen to generate the Rust types

    let bindings_rs = PathBuf::from(env::var("OUT_DIR").unwrap())
        .join("bindings.rs");
    let mut build_vendor = true;
    let mut sundials_version_major = 0;
    if let Ok(bindings) = generate_bindings(&[sundials.inc, klu.inc.clone()]) {
        bindings.write_to_file(&bindings_rs)
            .expect("Couldn't write file bindings.rs!");
        if let Some(v) = get_sundials_version_major(&bindings_rs) {
            if v >= 6 {
                build_vendor = false;
                sundials_version_major = v;
            } else {
                println!("cargo:warning=System sundials version = \
                          {} < 6, will use the vendor version", v);
            }
        }
    }
    if build_vendor {
        (sundials, library_type) = build_vendor_sundials(&klu);
        if let Ok(bindings) = generate_bindings(&[sundials.inc, klu.inc.clone()]) {
            bindings
                .write_to_file(&bindings_rs)
                .expect("Couldn't write file bindings.rs!");
            sundials_version_major = get_sundials_version_major(&bindings_rs)
                .expect("Cannot determine vendor sundials version!");
        } else {
            panic!("Unable to generate bindings of the vendor sundials!");
        }
    }
    println!("cargo:rustc-cfg=sundials_version_major=\"{}\"",
        sundials_version_major);

    // Third, we let Cargo know about the library files

    if let Some(dir) = sundials.lib {
        println!("cargo:rustc-link-search=native={}", dir)
    }

    if let Some(dir) = &klu.lib {
        println!("cargo:rustc-link-search=native={}", dir);
    }

    let mut lib_names = vec![
        "nvecserial",
        "sunlinsolband",
        "sunlinsoldense",
        "sunlinsolpcg",
        "sunlinsolspbcgs",
        "sunlinsolspfgmr",
        "sunlinsolspgmr",
        "sunlinsolsptfqmr",
        "sunmatrixband",
        "sunmatrixdense",
        "sunmatrixsparse",
        "sunnonlinsolfixedpoint",
        "sunnonlinsolnewton",
    ];
    if sundials_version_major >= 7 {
        lib_names.push("core");
    }
    macro_rules! link { ($($s:tt),*) => {
        $(if cfg!(feature = $s) { lib_names.push($s) })*
    }}
    link! ("arkode", "cvode", "cvodes", "ida", "idas", "kinsol",
        "nvecopenmp", "nvecpthreads");

    for lib_name in &lib_names {
        println!(
            "cargo:rustc-link-lib={}=sundials_{}",
            library_type, lib_name
        );
    }

    if let Some(dir) = &klu.inc {
        for lib_name in &["sundials_sunlinsolklu", "klu", "suitesparseconfig", "colamd", "klu", "btf", "amd"] {
            println!(
                "cargo:rustc-link-lib={}={}",
                library_type, lib_name
            );
        }
        println!("cargo:klu-include={}", dir)
    }

    // And that's all.
}
