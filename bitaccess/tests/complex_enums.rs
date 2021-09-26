use bitaccess::{bitaccess, FieldAccess};

#[bitaccess(base_type = u64)]
pub enum Variants {
    #[variants(
        FirstOn  => 0b001,
        SecondOn => 0b010,
        ThirdOn  => 0b100,
    )]
    #[bits(0..3)]
    ThreeBits,
}

#[derive(FieldAccess, PartialEq, Debug)]
#[field_access(u32)]
pub enum FourBitsVariant {
    Case1 = 0,
    Case2 = 8,
    Case3 = 15,
}

#[bitaccess(base_type = u32)]
pub enum ExternalVariants {
    #[variants(FourBitsVariant)]
    #[bits(0..4)]
    FourBits,
}

#[test]
fn can_use_variants() {
    let mut r = Variants::new();
    r.write(Variants::ThreeBits, ThreeBits::FirstOn);
    assert_eq!((&r.read(Variants::ThreeBits)).variant(), ThreeBits::FirstOn);
}

#[test]
fn can_use_external_variants() {
    let mut r = ExternalVariants::new();
    r.write(ExternalVariants::FourBits, FourBitsVariant::Case3);
    assert_eq!(
        r.read(ExternalVariants::FourBits).variant(),
        FourBitsVariant::Case3,
    )
}
