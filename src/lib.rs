extern crate proc_macro;
use proc_macro::TokenStream;

use crate::bitaccess::BitAccess;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, ItemStruct};

mod bitaccess;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn bitaccess(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    let tokens = BitAccess::new(args.into(), input)
        .unwrap()
        .into_token_stream();

    tokens.into()
}
