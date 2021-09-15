use syn::{parse_quote, Error, Lit, PatRange, RangeLimits};

use crate::field_level_macro_arguments::Position;

pub fn range_from_pat(input: &PatRange) -> syn::Result<Position> {
    let PatRange { lo, limits, hi, .. } = input;

    let lo: Lit = parse_quote! { #lo };
    let hi: Lit = parse_quote! { #hi };

    let lo = int_from_lit(lo)?;
    let hi = int_from_lit(hi)?;

    match limits {
        RangeLimits::HalfOpen(_) => Ok(Position { lo, len: hi - lo }), // 0 sized bitfields aren't supported anyway
        RangeLimits::Closed(_) => Ok(Position {
            lo,
            len: hi - lo + 1,
        }),
    }
}

pub fn int_from_lit(lit: Lit) -> syn::Result<u64> {
    match lit {
        Lit::Int(lit_int) => lit_int.base10_parse::<u64>(),
        _ => Err(Error::new(lit.span(), "invalid value for parameter")),
    }
}
