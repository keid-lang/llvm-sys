extern crate cc;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate semver;

use regex::Regex;
use semver::Version;
use std::env;
use std::ffi::OsStr;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::Command;

// Environment variables that can guide compilation
//
// When adding new ones, they should also be added to main() to force a
// rebuild if they are changed.
lazy_static! {
    /// A single path to search for LLVM in (containing bin/llvm-config)
    static ref ENV_LLVM_PREFIX: String =
        format!("LLVM_SYS_{}_PREFIX", env!("CARGO_PKG_VERSION_MAJOR"));

    /// If exactly "YES", ignore the version blocklist
    static ref ENV_IGNORE_BLOCKLIST: String =
        format!("LLVM_SYS_{}_IGNORE_BLOCKLIST", env!("CARGO_PKG_VERSION_MAJOR"));

    /// If set, enforce precise correspondence between crate and binary versions.
    static ref ENV_STRICT_VERSIONING: String =
        format!("LLVM_SYS_{}_STRICT_VERSIONING", env!("CARGO_PKG_VERSION_MAJOR"));

    /// If set, do not attempt to strip irrelevant options for llvm-config --cflags
    static ref ENV_NO_CLEAN_CFLAGS: String =
        format!("LLVM_SYS_{}_NO_CLEAN_CFLAGS", env!("CARGO_PKG_VERSION_MAJOR"));

    /// If set and targeting MSVC, force the debug runtime library
    static ref ENV_USE_DEBUG_MSVCRT: String =
        format!("LLVM_SYS_{}_USE_DEBUG_MSVCRT", env!("CARGO_PKG_VERSION_MAJOR"));

    /// If set, always link against libffi
    static ref ENV_FORCE_FFI: String =
        format!("LLVM_SYS_{}_FFI_WORKAROUND", env!("CARGO_PKG_VERSION_MAJOR"));
}

lazy_static! {
    /// LLVM version used by this version of the crate.
    static ref CRATE_VERSION: Version = {
        let crate_version = Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("Crate version is somehow not valid semver");
        Version {
            major: crate_version.major / 10,
            minor: crate_version.major % 10,
            .. crate_version
        }
    };

    /// Filesystem path to an llvm-config binary for the correct version.
    static ref LLVM_CONFIG_PATH: Option<PathBuf> = locate_llvm_config();
}

fn target_env_is(name: &str) -> bool {
    match env::var_os("CARGO_CFG_TARGET_ENV") {
        Some(s) => s == name,
        None => false,
    }
}

fn target_os_is(name: &str) -> bool {
    match env::var_os("CARGO_CFG_TARGET_OS") {
        Some(s) => s == name,
        None => false,
    }
}

/// Try to find a version of llvm-config that is compatible with this crate.
///
/// If $LLVM_SYS_<VERSION>_PREFIX is set, look for llvm-config ONLY in there. The assumption is
/// that the user know best, and they want to link to a specific build or fork of LLVM.
///
/// If $LLVM_SYS_<VERSION>_PREFIX is NOT set, then look for llvm-config in $PATH.
///
/// Returns None on failure.
fn locate_llvm_config() -> Option<PathBuf> {
    let prefix = env::var_os(&*ENV_LLVM_PREFIX).map(|p| PathBuf::from(p).join("bin")).unwrap_or_else(PathBuf::new);
    for binary_name in llvm_config_binary_names() {
        let binary_name = prefix.join(binary_name);
        match llvm_version(&binary_name) {
            Ok(ref version) if is_compatible_llvm(version) => {
                // Compatible version found. Nice.
                return Some(binary_name);
            }
            Ok(version) => {
                // Version mismatch. Will try further searches, but warn that
                // we're not using the system one.
                println!("Found LLVM version {} on PATH, but need {}.", version, *CRATE_VERSION);
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                // Looks like we failed to execute any llvm-config. Keep
                // searching.
            }
            // Some other error, probably a weird failure. Give up.
            Err(e) => panic!("Failed to search PATH for llvm-config: {}", e),
        }
    }

    None
}

/// Return an iterator over possible names for the llvm-config binary.
fn llvm_config_binary_names() -> std::vec::IntoIter<String> {
    let mut base_names = vec![
        "llvm-config".into(),
        format!("llvm-config-{}", CRATE_VERSION.major),
        format!("llvm{}-config", CRATE_VERSION.major),
        format!("llvm-config-{}.{}", CRATE_VERSION.major, CRATE_VERSION.minor),
        format!("llvm-config{}{}", CRATE_VERSION.major, CRATE_VERSION.minor),
    ];

    // On Windows, also search for llvm-config.exe
    if target_os_is("windows") {
        let mut exe_names = base_names.clone();
        for name in exe_names.iter_mut() {
            name.push_str(".exe");
        }
        base_names.extend(exe_names);
    }

    base_names.into_iter()
}

/// Check whether the given version of LLVM is blocklisted,
/// returning `Some(reason)` if it is.
fn is_blocklisted_llvm(llvm_version: &Version) -> Option<&'static str> {
    static BLOCKLIST: &'static [(u64, u64, u64, &'static str)] = &[];

    if let Some(x) = env::var_os(&*ENV_IGNORE_BLOCKLIST) {
        if &x == "YES" {
            println!("cargo:warning=Ignoring blocklist entry for LLVM {}", llvm_version);
            return None;
        } else {
            println!(
                "cargo:warning={} is set but not exactly \"YES\"; blocklist is still honored.",
                *ENV_IGNORE_BLOCKLIST
            );
        }
    }

    for &(major, minor, patch, reason) in BLOCKLIST.iter() {
        let bad_version = Version {
            major: major,
            minor: minor,
            patch: patch,
            pre: semver::Prerelease::EMPTY,
            build: semver::BuildMetadata::EMPTY,
        };

        if &bad_version == llvm_version {
            return Some(reason);
        }
    }
    None
}

/// Check whether the given LLVM version is compatible with this version of
/// the crate.
fn is_compatible_llvm(llvm_version: &Version) -> bool {
    if let Some(reason) = is_blocklisted_llvm(llvm_version) {
        println!("Found LLVM {}, which is blocklisted: {}", llvm_version, reason);
        return false;
    }

    let strict = env::var_os(&*ENV_STRICT_VERSIONING).is_some() || cfg!(feature = "strict-versioning");
    if strict {
        llvm_version.major == CRATE_VERSION.major && llvm_version.minor == CRATE_VERSION.minor
    } else {
        llvm_version.major >= CRATE_VERSION.major
            || (llvm_version.major == CRATE_VERSION.major && llvm_version.minor >= CRATE_VERSION.minor)
    }
}

/// Get the output from running `llvm-config` with the given argument.
///
/// Lazily searches for or compiles LLVM as configured by the environment
/// variables.
fn llvm_config(arg: &str) -> String {
    llvm_config_ex(&*LLVM_CONFIG_PATH.clone().unwrap(), arg).expect("Surprising failure from llvm-config")
}

/// Invoke the specified binary as llvm-config.
///
/// Explicit version of the `llvm_config` function that bubbles errors
/// up.
fn llvm_config_ex<S: AsRef<OsStr>>(binary: S, arg: &str) -> io::Result<String> {
    Command::new(binary)
        .arg(arg)
        .arg("--link-static") // Don't use dylib for >= 3.9
        .output()
        .and_then(|output| {
            if output.stdout.is_empty() {
                Err(io::Error::new(io::ErrorKind::NotFound, "llvm-config returned empty output"))
            } else {
                Ok(String::from_utf8(output.stdout).expect("Output from llvm-config was not valid UTF-8"))
            }
        })
}

/// Get the LLVM version using llvm-config.
fn llvm_version<S: AsRef<OsStr>>(binary: &S) -> io::Result<Version> {
    let version_str = llvm_config_ex(binary.as_ref(), "--version")?;

    // LLVM isn't really semver and uses version suffixes to build
    // version strings like '3.8.0svn', so limit what we try to parse
    // to only the numeric bits.
    let re = Regex::new(r"^(?P<major>\d+)\.(?P<minor>\d+)(?:\.(?P<patch>\d+))??").unwrap();
    let c = match re.captures(&version_str) {
        Some(c) => c,
        None => {
            panic!("Could not determine LLVM version from llvm-config. Version string: {}", version_str);
        }
    };

    // some systems don't have a patch number but Version wants it so we just append .0 if it isn't
    // there
    let s = match c.name("patch") {
        None => format!("{}.0", &c[0]),
        Some(_) => c[0].to_string(),
    };
    Ok(Version::parse(&s).unwrap())
}

/// Get the names of the dylibs required by LLVM, including the C++ standard
/// library.
fn get_system_libraries() -> Vec<String> {
    llvm_config("--system-libs")
        .split(&[' ', '\n'] as &[char])
        .filter(|s| !s.is_empty())
        .map(|flag| {
            if target_env_is("msvc") {
                // Same as --libnames, foo.lib
                assert!(flag.ends_with(".lib"), "system library {:?} does not appear to be a MSVC library file", flag);
                &flag[..flag.len() - 4]
            } else {
                if flag.starts_with("-l") {
                    // Linker flags style, -lfoo
                    if target_os_is("macos") && flag.starts_with("-llib") && flag.ends_with(".tbd") {
                        // .tdb libraries are "text-based stub" files that provide lists of symbols,
                        // which refer to libraries shipped with a given system and aren't shipped
                        // as part of the corresponding SDK. They're named like the underlying
                        // library object, including the 'lib' prefix that we need to strip.
                        return flag[5..flag.len() - 4].to_owned();
                    }
                    return flag[2..].to_owned();
                }

                let maybe_lib = Path::new(&flag);
                if maybe_lib.is_file() {
                    // Library on disk, likely an absolute path to a .so. We'll add its location to
                    // the library search path and specify the file as a link target.
                    println!("cargo:rustc-link-search={}", maybe_lib.parent().unwrap().display());

                    // Expect a file named something like libfoo.so, or with a version libfoo.so.1.
                    // Trim everything after and including the last .so and remove the leading 'lib'
                    let soname =
                        maybe_lib.file_name().unwrap().to_str().expect("Shared library path must be a valid string");
                    let stem =
                        soname.rsplit_once(target_dylib_extension()).expect("Shared library should be a .so file").0;

                    stem.trim_start_matches("lib")
                } else {
                    panic!("Unable to parse result of llvm-config --system-libs: was {:?}", flag)
                }
            }
            .to_owned()
        })
        .chain(get_system_libcpp().map(str::to_owned))
        .collect::<Vec<String>>()
}

fn target_dylib_extension() -> &'static str {
    if target_os_is("macos") {
        ".dylib"
    } else {
        ".so"
    }
}

/// Get the library that must be linked for C++, if any.
fn get_system_libcpp() -> Option<&'static str> {
    if target_env_is("msvc") {
        // MSVC doesn't need an explicit one.
        None
    } else if target_os_is("macos") {
        // On OS X 10.9 and later, LLVM's libc++ is the default. On earlier
        // releases GCC's libstdc++ is default. Unfortunately we can't
        // reasonably detect which one we need (on older ones libc++ is
        // available and can be selected with -stdlib=lib++), so assume the
        // latest, at the cost of breaking the build on older OS releases
        // when LLVM was built against libstdc++.
        Some("c++")
    } else if target_os_is("freebsd") {
        Some("c++")
    } else if target_env_is("musl") {
        // The one built with musl.
        Some("c++")
    } else {
        // Otherwise assume GCC's libstdc++.
        // This assumption is probably wrong on some platforms, but would need
        // testing on them.
        Some("stdc++")
    }
}

/// Get the names of libraries to link against.
fn get_link_libraries() -> Vec<String> {
    // Using --libnames in conjunction with --libdir is particularly important
    // for MSVC when LLVM is in a path with spaces, but it is generally less of
    // a hack than parsing linker flags output from --libs and --ldflags.
    llvm_config("--libnames")
        .split(&[' ', '\n'] as &[char])
        .filter(|s| !s.is_empty())
        .map(|name| {
            // --libnames gives library filenames. Extract only the name that
            // we need to pass to the linker.
            if target_env_is("msvc") {
                // LLVMfoo.lib
                assert!(name.ends_with(".lib"), "library name {:?} does not appear to be a MSVC library file", name);
                &name[..name.len() - 4]
            } else {
                // libLLVMfoo.a
                assert!(
                    name.starts_with("lib") && name.ends_with(".a"),
                    "library name {:?} does not appear to be a static library",
                    name
                );
                &name[3..name.len() - 2]
            }
        })
        .map(str::to_owned)
        .collect::<Vec<String>>()
}

fn get_llvm_cflags() -> String {
    let output = llvm_config("--cflags");

    // llvm-config includes cflags from its own compilation with --cflags that
    // may not be relevant to us. In particularly annoying cases, these might
    // include flags that aren't understood by the default compiler we're
    // using. Unless requested otherwise, clean CFLAGS of options that are
    // known to be possibly-harmful.
    let no_clean = env::var_os(&*ENV_NO_CLEAN_CFLAGS).is_some();
    if no_clean || target_env_is("msvc") {
        // MSVC doesn't accept -W... options, so don't try to strip them and
        // possibly strip something that should be retained. Also do nothing if
        // the user requests it.
        return output;
    }

    llvm_config("--cflags").split(&[' ', '\n'][..]).filter(|word| !word.starts_with("-W")).collect::<Vec<_>>().join(" ")
}

fn is_llvm_debug() -> bool {
    // Has to be either Debug or Release
    llvm_config("--build-mode").contains("Debug")
}

fn main() {
    println!("cargo:rerun-if-changed=wrappers/target.h");
    println!("cargo:rerun-if-changed=wrappers/target.cpp");

    // Behavior can be significantly affected by these vars.
    println!("cargo:rerun-if-env-changed={}", &*ENV_LLVM_PREFIX);
    if let Ok(path) = env::var(&*ENV_LLVM_PREFIX) {
        println!("cargo:rerun-if-changed={}", path);
    }

    println!("cargo:rerun-if-env-changed={}", &*ENV_IGNORE_BLOCKLIST);
    println!("cargo:rerun-if-env-changed={}", &*ENV_STRICT_VERSIONING);
    println!("cargo:rerun-if-env-changed={}", &*ENV_NO_CLEAN_CFLAGS);
    println!("cargo:rerun-if-env-changed={}", &*ENV_USE_DEBUG_MSVCRT);
    println!("cargo:rerun-if-env-changed={}", &*ENV_FORCE_FFI);

    if cfg!(feature = "no-llvm-linking") && cfg!(feature = "disable-alltargets-init") {
        // exit early as we don't need to do anything and llvm-config isn't needed at all
        return;
    }

    if LLVM_CONFIG_PATH.is_none() {
        println!("cargo:rustc-cfg=LLVM_SYS_NOT_FOUND");
        return;
    }

    // Build the extra wrapper functions.
    if !cfg!(feature = "disable-alltargets-init") {
        std::env::set_var("CXXFLAGS", get_llvm_cflags());
        cc::Build::new().cpp(true).file("wrappers/target.cpp").flag("-std=c++14").compile("targetwrappers");
    }

    if cfg!(feature = "no-llvm-linking") {
        return;
    }

    let libdir = llvm_config("--libdir");

    // Export information to other crates
    println!("cargo:config_path={}", LLVM_CONFIG_PATH.clone().unwrap().display()); // will be DEP_LLVM_CONFIG_PATH
    println!("cargo:libdir={}", libdir); // DEP_LLVM_LIBDIR

    // Link LLVM libraries
    println!("cargo:rustc-link-search=native={}", libdir);
    for name in get_link_libraries() {
        println!("cargo:rustc-link-lib=static={}", name);
    }

    // link LLD libraries
    println!("cargo:rustc-link-lib=static=lldCommon");
    println!("cargo:rustc-link-lib=static=lldELF");
    println!("cargo:rustc-link-lib=static=lldMachO");
    println!("cargo:rustc-link-lib=static=lldCOFF");
    println!("cargo:rustc-link-lib=static=lldWasm");
    println!("cargo:rustc-link-lib=static=lldMinGW");

    // Link system libraries
    for name in get_system_libraries() {
        let link_type = if target_env_is("musl") {
            "static"
        } else {
            "dylib"
        };
        println!("cargo:rustc-link-lib={}={}", link_type, name);
    }

    let use_debug_msvcrt = env::var_os(&*ENV_USE_DEBUG_MSVCRT).is_some();
    if target_env_is("msvc") && (use_debug_msvcrt || is_llvm_debug()) {
        println!("cargo:rustc-link-lib={}", "msvcrtd");
    }

    // Link libffi if the user requested this workaround.
    // See https://bitbucket.org/tari/llvm-sys.rs/issues/12/
    let force_ffi = env::var_os(&*ENV_FORCE_FFI).is_some();
    if force_ffi {
        println!("cargo:rustc-link-lib=dylib={}", "ffi");
    }
}
