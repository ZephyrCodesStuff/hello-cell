use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Defines the entry point for a PlayStation 3 application.
///
/// The annotated function will be executed as the application's entry point.
/// It can return `()`, `i32`, `u32`, `!`, or `Result<(), E>` where `E: core::fmt::Debug`.
///
/// When the function returns, the process terminates cleanly via LV2 Syscall 3 (`SYS_PROCESS_EXIT`).
///
/// # Example
///
/// ```rust
/// #![no_std]
/// #![no_main]
///
/// #[ps3::main]
/// fn main() -> i32 {
///     ps3::println!("Hello PlayStation 3!");
///     0
/// }
/// ```
#[proc_macro_attribute]
pub fn main(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut f = parse_macro_input!(input as ItemFn);

    if !f.sig.inputs.is_empty() {
        return syn::Error::new_spanned(
            &f.sig.inputs,
            "PS3 `main` entry point cannot take arguments",
        )
        .to_compile_error()
        .into();
    }

    let ident = f.sig.ident;
    let user_main_ident = syn::Ident::new("__ps3_user_main", ident.span());
    f.sig.ident = user_main_ident.clone();

    let expanded = quote! {
        #f

        #[no_mangle]
        pub unsafe extern "C" fn _start() -> ! {
            // Execute user main and convert termination status
            let code = ::ps3::sys::entry::Termination::report(#user_main_ident());

            // Exit process cleanly via LV2 Syscall 3 (SYS_PROCESS_EXIT)
            ::ps3::sys::syscalls::sys_process_exit(code);

            #[allow(unreachable_code)]
            loop {}
        }
    };

    TokenStream::from(expanded)
}
