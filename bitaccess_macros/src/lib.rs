extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, ItemEnum};

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
        .unwrap()
        .into_token_stream();

    tokens.into()
}
