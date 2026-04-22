mod wasmium;

pub use wasmium::*;
pub use wasmium_macro::*;
pub use paste;
pub use rmp_serde;
pub use serde;

#[test]
fn test() {
	
	let example_plugin_bytes = include_bytes!("../example-plugin/target/wasm32-unknown-unknown/debug/example_plugin.wasm");
	
	use wasmium_runtime::*;

	let mut example_plugin = WasmModule::new(example_plugin_bytes, vec![
		HostFunction::new("concat_example", move |input: (String, String)| {
			let (a, b) = input;
			return a + &b;
		}),
	]).expect("Failed to load example plugin module");

	let result: i64 = example_plugin.call("main", ()).expect("Failed to call main function");
	assert_eq!(result, 1337);

	let concat_result: String = example_plugin.call("concat_test", ("Hello, ", "world!")).expect("Failed to call concat_test function");
	assert_eq!(concat_result, "Hello, world!");

} // end fn test


#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod wasm_tests {
	// Note: These tests are meant to be run in a browser environment
	// using `wasm-pack test --headless --firefox`
	wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	use wasmium_runtime::*;
	use wasm_bindgen_test::*;
	#[wasm_bindgen_test]
	fn test_wasm() {

		let example_plugin_bytes = include_bytes!("../example-plugin/target/wasm32-unknown-unknown/debug/example_plugin.wasm");

		let example_plugin = WasmModule::new(example_plugin_bytes, vec![
			HostFunction::new("concat_example", move |input: (String, String)| {
				let (a, b) = input;
				return a + &b;
			}),
		]).expect("Failed to load example plugin module");

		let result: i64 = example_plugin.call("main", ()).expect("Failed to call main function");
		assert_eq!(result, 1337);

		let concat_result: String = example_plugin.call("concat_test", ("Hello, ", "world!")).expect("Failed to call concat_test function");
		assert_eq!(concat_result, "Hello, world!");

	} // end fn test_wasm
} // end mod wasm_tests