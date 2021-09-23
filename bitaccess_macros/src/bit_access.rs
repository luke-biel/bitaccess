use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse2, parse_quote, Attribute, Ident, ItemEnum, Visibility};

use crate::{bit_field::BitField, top_level_macro_arguments::TopLevelMacroArguments};

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
            .map(|field| field.const_enum(&base_type, &mod_ident))
            .collect();

        let enums: Vec<TokenStream2> = fields
            .iter()
            .map(|field| field.extra_enum_access(&base_type))
            .collect();

        let read_impl = if read {
            let readers: Vec<_> = fields.iter().map(|item| item.reader()).collect();
            quote! {
                #vis fn read<F: bitaccess::FieldAccess<#base_type>>(
                    &self,
                    bits: bitaccess::FieldDefinition<#base_type, F>
                ) -> bitaccess::Field<#base_type, F> {
                    let value = self.read_raw();
                    bitaccess::Field::new(match bits.mask() {
                        #(_ if Self::#enum_field_names.mask() == bits.mask() => #readers,)*
                        _ => panic!("use provided consts to read from register"),
                    })
                }
            }
        } else {
            TokenStream2::new()
        };

        let write_impl = if write {
            let writers: Vec<_> = fields.iter().map(|item| item.writer()).collect();
            quote! {
                #vis fn write<F: bitaccess::FieldAccess<#base_type>>(
                    &mut self,
                    bits: bitaccess::FieldDefinition<#base_type, F>,
                    new_value: impl Into<bitaccess::Field<#base_type, F>>
                ) {
                    let new_value: bitaccess::Field<#base_type, F> = new_value.into();
                    match bits.mask() {
                        #(_ if Self::#enum_field_names.mask() == bits.mask() => #writers,)*
                        _ => panic!("use provided consts to write to register"),
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

        let representation_ident = Ident::new(
            &format!("{}Representation", ident.to_string().to_case(Case::Pascal)),
            ident.span(),
        );

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

                #vis fn fetch() -> super::#representation_ident {
                    let me = Self::new_global();
                    super::#representation_ident::new(me.read_raw())
                }
            }
        };

        let fetch = if read_via.is_some() || write_via.is_some() {
            quote! {
                #vis struct #representation_ident {
                    value: #base_type,
                }


                #[allow(non_upper_case_globals)]
                impl #representation_ident {
                    #(#const_enums)*
                }
            }
        } else {
            TokenStream2::new()
        };

        let fetch_mod = if read_via.is_some() || write_via.is_some() {
            quote! {
                impl super::#representation_ident {
                    pub fn new(value: #base_type) -> Self {
                        Self {
                            value,
                        }
                    }

                    fn read_raw(&self) -> #base_type {
                        self.value
                    }

                    #read_impl
                }
            }
        } else {
            TokenStream2::new()
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
                #(#enums)*

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

                    #vis fn get(&self) -> #base_type {
                        self.read_raw()
                    }
                }

                #fetch_mod
            }

            #fetch
        }
    }
}
