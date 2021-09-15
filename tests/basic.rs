use bitaccess::bitaccess;

#[bitaccess(base_type = u64)]
pub enum Register {
    #[bitaccess(offset = 0, size = 3)]
    F1,
    #[bitaccess(offset = 3, size = 6)]
    F2,
}

#[test]
fn initializes_to_zero() {
    let r = Register::zero();
    assert_eq!(r.get_raw(), 0)
}

#[test]
fn can_init_value() {
    let r = Register::new(0b110 + (0b10_1100 << 3));
    assert_eq!(r.get_raw(), 0b1_0110_0110)
}

#[test]
fn can_read_bits_value() {
    let r = Register::new(0b110 + (0b10_1100 << 3));
    assert_eq!(r.read(Register::F1), 0b110);
    assert_eq!(r.read(Register::F2), 0b10_1100)
}

#[test]
fn can_write_bits_value() {
    let mut r = Register::zero();
    r.write(Register::F1, 0b101);
    r.write(Register::F2, 0b11_1000);
    assert_eq!(r.read(Register::F1), 0b101);
    assert_eq!(r.read(Register::F2), 0b11_1000);
    assert_eq!(r.get_raw(), 0b1_1100_0101)
}
