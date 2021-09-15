use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote::parse,
    Attribute,
    Error,
    Expr,
    Ident,
    ItemEnum,
    Token,
    Type,
    Visibility,
};

use crate::bitfield::BitField;

pub(crate) mod kw {
    syn::custom_keyword!(base_type);
    syn::custom_keyword!(offset);
    syn::custom_keyword!(size);
    syn::custom_keyword!(kind);
}

pub struct BitAccess {
    top_level_arguments: TopLevelMacroArguments,
    struct_identifier: Ident,
    struct_visibility: Visibility,
    fields: Vec<BitField>,
    attributes: Vec<Attribute>,
}

struct TopLevelMacroArguments {
    base_type: Type,
    read: bool,
    write: bool,
    write_via: Option<Expr>,
    read_via: Option<Expr>,
}

impl BitAccess {
    pub fn new(args: TokenStream2, item: ItemEnum) -> syn::Result<Self> {
        Ok(Self {
            top_level_arguments: parse::<TopLevelMacroArguments>(args),
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
                pub(super) struct #private_ident {
                    value: #base_type,
                }

                impl super::#ident {
                    #vis fn zero() -> Self {
                        Self { inner: #private_ident { value: 0 } }
                    }

                    #vis fn new(value: #base_type) -> Self {
                        Self { inner: #private_ident { value, }, }
                    }

                    #read_impl

                    #write_impl

                    #vis fn get_raw(&self) -> #base_type {
                        self.inner.value
                    }
                }
            }
        }
    }
}

impl Parse for TopLevelMacroArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _: kw::base_type = input.parse()?;
        let _: Token![=] = input.parse()?;
        let base_type = input.parse()?;
        let _: Token![,] = input.parse()?;
        let _: kw::kind = input.parse()?;
        let _: Token![=] = input.parse()?;
        let kind: Ident = input.parse()?;

        let (read, write) = match kind.to_string().as_str() {
            "read_only" => (true, false),
            "write_only" => (false, true),
            "read_write" | "default" => (true, true),
            _ => {
                return Err(Error::new(
                    kind.span(),
                    "unsupported value for bitaccess kind",
                ))
            }
        };

        Ok(Self {
            base_type,
            read,
            write,
            read_via: None,
            write_via: None,
        })
    }
}
