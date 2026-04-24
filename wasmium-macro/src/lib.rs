use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, Ident, ItemFn, LitStr, Pat, Token, Type, parenthesized,
    parse_macro_input, punctuated::Punctuated,
};

// --- Begin wasmium_fn macro ---

#[proc_macro_attribute]
pub fn wasmium_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {

	// Parse the function the attribute is applied to
    let input_fn = parse_macro_input!(item as ItemFn);

	// Extract function name, visibility, return type, and body
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
	let fn_output = &input_fn.sig.output;
    let fn_block = &input_fn.block;
	
	// Generate a unique wrapper module name
	let wrapper_mod = format_ident!("__{}", fn_name);

    // Collect typed arguments into a vector of pattern and type pairs
    let typed_args: Vec<_> = input_fn
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(typed_arg) => Some(typed_arg),
            FnArg::Receiver(_) => None,
        })
        .collect();

	// Generate different wrapper code based on the number of typed arguments
    let expanded = match typed_args.as_slice() {

        // No arguments: ignore ptr, call block with no input
		[] => quote! {
			mod #wrapper_mod {
				use super::*;
				#[unsafe(no_mangle)]
		        #fn_vis extern "C" fn #fn_name(_ptr: u64) -> u64 {
					let result = (move || #fn_block)();
		            let payload = rmp_serde::to_vec(&result).expect("Failed to serialize");
		            let mut out_bytes = Vec::with_capacity(8 + payload.len());
		            out_bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
                    out_bytes.extend_from_slice(&payload);

					let out_ptr = out_bytes.as_ptr() as u64;
					std::mem::forget(out_bytes);
		            out_ptr
				}
			}	#fn_vis fn #fn_name() #fn_output #fn_block
			// end unit type variant of __#fn_name module and original function
		}, // end no argument match arm

        // Single argument: deserialize directly as that type
        [pt] => {
            let arg_pat = &pt.pat;
            let arg_type = &pt.ty;
            quote! {
				mod #wrapper_mod {
					use super::*;
					#[unsafe(no_mangle)]
					#fn_vis extern "C" fn #fn_name(ptr: u64) -> u64 {
						let length_bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, 8) };
						let len = u64::from_le_bytes(length_bytes.try_into().expect("Failed to convert length to u64"));
						let bytes = unsafe { std::slice::from_raw_parts((ptr + 8) as *const u8, len as usize) };
						let decoded: #arg_type = rmp_serde::from_slice(bytes).expect("Failed to deserialize");
						let result = (move |#arg_pat: #arg_type| #fn_block)(decoded);
                        let payload = rmp_serde::to_vec(&result).expect("Failed to serialize");
                        let mut out_bytes = Vec::with_capacity(8 + payload.len());
                        out_bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
						out_bytes.extend_from_slice(&payload);
						let out_ptr = out_bytes.as_ptr() as u64;
						std::mem::forget(out_bytes);
                        out_ptr
					}
				}	#fn_vis fn #fn_name(#arg_pat: #arg_type) #fn_output #fn_block
            } // end single type variant of __#fn_name module and original function
        }, // end single argument match arm

        // Multiple arguments: deserialize as a tuple, destructure into individual bindings
        typed_params => {
            let pats: Vec<&Box<Pat>> = typed_params.iter().map(|param| &param.pat).collect();
            let types: Vec<&Box<Type>> = typed_params.iter().map(|param| &param.ty).collect();
            quote! {
				mod #wrapper_mod {
					use super::*;
					#[unsafe(no_mangle)]
					#fn_vis extern "C" fn #fn_name(ptr: u64) -> u64 {
						let length_bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, 8) };
						let len = u64::from_le_bytes(length_bytes.try_into().expect("Failed to convert length to u64"));
						let bytes = unsafe { std::slice::from_raw_parts((ptr + 8) as *const u8, len as usize) };
						let (#(#pats),*): (#(#types),*) = rmp_serde::from_slice(bytes).expect("Failed to deserialize");
						let result = (move || #fn_block)();
                        let payload = rmp_serde::to_vec(&result).expect("Failed to serialize");
                        let mut out_bytes = Vec::with_capacity(8 + payload.len());
						// Write the length of the payload as the first 8 bytes, followed by the payload itself
                        out_bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
						out_bytes.extend_from_slice(&payload);
						let out_ptr = out_bytes.as_ptr() as u64;
						std::mem::forget(out_bytes);
                        out_ptr
					}
				}	#fn_vis fn #fn_name(#(#pats: #types),*) #fn_output #fn_block
            } // end __#fn_name module and original function
        }, // end multiple argument match arm
    };	TokenStream::from(expanded)
} // end wasmium_fn macro


// --- Begin import_module macro ---

// /// Type alias for an import function signature, consisting of the function name, argument types, and optional return type.
type ImportSignature = (Ident, Vec<Type>, Option<Type>);

/// Parses an import function signature of the form `fn_name(arg1: Type1, arg2: Type2) -> RetType`.
/// Returns a tuple of the function name, argument types, and optional return type.
fn parse_import_signature(input: syn::parse::ParseStream<'_>) -> syn::Result<ImportSignature> {
    let name = input.parse()?;

    let args_content;
    parenthesized!(args_content in input);
    let args = Punctuated::<Type, Token![,]>::parse_terminated(&args_content)?
        .into_iter()
        .collect();

    let ret = if input.peek(Token![->]) {
        input.parse::<Token![->]>()?;
        Some(input.parse()?)
    } else {
        None
    };

    Ok((name, args, ret))
}

/// Parses the input for the `import_module!` macro, which consists of a module name string followed by a list of import signatures.
/// Returns a tuple of the module name and a vector of import signatures.
fn parse_import_module_input(
    input: syn::parse::ParseStream<'_>,
) -> syn::Result<(LitStr, Vec<ImportSignature>)> {
    let module_name = input.parse()?;
    input.parse::<Token![,]>()?;

    let content;
    parenthesized!(content in input);
    let imports = Punctuated::<ImportSignature, Token![,]>::parse_terminated_with(
        &content,
        parse_import_signature,
    )?
    .into_iter()
    .collect();

    Ok((module_name, imports))
}

// Sanitizes a module name string into a valid Rust identifier by replacing non-alphanumeric characters with underscores.
fn sanitize_module_ident(module_name: &str) -> String {
    module_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

/// Generates an import module with the given name and function signatures.
/// For each import, creates an unsafe extern "C" declaration with the appropriate link name, and a safe wrapper that handles serialization/deserialization of arguments and return values.

/// Copies bytes from guest memory into an owned buffer.
fn read_bytes(ptr: u64) -> Vec<u8> {
	let length_bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, 8) };
	let len = u64::from_le_bytes(length_bytes.try_into().expect("Failed to convert length to u64"));
	unsafe { std::slice::from_raw_parts((ptr  + 8) as *const u8, len as usize).to_vec() }
} // end fn read_bytes

fn write_bytes(ptr: u64, data: &[u8]) {
	let length_bytes = (data.len() as u64).to_le_bytes();
	unsafe { std::ptr::copy(length_bytes.as_ptr(), ptr as *mut u8, 8); }
	unsafe { std::ptr::copy(data.as_ptr(), (ptr + 8) as *mut u8, data.len()); }
} // end fn write_bytes

#[proc_macro]
pub fn import_module(input: TokenStream) -> TokenStream {

	// Parse the input to extract the module name and import signatures
    let (module_name, imports) = parse_macro_input!(input with parse_import_module_input);

	// Generate a unique identifier for the FFI module based on the sanitized module name
    let ffi_module_ident = format_ident!("__wasm_imports_{}", sanitize_module_ident(&module_name.value()));

	// Generate the unsafe extern "C" declarations for each import function
    let ffi_imports = imports.iter().map(|import| {
        let name = &import.0;
		let name_str = name.to_string();
        quote! {
            #[link_name = #name_str]
			pub fn #name(ptr: u64) -> u64;
        }
    });

	// Generate the safe wrapper functions for each import, handling serialization/deserialization of arguments and return values
    let wrappers = imports.iter().map(|import| {
        let name = &import.0;
        let args = &import.1;
        let arg_names: Vec<_> = args
            .iter()
            .enumerate()
            .map(|(index, _)| format_ident!("t{}", index))
            .collect();

        let input_tuple = match arg_names.as_slice() {
            [] => quote! { () },
            [arg] => quote! { #arg },
            _ => quote! { (#(#arg_names),*) },
        };

		let ret = import.2.as_ref().map_or(quote! { () }, |ret| quote! { #ret });

		quote! {
			pub fn #name(#(#arg_names: #args),*) -> #ret {
				let input = #input_tuple;
				let input: Vec<u8> = crate::rmp_serde::to_vec(&input)
					.expect("Failed to serialize input for import");
                let input_len = input.len() as u64;
                let input_ptr = crate::wasmium_alloc(input_len + 8);
                crate::write_bytes(input_ptr, &input);
				let out_ptr = unsafe { #ffi_module_ident::#name(input_ptr) };
                crate::wasmium_free(input_ptr, input_len + 8);
                let out_bytes = crate::read_bytes(out_ptr);
                let result = crate::rmp_serde::from_slice(&out_bytes)
					.expect("Failed to deserialize output from import");
                crate::wasmium_free(out_ptr, out_bytes.len() as u64 + 8);
				result
			}
		}


    }); // end wrapper generation

	// Generate the final token stream that defines the FFI module with the unsafe extern "C" declarations and the safe wrapper functions
    TokenStream::from(quote! {
        mod #ffi_module_ident {
            #[link(wasm_import_module = #module_name)]
            unsafe extern "C" {
                #(#ffi_imports)*
            }
        }	#(#wrappers)*
    })

}
