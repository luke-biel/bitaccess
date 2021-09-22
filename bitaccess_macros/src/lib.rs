extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemEnum, LitInt, Type};

use crate::bit_access::BitAccess;

mod bit_access;
mod bit_field;
mod common;
mod extra_enum_access;
mod field_level_macro_arguments;
mod top_level_macro_arguments;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn bitaccess(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemEnum);

    let tokens = BitAccess::new(args.into(), input)
        .expect("BitAccess::new")
        .into_token_stream();

    tokens.into()
}

#[proc_macro_derive(FieldAccess, attributes(field_access))]
#[proc_macro_error]
pub fn field_access(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);
    let name = item.ident;
    let btattr = item
        .attrs
        .iter()
        .find(|attr| attr.path.is_ident("field_access"))
        .expect("missing field_access attribute on derivec(FieldAccess)");
    let base_type: Type = btattr
        .parse_args_with(<Type as syn::parse::Parse>::parse)
        .expect("expected base_type value in field_access attribute");

    let mut matchers = Vec::new();
    let mut val_matchers = Vec::new();
    let mut acc = 0;
    for item in item.variants {
        let item_name = item.ident;
        if let Some((_, val)) = item.discriminant {
            let lit: LitInt = parse_quote! { #val };
            acc = lit.base10_parse().expect("parse literal");
        }
        let acc_lit: LitInt = LitInt::new(&acc.to_string(), Span::call_site());

        matchers.push(quote! {
            #name::#item_name => #acc_lit,
        });
        val_matchers.push(quote! {
            #acc_lit => #name::#item_name,
        });
        acc += 1;
    }

    let ts = quote! {
        impl FieldAccess<#base_type> for #name {
            fn to_raw(&self) -> #base_type {
                match self {
                    #(#matchers)*
                }
            }
        }

        impl From<#name> for bitaccess::Field<#base_type, #name> {
            fn from(e: #name) -> Self {
                bitaccess::Field::new(e.to_raw())
            }
        }

        impl From<#base_type> for #name {
            fn from(v: #base_type) -> Self {
                match v {
                    #(#val_matchers)*
                    _ => panic!("unknown value for {}", stringify!(#name)),
                }
            }
        }
    };

    ts.into()
}
