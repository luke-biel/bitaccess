use bitaccess::bitaccess;

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

#[test]
fn can_use_variants() {
    let mut r = Variants::zero();
    r.write(Variants::ThreeBits, variants::ThreeBits::FirstOn);
    assert_eq!(
        (&r.read(Variants::ThreeBits)).variant(),
        variants::ThreeBits::FirstOn
    );
}
