use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_quote, parse_quote::parse, punctuated::Punctuated, Token, Type, Variant};

use crate::{
    extra_enum_access::{ExtraEnumAccess, InlineEnumAccess, InlineEnumEntry},
    field_level_macro_arguments::FieldLevelMacroArguments,
};

pub struct BitField {
    field_level_arguments: FieldLevelMacroArguments,
    extra_enum_access: Option<ExtraEnumAccess>,
    pub ident: Ident,
}

impl BitField {
    pub fn many(fields: Punctuated<Variant, Token![,]>) -> syn::Result<Vec<Self>> {
        fields.into_iter().map(BitField::single).collect()
    }

    fn single(variant: Variant) -> syn::Result<Self> {
        let mut bit_attribute = None;
        let mut variant_attribute = None;

        for attr in variant.attrs {
            if attr.path.is_ident("bitaccess")
                || attr.path.is_ident("bits")
                || attr.path.is_ident("bit")
            {
                if bit_attribute.is_some() {
                    proc_macro_error::abort_call_site!(
                        "duplicate bit declaration attribute on a field"
                    );
                } else {
                    bit_attribute = Some(attr);
                }
            } else if attr.path.is_ident("variants") {
                if variant_attribute.is_some() {
                    proc_macro_error::abort_call_site!(
                        "duplicate variants declaration attribute on a field"
                    );
                } else {
                    variant_attribute = Some(attr);
                }
            }
        }

        let bit_attribute = if let Some(bit_attribute) = bit_attribute {
            bit_attribute
        } else {
            proc_macro_error::abort_call_site!(
                "missing bitaccess attribute on field `{}`",
                &variant.ident
            )
        };

        Ok(Self {
            field_level_arguments: parse::<FieldLevelMacroArguments>(bit_attribute.tokens),
            extra_enum_access: variant_attribute.map(|i| parse::<ExtraEnumAccess>(i.tokens)),
            ident: variant.ident,
        })
    }

    pub fn reader(&self) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, .. },
            ..
        } = self;
        quote! {
            (value & bits.mask()) >> #offset
        }
    }

    pub fn writer(&self) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, .. },
            ..
        } = self;
        quote! {
            self.write_raw((new_value.value() & (bits.mask() >> #offset)) << #offset)
        }
    }

    pub fn const_enum(&self, base_type: &Type, mod_ident: &Ident) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, size },
            ident,
            extra_enum_access,
        } = self;

        let name = Ident::new(&ident.to_string(), ident.span());
        let field_type = extra_enum_access
            .as_ref()
            .map(|e| match e {
                ExtraEnumAccess::ExternalEnum(typ) => typ.clone(),
                ExtraEnumAccess::InlineEnum(_) => {
                    let ident = self.inline_enum_ident();
                    parse_quote! { #mod_ident::#ident }
                }
            })
            .unwrap_or(base_type.clone());

        quote! {
            const #name: bitaccess::FieldDefinition<#base_type, #field_type> = bitaccess::FieldDefinition::new(((1 << #size) - 1) << #offset);
        }
    }

    pub fn extra_enum_access(&self, base_type: &Type) -> TokenStream2 {
        match &self.extra_enum_access {
            Some(ExtraEnumAccess::InlineEnum(InlineEnumAccess { items })) => {
                let entries: Vec<TokenStream2> = items
                    .iter()
                    .map(|InlineEnumEntry { ident, value, .. }| {
                        quote! {
                            #ident = #value
                        }
                    })
                    .collect();

                let enum_ident = self.inline_enum_ident();

                let matchers: Vec<TokenStream2> = items
                    .iter()
                    .map(|InlineEnumEntry { ident, value, .. }| {
                        quote! {
                            #enum_ident::#ident => #value
                        }
                    })
                    .collect();

                let val_matchers: Vec<TokenStream2> = items
                    .iter()
                    .map(|InlineEnumEntry { ident, value, .. }| {
                        quote! {
                            #value => #enum_ident::#ident,
                        }
                    })
                    .collect();

                quote! {
                    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
                    pub enum #enum_ident {
                        #(#entries),*
                    }

                    impl bitaccess::FieldAccess<#base_type> for #enum_ident {
                        fn to_raw(&self) -> #base_type {
                            match self {
                                #(#matchers),*
                            }
                        }
                    }

                    impl From<#enum_ident> for bitaccess::Field<#base_type, #enum_ident> {
                        fn from(e: #enum_ident) -> Self {
                            use bitaccess::FieldAccess as _;
                            bitaccess::Field::new(e.to_raw())
                        }
                    }

                    impl From<#base_type> for #enum_ident {
                        fn from(val: #base_type) -> #enum_ident {
                            match val {
                                #(#val_matchers)*
                                _ => panic!("value out of range of enum {}", stringify!(#enum_ident)),
                            }
                        }
                    }
                }
            }
            _ => TokenStream2::new(),
        }
    }

    fn inline_enum_ident(&self) -> Ident {
        Ident::new(
            &self.ident.to_string().to_case(Case::UpperCamel),
            self.ident.span(),
        )
    }
}
