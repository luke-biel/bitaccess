use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse2, parse_quote, Attribute, Ident, ItemEnum, Visibility};

use crate::{bit_field::BitField, top_level_macro_arguments::TopLevelMacroArguments};

pub mod kw {
    syn::custom_keyword!(base_type);
    syn::custom_keyword!(kind);
}

pub struct BitAccess {
    top_level_arguments: TopLevelMacroArguments,
    struct_identifier: Ident,
    struct_visibility: Visibility,
    fields: Vec<BitField>,
    attributes: Vec<Attribute>,
}

impl BitAccess {
    pub fn new(args: TokenStream2, item: ItemEnum) -> syn::Result<Self> {
        Ok(Self {
            top_level_arguments: parse2::<TopLevelMacroArguments>(args)?,
            struct_identifier: item.ident,
            struct_visibility: item.vis,
            fields: BitField::many(item.variants)?,
            attributes: item.attrs,
        })
    }

    pub fn into_token_stream(self) -> TokenStream2 {
        let Self {
            top_level_arguments:
                TopLevelMacroArguments {
                    base_type,
                    read,
                    write,
                    write_via,
                    read_via,
                },
            struct_identifier: ident,
            struct_visibility: vis,
            fields,
            attributes,
        } = self;

        let enum_field_names: Vec<_> = fields
            .iter()
            .map(|field| Ident::new(&field.ident.to_string(), field.ident.span()))
            .collect();

        let mod_ident = Ident::new(&ident.to_string().to_case(Case::Snake), ident.span());
        let private_ident = Ident::new(&format!("__private_{}", &ident), ident.span());

        let const_enums: Vec<_> = fields
            .iter()
            .map(|field| field.const_enum(&base_type))
            .collect();

        let read_impl = if read {
            let readers: Vec<_> = fields.iter().map(|item| item.reader()).collect();
            quote! {
                #vis fn read(&self, bits: #base_type) -> #base_type {
                    let value = self.read_raw();
                    match bits {
                        #(Self::#enum_field_names => #readers,)*
                        _ => panic!("Use provided consts to access register"),
                    }
                }
            }
        } else {
            TokenStream2::new()
        };

        let write_impl = if write {
            let writers: Vec<_> = fields.iter().map(|item| item.writer()).collect();
            quote! {
                #vis fn write(&mut self, bits: #base_type, new_value: #base_type) {
                    match bits {
                        #(Self::#enum_field_names => #writers,)*
                        _ => panic!("Use provided consts to access register"),
                    }
                }
            }
        } else {
            TokenStream2::new()
        };

        let private_value_holder = if read_via.is_none() && write_via.is_none() {
            quote! { value: #base_type }
        } else {
            TokenStream2::new()
        };

        let constructors = if read_via.is_none() && write_via.is_none() {
            quote! {
                #vis fn zero() -> Self {
                    Self { inner: #private_ident { value: 0 } }
                }

                #vis fn new(value: #base_type) -> Self {
                    Self { inner: #private_ident { value, }, }
                }
            }
        } else {
            quote! {
                #vis fn new_global() -> Self {
                    Self { inner: #private_ident {}, }
                }
            }
        };
        let read_via = read_via.unwrap_or_else(|| parse_quote! { value = self.inner.value });
        let write_via = write_via.unwrap_or_else(|| parse_quote! { self.inner.value = value });

        quote! {
            #(#attributes)*
            #vis struct #ident {
                inner: #mod_ident::#private_ident,
            }

            #[allow(non_upper_case_globals)]
            impl #ident {
                #(#const_enums)*
            }

            #vis mod #mod_ident {
                #[allow(non_camel_case_types)]
                #(#attributes)*
                pub(super) struct #private_ident {
                    #private_value_holder
                }

                impl super::#ident {
                    #constructors

                    #read_impl

                    #write_impl

                    fn read_raw(&self) -> #base_type {
                        let value: #base_type;
                        #read_via;
                        value
                    }

                    fn write_raw(&mut self, new_value: #base_type) {
                        let old_value = self.read_raw();
                        let value = old_value | new_value;
                        #write_via
                    }

                    #vis fn get_raw(&self) -> #base_type {
                        self.read_raw()
                    }
                }
            }
        }
    }
}
