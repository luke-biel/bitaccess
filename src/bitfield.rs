use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote,
    parse_quote::parse,
    punctuated::Punctuated,
    Error,
    Lit,
    Pat,
    PatRange,
    RangeLimits,
    Token,
    Type,
    Variant,
};

use crate::bitaccess::kw;

pub struct BitField {
    field_level_arguments: FieldLevelMacroArguments,
    pub(crate) ident: Ident,
}

#[derive(Default)]
pub struct ModifiersBuilder {
    pub(crate) offset: Option<u64>,
    size: Option<u64>,
}

pub struct ModifierRange {
    pub lo: u64,
    pub len: u64,
}

pub enum ModifierKW {
    Offset(u64),
    Size(u64),
    Range(ModifierRange),
    Single(u64),
}

impl BitField {
    pub(crate) fn many(fields: Punctuated<Variant, Token![,]>) -> syn::Result<Vec<Self>> {
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

    pub(crate) fn reader(&self) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, .. },
            ..
        } = self;
        quote! {
            (self.inner.value & bits) >> #offset
        }
    }

    pub(crate) fn writer(&self) -> TokenStream2 {
        let Self {
            field_level_arguments: FieldLevelMacroArguments { offset, .. },
            ..
        } = self;
        quote! {
            self.inner.value |= (new_value & (bits >> #offset)) << #offset
        }
    }

    pub(crate) fn const_enum(&self, base_type: &Type) -> TokenStream2 {
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
            let lit = int_from_lit(input.parse::<Lit>()?)?;
            Ok(Self::Offset(lit))
        } else if lookahead.peek(kw::size) {
            let _: kw::size = input.parse()?;
            let _: Token![=] = input.parse()?;
            let lit = int_from_lit(input.parse::<Lit>()?)?;
            Ok(Self::Size(lit))
        } else {
            if let Ok(pat) = input.parse::<Pat>() {
                match pat {
                    Pat::Range(range) => return Ok(Self::Range(range_from_pat(&range)?)),
                    Pat::Lit(lit) => {
                        let lit: Lit = parse_quote! { #lit };
                        let single = int_from_lit(lit)?;
                        return Ok(Self::Single(single));
                    }
                    _ => {}
                }
            }

            Err(Error::new(input.span(), "unknown ModifierKW token"))
        }
    }
}

fn range_from_pat(input: &PatRange) -> syn::Result<ModifierRange> {
    let PatRange { lo, limits, hi, .. } = input;

    let lo: Lit = parse_quote! { #lo };
    let hi: Lit = parse_quote! { #hi };

    let lo = int_from_lit(lo)?;
    let hi = int_from_lit(hi)?;

    match limits {
        RangeLimits::HalfOpen(_) => Ok(ModifierRange { lo, len: hi - lo }), // 0 sized bitfields aren't supported anyway
        RangeLimits::Closed(_) => Ok(ModifierRange {
            lo,
            len: hi - lo + 1,
        }),
    }
}

fn int_from_lit(lit: Lit) -> syn::Result<u64> {
    match lit {
        Lit::Int(lit_int) => lit_int.base10_parse::<u64>(),
        _ => Err(Error::new(lit.span(), "invalid value for parameter")),
    }
}

pub(crate) struct FieldLevelMacroArguments {
    offset: u64,
    size: u64,
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
                ModifierKW::Range(range) => {
                    let ex1 = builder.size.replace(range.len);
                    let ex2 = builder.offset.replace(range.lo);
                    if ex1.is_some() || ex2.is_some() {
                        proc_macro_error::abort_call_site!(
                            "cannot use other bit specifiers with `range` bitaccess definition"
                        )
                    }
                }
                ModifierKW::Single(single) => {
                    let ex1 = builder.size.replace(1);
                    let ex2 = builder.offset.replace(*single);
                    if ex1.is_some() || ex2.is_some() {
                        proc_macro_error::abort_call_site!(
                            "cannot use other bit specifiers with `single` bitaccess definition"
                        )
                    }
                }
            }
        }

        Ok(builder.build())
    }
}
