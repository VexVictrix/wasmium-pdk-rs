/// Allocates guest memory and returns its pointer as `u32`.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn alloc(size: u32) -> u32 {
	let mut buf = Vec::with_capacity(size as usize);
	let ptr: *mut u8 = buf.as_mut_ptr();
	std::mem::forget(buf);
	ptr as u32
}

/// Frees memory previously allocated by [`alloc`].
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn free(ptr: u32, size: u32) {
	unsafe { let _ = Vec::from_raw_parts(ptr as *mut u8, 0, size as usize); }
}

/// Copies bytes from guest memory into an owned buffer.
pub fn bytes(ptr: u32, len: u32) -> Vec<u8> {
	unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec() }
}

/// Packs pointer and length into a single `u64` as `(ptr << 32) | len`.
pub fn get_ptr_and_len<T>(bytes: &T) -> u64 where T: AsRef<[u8]> {
	let slice = bytes.as_ref();
	let ptr = slice.as_ptr() as u32;
	let len = slice.len() as u32;
	((ptr as u64) << 32) | (len as u64)
}

use std::sync::Once;
use wasmium_macro::wasmium_fn;

#[cfg(not(all(target_arch = "wasm32", test)))]
use wasmium_macro::import_module;

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
import_module!("sys", (
	log(&str),
));

#[cfg(all(target_arch = "wasm32", test))]
fn log(_msg: &str) {}

