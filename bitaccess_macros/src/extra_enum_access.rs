use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    LitInt,
    Token,
    Type,
};

pub enum ExtraEnumAccess {
    ExternalEnum(Type),
    InlineEnum(InlineEnumAccess),
}

pub struct InlineEnumAccess {
    pub items: Punctuated<InlineEnumEntry, Token![,]>,
}

pub struct InlineEnumEntry {
    pub ident: Ident,
    _fish_token: Token![=>],
    pub value: LitInt,
}

impl Parse for ExtraEnumAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek2(Token![=>]) {
            Ok(Self::InlineEnum(input.parse()?))
        } else {
            let typ = input.parse()?;
            Ok(Self::ExternalEnum(typ))
        }
    }
}

impl Parse for InlineEnumAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(InlineEnumAccess {
            items: input.parse_terminated(InlineEnumEntry::parse)?,
        })
    }
}

impl Parse for InlineEnumEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            _fish_token: input.parse()?,
            value: input.parse()?,
        })
    }
}
