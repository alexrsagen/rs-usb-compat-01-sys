extern crate cc;
extern crate bindgen;
extern crate pkg_config;

use bindgen::Builder;
use cc::Build;
use std::{env, fs, io};
use std::path::{Path, PathBuf};

static VERSION: &'static str = "0.1.7";

fn link_objects_recursively<P: AsRef<Path>>(build: &mut Build, path: P) -> Result<(), io::Error> {
	for entry in fs::read_dir(path)? {
		let entry = entry?;
		let file_type = entry.file_type()?;
		let entry_path = entry.path();
		if file_type.is_dir() {
			link_objects_recursively(build, entry_path)?;
		} else if file_type.is_file() && entry_path.extension().map(|ext| ext == "o").unwrap_or(false) {
			build.object(entry_path);
		}
	}
	Ok(())
}

fn main() {
	let usb1_include_dir = PathBuf::from(env::var("DEP_USB_1.0_INCLUDE").expect("libusb1-sys did not export DEP_USB_1.0_INCLUDE"));
	let vendor_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR var not set")).join("vendor");
	let usb01_dir = vendor_dir.join("usb-compat-0.1").join("libusb");
	let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR var not set"));
	let include_dir = &out_dir.join("include");

	// Build libusb-compat-0.1
	fs::create_dir_all(&include_dir).unwrap();
	fs::copy(
		usb01_dir.join("usb.h"),
		include_dir.join("usb.h"),
	).unwrap();
	fs::copy(
		usb01_dir.join("usb.h"),
		include_dir.join("lusb0_usb.h"),
	).unwrap();
	println!("cargo:include={}", include_dir.to_str().unwrap());
	fs::File::create(&include_dir.join("config.h")).unwrap();
	let mut base_config = cc::Build::new();
	base_config.include(&usb1_include_dir);
	link_objects_recursively(&mut base_config, &usb1_include_dir.parent().unwrap().join("libusb").join("libusb")).unwrap();
	base_config.include(&include_dir);
	base_config.include(&usb01_dir);

	base_config.define("PRINTF_FORMAT(a, b)", Some(""));
	base_config.define("ENABLE_LOGGING", Some("1"));
	if cfg!(feature = "logging") {
		base_config.define("ENABLE_DEBUG_LOGGING", Some("1"));
	}

	if cfg!(target_os = "macos") {
		base_config.define("OS_DARWIN", Some("1"));
	}

	if cfg!(target_os = "linux") || cfg!(target_os = "android") {
		base_config.define("OS_LINUX", Some("1"));
		base_config.define("HAVE_ASM_TYPES_H", Some("1"));
		base_config.define("_GNU_SOURCE", Some("1"));
		base_config.define("HAVE_TIMERFD", Some("1"));
		base_config.define("HAVE_EVENTFD", Some("1"));
	}

	if cfg!(target_family = "unix") {
		base_config.define("HAVE_SYS_TIME_H", Some("1"));
		base_config.define("HAVE_NFDS_T", Some("1"));
		base_config.define("PLATFORM_POSIX", Some("1"));
		base_config.define("HAVE_CLOCK_GETTIME", Some("1"));
		base_config.define(
			"DEFAULT_VISIBILITY",
			Some("__attribute__((visibility(\"default\")))"),
		);

		match pkg_config::probe_library("libudev") {
			Ok(_lib) => {
				base_config.define("USE_UDEV", Some("1"));
				base_config.define("HAVE_LIBUDEV", Some("1"));
			}
			_ => {}
		};
	}

	if cfg!(target_os = "windows") {
		#[cfg(target_env = "msvc")]
		base_config.flag("/source-charset:utf-8");

		base_config.warnings(false);
		base_config.define("OS_WINDOWS", Some("1"));

		base_config.define("DEFAULT_VISIBILITY", Some(""));
		base_config.define("PLATFORM_WINDOWS", Some("1"));
	}

	base_config.file(usb01_dir.join("core.c"));

	base_config.out_dir(&out_dir);
	base_config.compile("usb");

	// Output metainfo
	println!("cargo:vendored=1");
    println!("cargo:static=1");
	println!("cargo:include={}", include_dir.display());
	println!("cargo:version_number={}", VERSION);
	if cfg!(target_os = "macos") {
		println!("cargo:rustc-link-lib=framework=CoreFoundation");
		println!("cargo:rustc-link-lib=framework=IOKit");
		println!("cargo:rustc-link-lib=objc");
	}
	if cfg!(target_family = "unix") && pkg_config::probe_library("libudev").is_ok() {
		println!("cargo:rustc-link-lib=udev");
	}

	// Generate libusb-compat-0.1 bindings
	Builder::default()
		.header(include_dir.join("usb.h").to_str().unwrap())
		.clang_arg(format!("-I{}", include_dir.display()))
		.allowlist_function("usb_.*")
		.allowlist_type("usb_.*")
		.allowlist_var("USB_.*")
		.generate()
		.expect("Unable to generate usb bindings")
		.write_to_file(out_dir.join("bindings.rs"))
		.expect("Unable to write usb bindings");
}