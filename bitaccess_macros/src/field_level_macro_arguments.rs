use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    Error,
    Lit,
    Pat,
    Token,
};

use crate::common;

mod kw {
    syn::custom_keyword!(offset);
    syn::custom_keyword!(size);
}

pub struct FieldLevelMacroArguments {
    pub offset: u64,
    pub size: u64,
}

#[derive(Default)]
pub struct FieldLevelMacroArgumentsBuilder {
    pub offset: Option<u64>,
    size: Option<u64>,
}

pub struct Position {
    pub lo: u64,
    pub len: u64,
}

pub enum FieldArgument {
    Offset(u64),
    Size(u64),
    Range(Position),
    Single(u64),
}

impl FieldLevelMacroArgumentsBuilder {
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

impl Parse for FieldArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::offset) {
            let _: kw::offset = input.parse()?;
            let _: Token![=] = input.parse()?;
            let lit = common::int_from_lit(input.parse::<Lit>()?)?;
            Ok(Self::Offset(lit))
        } else if lookahead.peek(kw::size) {
            let _: kw::size = input.parse()?;
            let _: Token![=] = input.parse()?;
            let lit = common::int_from_lit(input.parse::<Lit>()?)?;
            Ok(Self::Size(lit))
        } else {
            if let Ok(pat) = input.parse::<Pat>() {
                match pat {
                    Pat::Range(range) => return Ok(Self::Range(common::range_from_pat(&range)?)),
                    Pat::Lit(lit) => {
                        let lit: Lit = parse_quote! { #lit };
                        let single = common::int_from_lit(lit)?;
                        return Ok(Self::Single(single));
                    }
                    _ => {}
                }
            }

            Err(Error::new(input.span(), "unsupported field level argument"))
        }
    }
}

impl Parse for FieldLevelMacroArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        let _ = parenthesized!(content in input);
        let punct: Punctuated<FieldArgument, Token![,]> =
            content.parse_terminated(FieldArgument::parse)?;
        let mut builder = FieldLevelMacroArgumentsBuilder::default();

        for item in punct {
            match item {
                FieldArgument::Offset(offset) => {
                    let existing = builder.offset.replace(offset);
                    if existing.is_some() {
                        return Err(Error::new(
                            input.span(),
                            "multiple `offset` entries in field attribute",
                        ));
                    }
                }
                FieldArgument::Size(size) => {
                    let existing = builder.size.replace(size);
                    if existing.is_some() {
                        return Err(Error::new(
                            input.span(),
                            "multiple `size` entries in field attribute",
                        ));
                    }
                }
                FieldArgument::Range(range) => {
                    let ex1 = builder.size.replace(range.len);
                    let ex2 = builder.offset.replace(range.lo);
                    if ex1.is_some() || ex2.is_some() {
                        return Err(Error::new(
                            input.span(),
                            "cannot use other bit specifiers with `range` field definition",
                        ));
                    }
                }
                FieldArgument::Single(single) => {
                    let ex1 = builder.size.replace(1);
                    let ex2 = builder.offset.replace(single);
                    if ex1.is_some() || ex2.is_some() {
                        return Err(Error::new(
                            input.span(),
                            "cannot use other bit specifiers with `single` field definition",
                        ));
                    }
                }
            }
        }

        Ok(builder.build())
    }
}
