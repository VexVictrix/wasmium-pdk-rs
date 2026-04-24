pub use wasmium_macro::wasmium_fn;
pub use wasmium_macro::import_module;

#[unsafe(no_mangle)]
pub extern "C" fn wasmium_alloc(size: u64) -> u64 {
	let mut buf = Vec::with_capacity(size as usize);
	let ptr: *mut u8 = buf.as_mut_ptr();
	std::mem::forget(buf);
	ptr as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmium_free(ptr: u64, size: u64) {
	unsafe { let _ = Vec::from_raw_parts(ptr as *mut u8, 0, size as usize); }
}

/// Copies bytes from guest memory into an owned buffer.
pub fn read_bytes(ptr: u64) -> Vec<u8> {
	let length_bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, 8) };
	let len = u64::from_le_bytes(length_bytes.try_into().expect("Failed to convert length to u64"));
	unsafe { std::slice::from_raw_parts((ptr  + 8) as *const u8, len as usize).to_vec() }
} // end fn read_bytes

pub fn write_bytes(ptr: u64, data: &[u8]) {
	let length_bytes = (data.len() as u64).to_le_bytes();
	unsafe { std::ptr::copy(length_bytes.as_ptr(), ptr as *mut u8, 8); }
	unsafe { std::ptr::copy(data.as_ptr(), (ptr + 8) as *mut u8, data.len()); }
} // end fn write_bytes

#[unsafe(no_mangle)]
pub extern "C" fn example_function(input_ptr: u64) -> u64 {
	log("Hello from WASM! Received input pointer");
	let input = read_bytes(input_ptr);
	// Process the input bytes and produce output bytes
	let output = input; // For demonstration, just echo the input back as output
	let output_ptr = wasmium_alloc(output.len() as u64 + 8);
	write_bytes(output_ptr, &output);
	output_ptr
}

use std::sync::Once;
// use wasmium_macro::wasmium_fn;

// #[cfg(not(all(target_arch = "wasm32", test)))]
// use wasmium_macro::import_module;

/// Runtime one-time initialization entrypoint invoked by the host.
#[wasmium_fn]
fn __sys_init() {
	install_panic_hook();
}

static PANIC_HOOK_ONCE: Once = Once::new();

/// Installs a panic hook that forwards panic messages to the host logger.
fn install_panic_hook() {
	PANIC_HOOK_ONCE.call_once(|| {
		std::panic::set_hook(Box::new(|panic_info| {
			let msg = format!(
				"panic: {}{}",
				panic_info.payload()
					.downcast_ref::<&str>()
					.copied()
					.or_else(|| panic_info.payload().downcast_ref::<String>().map(String::as_str))
					.unwrap_or("<non-string panic payload>"),
				panic_info.location()
					.map(|l| format!(" at {}:{}:{}", l.file(), l.line(), l.column()))
					.unwrap_or_default()
			); log(msg.as_str());
		}));
	});
}

#[cfg(not(all(target_arch = "wasm32", test)))]
import_module!("wasmium_sys", (
	log(&str),
));

#[cfg(all(target_arch = "wasm32", test))]
fn log(_: &str) {}

