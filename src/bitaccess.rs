use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::parse_quote::parse;
use syn::punctuated::Punctuated;
use syn::Ident;
use syn::{parenthesized, Field, Fields, ItemStruct, Lit, Token, Type, Visibility};

mod kw {
    syn::custom_keyword!(base_type);
    syn::custom_keyword!(offset);
    syn::custom_keyword!(size);
}

pub struct BitAccess {
    base: Base,
    ident: Ident,
    vis: Visibility,
    fields: Vec<BitField>,
}

struct Base {
    base_type: Type,
}

struct BitField {
    mods: Modifiers,
    ident: Ident,
}

struct Modifiers {
    offset: u64,
    size: u64,
}

#[derive(Default)]
struct ModifiersBuilder {
    offset: Option<u64>,
    size: Option<u64>,
}

enum ModifierKW {
    Offset(u64),
    Size(u64),
}

impl BitAccess {
    pub fn new(args: TokenStream2, item_struct: ItemStruct) -> syn::Result<Self> {
        Ok(Self {
            base: parse::<Base>(args),
            ident: item_struct.ident,
            vis: item_struct.vis,
            fields: BitField::many(item_struct.fields)?,
        })
    }

    pub fn into_token_stream(self) -> TokenStream2 {
        let Self {
            base: Base { base_type },
            ident,
            vis,
            fields,
        } = self;

        let enum_field_names: Vec<_> = fields
            .iter()
            .map(|field| {
                Ident::new(
                    &field.ident.to_string().to_case(Case::Pascal),
                    field.ident.span(),
                )
            })
            .collect();

        let mod_ident = Ident::new(&ident.to_string().to_case(Case::Snake), ident.span());
        let private_ident = Ident::new(&format!("__private_{}", &ident), ident.span());

        let readers: Vec<_> = fields.iter().map(|item| item.reader()).collect();
        let writers: Vec<_> = fields.iter().map(|item| item.writer()).collect();

        quote! {
            #vis struct #ident {
                inner: #mod_ident::#private_ident,
            }

            #vis mod #mod_ident {
                #vis enum Fields {
                    #(#enum_field_names),*
                }

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

                    #vis fn read(&self, bits: Fields) -> #base_type {
                        match bits {
                            #(Fields::#enum_field_names => #readers),*
                        }
                    }

                    #vis fn write(&mut self, bits: Fields, new_value: #base_type) {
                        match bits {
                            #(Fields::#enum_field_names => #writers),*
                        }
                    }

                    #vis fn get_raw(&self) -> #base_type {
                        self.inner.value
                    }
                }
            }
        }
    }
}

impl Parse for Base {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _: kw::base_type = input.parse()?;
        let _: Token![=] = input.parse()?;
        Ok(Self {
            base_type: input.parse()?,
        })
    }
}

impl BitField {
    fn many(fields: Fields) -> syn::Result<Vec<Self>> {
        fields
            .into_iter()
            .map(|field: Field| BitField::single(field))
            .collect()
    }

    fn single(field: Field) -> syn::Result<Self> {
        let field_ident = match field.ident {
            Some(ident) => ident,
            None => {
                proc_macro_error::abort_call_site!("cannot implement bitaccess with unnamed fields")
            }
        };

        let mods = match field
            .attrs
            .into_iter()
            .find(|attr| attr.path.is_ident("bitaccess"))
        {
            Some(mods) => mods,
            None => proc_macro_error::abort_call_site!(
                "missing bitaccess attribute on field `{}`",
                &field_ident
            ),
        };
        Ok(Self {
            mods: parse::<Modifiers>(mods.tokens),
            ident: field_ident,
        })
    }

    fn reader(&self) -> TokenStream2 {
        let Self {
            mods: Modifiers { offset, size },
            ..
        } = self;
        quote! {
            (self.inner.value & (((1 << #size) - 1) << #offset)) >> #offset
        }
    }

    fn writer(&self) -> TokenStream2 {
        let Self {
            mods: Modifiers { offset, size },
            ..
        } = self;
        quote! {
            self.inner.value |= (new_value & (1 << #size) - 1) << #offset
        }
    }
}

impl Parse for Modifiers {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        let _ = parenthesized!(content in input);
        let punct: Punctuated<ModifierKW, Token![,]> =
            content.parse_terminated(ModifierKW::parse)?;
        let mut builder = ModifiersBuilder::default();

        for item in punct.iter() {
            match item {
                ModifierKW::Offset(offset) => {
                    let existing = builder.offset.replace(*offset);
                    if existing.is_some() {
                        proc_macro_error::abort_call_site!(
                            "multiple `offset` entries in bitaccess attribute"
                        );
                    }
                }
                ModifierKW::Size(size) => {
                    let existing = builder.size.replace(*size);
                    if existing.is_some() {
                        proc_macro_error::abort_call_site!(
                            "multiple `size` entries in bitaccess attribute"
                        );
                    }
                }
            }
        }

        Ok(builder.build())
    }
}

impl ModifiersBuilder {
    fn build(self) -> Modifiers {
        let offset = match self.offset {
            Some(offset) => offset,
            None => {
                proc_macro_error::abort_call_site!("missing `offset` entry in bitaccess attribute")
            }
        };
        let size = match self.size {
            Some(size) => size,
            None => {
                proc_macro_error::abort_call_site!("missing `size` entry in bitaccess attribute")
            }
        };
        Modifiers { offset, size }
    }
}

impl Parse for ModifierKW {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::offset) {
            let _: kw::offset = input.parse()?;
            let _: Token![=] = input.parse()?;
            let lit = match input.parse::<Lit>()? {
                Lit::Int(lit_int) => lit_int.base10_parse::<u64>()?,
                _ => proc_macro_error::abort_call_site!("invalid value for `offset`"),
            };
            Ok(Self::Offset(lit))
        } else if lookahead.peek(kw::size) {
            let _: kw::size = input.parse()?;
            let _: Token![=] = input.parse()?;
            let lit = match input.parse::<Lit>()? {
                Lit::Int(lit_int) => lit_int.base10_parse::<u64>()?,
                _ => proc_macro_error::abort_call_site!("invalid value for `size`"),
            };
            Ok(Self::Size(lit))
        } else {
            proc_macro_error::abort_call_site!("unknown ModifierKW token")
        }
    }
}
