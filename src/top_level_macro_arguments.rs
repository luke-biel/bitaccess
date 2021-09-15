use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error,
    Expr,
    LitStr,
    Token,
    Type,
};

mod kw {
    syn::custom_keyword!(base_type);
    syn::custom_keyword!(kind);
    syn::custom_keyword!(write_via);
    syn::custom_keyword!(read_via);
}

pub struct TopLevelMacroArguments {
    pub base_type: Type,
    pub read: bool,
    pub write: bool,
    pub write_via: Option<Expr>,
    pub read_via: Option<Expr>,
}

#[derive(Default)]
pub struct TopLevelMacroArgumentsBuilder {
    base_type: Option<Type>,
    kind: Option<KindArg>,
    write_via: Option<Expr>,
    read_via: Option<Expr>,
}

pub struct KindArg {
    pub read: bool,
    pub write: bool,
}

pub enum TopLevelArgument {
    BaseType(Type),
    Kind(KindArg),
    WriteVia(Expr),
    ReadVia(Expr),
}

impl Parse for TopLevelArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::base_type) {
            let _: kw::base_type = input.parse()?;
            let _: Token![=] = input.parse()?;
            Ok(Self::BaseType(input.parse()?))
        } else if lookahead.peek(kw::kind) {
            let _: kw::kind = input.parse()?;
            let _: Token![=] = input.parse()?;
            let kind: Ident = input.parse()?;

            Ok(match kind.to_string().as_str() {
                "read_only" => Self::Kind(KindArg {
                    read: true,
                    write: false,
                }),
                "write_only" => Self::Kind(KindArg {
                    read: false,
                    write: true,
                }),
                "read_write" | "write_read" | "default" => Self::Kind(KindArg {
                    read: true,
                    write: true,
                }),
                _ => return Err(Error::new(kind.span(), "unsupported access kind")),
            })
        } else if lookahead.peek(kw::read_via) {
            let _: kw::read_via = input.parse()?;
            let _: Token![=] = input.parse()?;
            let ins: LitStr = input.parse()?;

            let expr = syn::parse_str(&ins.value())?;

            Ok(Self::ReadVia(expr))
        } else if lookahead.peek(kw::write_via) {
            let _: kw::write_via = input.parse()?;
            let _: Token![=] = input.parse()?;
            let ins: LitStr = input.parse()?;

            let expr = syn::parse_str(&ins.value())?;

            Ok(Self::WriteVia(expr))
        } else {
            Err(Error::new(input.span(), "unsupported top level argument"))
        }
    }
}

impl Parse for TopLevelMacroArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let punct: Punctuated<TopLevelArgument, Token![,]> =
            input.parse_terminated(TopLevelArgument::parse)?;

        let mut builder = TopLevelMacroArgumentsBuilder::default();

        for item in punct {
            match item {
                TopLevelArgument::BaseType(base_type) => {
                    let ex = builder.base_type.replace(base_type);
                    if ex.is_some() {
                        return Err(Error::new(
                            input.span(),
                            "multiple `base_type` entries in top level attribute",
                        ));
                    }
                }
                TopLevelArgument::Kind(kind) => {
                    let ex = builder.kind.replace(kind);
                    if ex.is_some() {
                        return Err(Error::new(
                            input.span(),
                            "multiple `kind` entries in top level attribute",
                        ));
                    }
                }
                TopLevelArgument::WriteVia(write_via) => {
                    let ex = builder.write_via.replace(write_via);
                    if ex.is_some() {
                        return Err(Error::new(
                            input.span(),
                            "multiple `write_via` entries in top level attribute",
                        ));
                    }
                }
                TopLevelArgument::ReadVia(read_via) => {
                    let ex = builder.read_via.replace(read_via);
                    if ex.is_some() {
                        return Err(Error::new(
                            input.span(),
                            "multiple `read_via` entries in top level attribute",
                        ));
                    }
                }
            }
        }

        Ok(builder.build())
    }
}

impl TopLevelMacroArgumentsBuilder {
    fn build(self) -> TopLevelMacroArguments {
        let base_type = if let Some(base_type) = self.base_type {
            base_type
        } else {
            proc_macro_error::abort_call_site!("missing `base_type` on bitaccess enum")
        };
        let KindArg { read, write } = self.kind.unwrap_or(KindArg { read: true, write: true });
        if (self.write_via.is_some() && self.read_via.is_none()) || (self.write_via.is_none() && self.read_via.is_some()) {
            proc_macro_error::abort_call_site!("must specify either both `write_via` and `read_via` or none")
        }

        TopLevelMacroArguments {
            base_type,
            read,
            write,
            write_via: self.write_via,
            read_via: self.read_via,
        }
    }
}
