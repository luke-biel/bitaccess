use bitaccess::{bitaccess, ReadBits, WriteBits};

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

#[test]
fn initializes_to_zero() {
    let r = Register::new();
    assert_eq!(r.get(), 0)
}

#[test]
fn can_init_value() {
    let r = Register::from_value(0b1100 + (0b1101 << 4) + (0b1001 << 8));
    assert_eq!(r.get(), 0b1001_1101_1100)
}

#[test]
fn can_set_value() {
    let mut r = Register::new();
    r.set(0b1100 + (0b1101 << 4) + (0b1001 << 8));
    assert_eq!(r.get(), 0b1001_1101_1100)
}

#[test]
fn can_read_bits_value() {
    let r = Register::from_value(0b1100 + (0b1101 << 4) + (0b1001 << 8));
    assert_eq!(r.read(Register::F1).value(), 0b1100);
    assert_eq!(r.read(Register::F2).value(), 0b1101);
    assert_eq!(r.read(Register::F3).value(), 0b1001);
    assert_eq!(r.read(Register::ThirdBit).value(), 1);
}

#[test]
fn can_write_bits_value() {
    let mut r = Register::new();
    r.write(Register::F1, 0b0111u64);
    r.write(Register::F2, 0b1000u64);
    r.write(Register::F3, 0b1111u64);
    assert_eq!(r.read(Register::F1).value(), 0b111);
    assert_eq!(r.read(Register::F2).value(), 0b1000);
    assert_eq!(r.read(Register::F3).value(), 0b1111);
    assert_eq!(r.get(), 0b1111_1000_0111);
    assert_eq!(r.read(Register::ThirdBit).value(), 1);
}

#[test]
fn propagates_top_level_attributes() {
    let v1 = Register::from_value(123);
    let v2 = v1.clone();
    assert_eq!(v1, v2);
}

#[test]
fn can_use_custom_read_via() {
    unsafe { GLOBAL_TEST = 0 };
    let mut r = ViaTests::new();
    r.write(ViaTests::BitZero, 0b1u64);
    assert_eq!(r.read(ViaTests::BitZero).value(), 1);
    assert_eq!(unsafe { GLOBAL_TEST }, 1);
}

#[test]
fn can_call_directly() {
    unsafe { GLOBAL_TEST = 0 };
    ViaTests.write(ViaTests::BitZero, 1);
    assert_eq!(ViaTests.read(ViaTests::BitZero).value(), 1)
}

#[test]
fn can_zero_fields() {
    let mut r = Register::from_value(0b1111_0000);
    r.write(Register::F2, 0);
    assert_eq!(r.read(Register::F2).value(), 0);
}

#[test]
fn set_overwrites_value() {
    let mut r = Register::from_value(0b1111_0000);
    r.set(0);
    r.set(1);
    r.set(0b10);
    assert_eq!(r.get(), 0b10);
}
