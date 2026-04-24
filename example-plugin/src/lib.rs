use wasmium_pdk_rs::*;

#[wasmium_fn]
fn addition(x: i64, y: i64) -> i64 {
	log(&format!("Example addition function called with x={}, y={}", x, y));
	return x + y;
} // end fn addition

#[wasmium_fn]
fn concat_test(a: &str, b: &str) -> String {
	concat_example(a, b)
} // end fn concat_test

#[wasmium_fn]
fn main() -> i64 {
	return 1337;
} // end fn main

import_module!("env", (
	concat_example(&str, &str) -> String,
));

