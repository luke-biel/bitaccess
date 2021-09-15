use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::parse_quote::parse;
use syn::punctuated::Punctuated;
use syn::{parenthesized, Lit, Token, Type, Visibility};
use syn::{Ident, ItemEnum, Variant};

mod kw {
    syn::custom_keyword!(base_type);
    syn::custom_keyword!(offset);
    syn::custom_keyword!(size);
}

pub struct BitAccess {
    top_level_arguments: TopLevelMacroArguments,
    struct_identifier: Ident,
    struct_visibility: Visibility,
    fields: Vec<BitField>,
}

struct TopLevelMacroArguments {
    base_type: Type,
}

struct BitField {
    field_level_arguments: FieldLevelMacroArguments,
    ident: Ident,
}

struct FieldLevelMacroArguments {
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
    pub fn new(args: TokenStream2, item_struct: ItemEnum) -> syn::Result<Self> {
        Ok(Self {
            top_level_arguments: parse::<TopLevelMacroArguments>(args),
            struct_identifier: item_struct.ident,
            struct_visibility: item_struct.vis,
            fields: BitField::many(item_struct.variants)?,
        })
    }

    pub fn into_token_stream(self) -> TokenStream2 {
        let Self {
            top_level_arguments: TopLevelMacroArguments { base_type },
            struct_identifier: ident,
            struct_visibility: vis,
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

        // TODO: Consider adding some random string to this module name, so that users can't manually edit it
        let mod_ident = Ident::new(&ident.to_string().to_case(Case::Snake), ident.span());
        let private_ident = Ident::new(&format!("__private_{}", &ident), ident.span());

        let readers: Vec<_> = fields.iter().map(|item| item.reader()).collect();
        let writers: Vec<_> = fields.iter().map(|item| item.writer()).collect();

        let const_enums: Vec<_> = fields
            .iter()
            .map(|field| field.const_enum(&base_type))
            .collect();

        quote! {
            #vis struct #ident {
                inner: #mod_ident::#private_ident,
            }

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

                    #vis fn read(&self, bits: #base_type) -> #base_type {
                        match bits {
                            #(Self::#enum_field_names => #readers,)*
                            _ => panic!("Use provided consts to access register"),
                        }
                    }

                    #vis fn write(&mut self, bits: #base_type, new_value: #base_type) {
                        match bits {
                            #(Self::#enum_field_names => #writers,)*
                            _ => panic!("Use provided consts to access register"),
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

impl Parse for TopLevelMacroArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _: kw::base_type = input.parse()?;
        let _: Token![=] = input.parse()?;
        Ok(Self {
            base_type: input.parse()?,
        })
    }
}

impl BitField {
    fn many(fields: Punctuated<Variant, Token![,]>) -> syn::Result<Vec<Self>> {
        fields
            .into_iter()
            .map(|variant: Variant| BitField::single(variant))
            .collect()
    }

    fn single(variant: Variant) -> syn::Result<Self> {
        let mods = match variant
            .attrs
            .into_iter()
            .find(|attr| attr.path.is_ident("bitaccess"))
        {
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

    fn reader(&self) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, .. },
            ..
        } = self;
        quote! {
            (self.inner.value & bits) >> #offset
        }
    }

    fn writer(&self) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, .. },
            ..
        } = self;
        quote! {
            self.inner.value |= (new_value & (bits >> #offset)) << #offset
        }
    }

    fn const_enum(&self, base_type: &Type) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, size },
            ident,
        } = self;

        let name = Ident::new(&ident.to_string().to_case(Case::Pascal), ident.span());

        quote! {
            const #name: #base_type = ((1 << #size) - 1) << #offset;
        }
    }
}

impl Parse for FieldLevelMacroArguments {
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
    fn build(self) -> FieldLevelMacroArguments {
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
        FieldLevelMacroArguments { offset, size }
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
