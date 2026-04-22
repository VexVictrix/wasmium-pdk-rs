use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
	println!("cargo:rerun-if-env-changed=WASMIUM_SKIP_WASM_BUILD");

	if env::var_os("WASMIUM_SKIP_WASM_BUILD").is_some() { return; }

	let manifest_dir = PathBuf::from(
		env::var("CARGO_MANIFEST_DIR")
			.expect("CARGO_MANIFEST_DIR was not set"),
	);
	
	let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

	println!("Building example plugin WASM module...");
	let mut command = Command::new("cargo");
	command
		.current_dir(&manifest_dir)
		.args([ "build", "--target", "wasm32-unknown-unknown" ])
		.env("WASMIUM_SKIP_WASM_BUILD", "1");

	if profile == "release" { command.arg("--release"); }

	let status = command
		.status()
		.expect("Failed to build WASM subproject");

	if !status.success() { panic!("WASM subproject build failed!"); }
	else { println!("WASM subproject built successfully."); }
} // end build.rs fn main