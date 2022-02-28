extern crate cc;
extern crate bindgen;
extern crate pkg_config;

use bindgen::Builder;
use std::{env, fs};
use std::path::PathBuf;

static VERSION: &'static str = "0.1.7";

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
	base_config.object(&usb1_include_dir.parent().unwrap().join("libusb.a"));
	base_config.include(&include_dir);
	base_config.include(&usb01_dir);

	base_config.define("PRINTF_FORMAT(a, b)", Some(""));
	base_config.define("ENABLE_LOGGING", Some("1"));

	if std::env::var("CARGO_CFG_TARGET_OS") == Ok("macos".into()) {
		base_config.define("OS_DARWIN", Some("1"));
	}

	if std::env::var("CARGO_CFG_TARGET_OS") == Ok("linux".into())
		|| std::env::var("CARGO_CFG_TARGET_OS") == Ok("android".into())
	{
		base_config.define("OS_LINUX", Some("1"));
		base_config.define("HAVE_ASM_TYPES_H", Some("1"));
		base_config.define("_GNU_SOURCE", Some("1"));
		base_config.define("HAVE_TIMERFD", Some("1"));
		base_config.define("HAVE_EVENTFD", Some("1"));
	}

	if std::env::var("CARGO_CFG_TARGET_FAMILY") == Ok("unix".into()) {
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

	if std::env::var("CARGO_CFG_TARGET_OS") == Ok("windows".into()) {
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
	if std::env::var("CARGO_CFG_TARGET_OS") == Ok("macos".into()) {
		println!("cargo:rustc-link-lib=framework=CoreFoundation");
		println!("cargo:rustc-link-lib=framework=IOKit");
		println!("cargo:rustc-link-lib=objc");
	}
	if std::env::var("CARGO_CFG_TARGET_FAMILY") == Ok("unix".into()) && pkg_config::probe_library("libudev").is_ok() {
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