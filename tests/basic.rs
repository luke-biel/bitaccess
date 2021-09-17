use bitaccess::bitaccess;

#[bitaccess(base_type = u64, kind = default)]
#[derive(Clone, Debug, PartialEq)]
pub enum Register {
    #[bitaccess(offset = 0, size = 4)]
    F1,
    #[bits(4..=7)]
    F2,
    #[bits(8..12)]
    F3,
    #[bit(2)]
    ThirdBit,
}

// Don't do this at home
static mut GLOBAL_TEST: u64 = 0;

#[bitaccess(
    base_type = u64,
    kind = read_write,
    read_via = "unsafe { value = crate::GLOBAL_TEST }",
    write_via = "unsafe { crate::GLOBAL_TEST = value }"
)]
pub enum ViaTests {
    #[bit(0)]
    BitZero,
}

#[bitaccess(base_type = u64)]
pub enum Variants {
    #[bits(0..3)]
    #[variants(
        FirstOn  => 0b001,
        SecondOn => 0b010,
        ThirdOn  => 0b100,
    )]
    ThreeBits,
}

#[test]
fn initializes_to_zero() {
    let r = Register::zero();
    assert_eq!(r.get_raw(), 0)
}

#[test]
fn can_init_value() {
    let r = Register::new(0b1100 + (0b1101 << 4) + (0b1001 << 8));
    assert_eq!(r.get_raw(), 0b1001_1101_1100)
}

#[test]
fn can_read_bits_value() {
    let r = Register::new(0b1100 + (0b1101 << 4) + (0b1001 << 8));
    assert_eq!(r.read(Register::F1), 0b1100);
    assert_eq!(r.read(Register::F2), 0b1101);
    assert_eq!(r.read(Register::F3), 0b1001);
    assert_eq!(r.read(Register::ThirdBit), 1);
}

#[test]
fn can_write_bits_value() {
    let mut r = Register::zero();
    r.write(Register::F1, 0b0111u64);
    r.write(Register::F2, 0b1000u64);
    r.write(Register::F3, 0b1111u64);
    assert_eq!(r.read(Register::F1), 0b111);
    assert_eq!(r.read(Register::F2), 0b1000);
    assert_eq!(r.read(Register::F3), 0b1111);
    assert_eq!(r.get_raw(), 0b1111_1000_0111);
    assert_eq!(r.read(Register::ThirdBit), 1);
}

#[test]
fn propagates_top_level_attributes() {
    let v1 = Register::new(123);
    let v2 = v1.clone();
    assert_eq!(v1, v2);
}

#[test]
fn can_use_custom_read_via() {
    let mut r = ViaTests::new_global();
    r.write(ViaTests::BitZero, 0b1u64);
    assert_eq!(r.read(ViaTests::BitZero), 1);
    assert_eq!(unsafe { GLOBAL_TEST }, 1);
}

#[test]
fn can_use_variants() {
    let mut r = Variants::zero();
    r.write(Variants::ThreeBits, variants::ThreeBits::FirstOn);
    assert_eq!(r.read(Variants::ThreeBits), variants::ThreeBits::FirstOn);
}
