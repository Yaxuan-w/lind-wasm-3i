extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Meta, MetaNameValue, Lit, Expr};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::parse::Parser;

#[proc_macro_attribute]
pub fn syscall_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;
    let block = &input.block;

    let attr_args = Punctuated::<Meta, Comma>::parse_terminated
        .parse(attr.into())
        .unwrap();

    let mut conversions = Vec::new();

    for arg in attr_args.iter() {
        if let Meta::NameValue(nv) = arg {
            if let Some(ident) = nv.path.get_ident() {
                if let Expr::Lit(syn::ExprLit { lit: Lit::Str(ref lit_str), .. }) = &nv.value {
                    let param_name = ident.to_string();
                    let conversion_fn = lit_str.value();
                    conversions.push((param_name, conversion_fn));
                }
            }
        }
    }

    let params = input.sig.inputs.iter().map(|arg| {
        if let syn::FnArg::Typed(pat) = arg {
            let param_name = if let syn::Pat::Ident(pat_ident) = &*pat.pat {
                pat_ident.ident.to_string()
            } else {
                return quote! { #arg };
            };

            if let Some((_, conv_fn)) = conversions.iter().find(|(name, _)| name == &param_name) {
                let param_ident = &pat.pat;
                return quote! {
                    let #param_ident = #conv_fn(#param_ident);
                };
            }
        }
        quote! {}
    });

    let expanded = quote! {
        pub fn #name(cageid: u32, path: u64, mode_arg: u32) -> i32 {
            #(#params)*
            #block
        }
    };

    TokenStream::from(expanded)
}
