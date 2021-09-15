use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_quote::parse, punctuated::Punctuated, Token, Type, Variant};

use crate::field_level_macro_arguments::FieldLevelMacroArguments;

pub struct BitField {
    field_level_arguments: FieldLevelMacroArguments,
    pub ident: Ident,
}

impl BitField {
    pub fn many(fields: Punctuated<Variant, Token![,]>) -> syn::Result<Vec<Self>> {
        fields.into_iter().map(BitField::single).collect()
    }

    fn single(variant: Variant) -> syn::Result<Self> {
        let mods = match variant.attrs.into_iter().find(|attr| {
            attr.path.is_ident("bitaccess")
                || attr.path.is_ident("bits")
                || attr.path.is_ident("bit")
        }) {
            Some(mods) => mods,
            None => proc_macro_error::abort_call_site!(
                "missing bitaccess attribute on field `{}`",
                &variant.ident
            ),
        };
        Ok(Self {
            field_level_arguments: parse::<FieldLevelMacroArguments>(mods.tokens),
            ident: variant.ident,
        })
    }

    pub fn reader(&self) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, .. },
            ..
        } = self;
        quote! {
            (value & bits) >> #offset
        }
    }

    pub fn writer(&self) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, .. },
            ..
        } = self;
        quote! {
            self.write_raw((new_value & (bits >> #offset)) << #offset)
        }
    }

    pub fn const_enum(&self, base_type: &Type) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, size },
            ident,
        } = self;

        let name = Ident::new(&ident.to_string(), ident.span());

        quote! {
            const #name: #base_type = ((1 << #size) - 1) << #offset;
        }
    }
}
