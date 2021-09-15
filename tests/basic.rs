use bitaccess::bitaccess;

#[bitaccess(base_type = u64, kind = default)]
pub enum Register {
    #[bitaccess(offset = 0, size = 4)]
    F1,
    #[bitaccess(4..=7)]
    F2,
    #[bitaccess(8..12)]
    F3,
    #[bitaccess(2)]
    ThirdBit,
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
    r.write(Register::F1, 0b0111);
    r.write(Register::F2, 0b1000);
    r.write(Register::F3, 0b1111);
    assert_eq!(r.read(Register::F1), 0b111);
    assert_eq!(r.read(Register::F2), 0b1000);
    assert_eq!(r.read(Register::F3), 0b1111);
    assert_eq!(r.get_raw(), 0b1111_1000_0111);
    assert_eq!(r.read(Register::ThirdBit), 1);
}
