use proc_macro2::Ident;
use syn::{
    parenthesized,
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
        let content;

        let _ = parenthesized!(content in input);

        let fork = content.fork();
        if let Ok(typ) = fork.parse() {
            if fork.is_empty() {
                return Ok(Self::ExternalEnum(typ));
            }
        }

        Ok(Self::InlineEnum(content.parse()?))
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
