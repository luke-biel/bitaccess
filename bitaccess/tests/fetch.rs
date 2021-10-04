use bitaccess::{bitaccess, FieldAccess};

// Don't do this at home
static mut GLOBAL_TEST: u64 = 0;

#[derive(FieldAccess, Debug, PartialEq)]
#[field_access(u64)]
pub enum ExternalVariant {
    Fib5 = 8,
    Fib6 = 13,
}

#[bitaccess(
    base_type = u64,
    kind = read_write,
    read_via = "unsafe { value = crate::GLOBAL_TEST }",
    write_via = "unsafe { crate::GLOBAL_TEST = value }"
)]
pub enum ViaTests {
    #[bit(0)]
    BitZero,
    #[bit(1)]
    BitOne,
    #[bits(2..4)]
    BitsTwoThree,
    #[bits(4..8)]
    #[variants(
        Fib1 => 1,
        Fib2 => 2,
        Fib3 => 3,
        Fib4 => 5
    )]
    InlineVariants,
    #[bits(8..12)]
    #[variants(ExternalVariant)]
    ExternalVariants,
}

#[test]
fn fetches_whole_struct() {
    unsafe {
        GLOBAL_TEST = 0b1101_0011_0111;
    }
    let val = ViaTests::fetch();
    unsafe {
        GLOBAL_TEST = 0;
    }

    assert_eq!(val.read(ViaTests::BitZero).value(), 1);
    assert_eq!(val.read(ViaTests::BitOne).value(), 1);
    assert_eq!(val.read(ViaTests::BitsTwoThree).value(), 0b01);
    assert_eq!(
        val.read(ViaTests::InlineVariants).variant(),
        InlineVariants::Fib3
    );
    assert_eq!(
        val.read(ViaTests::ExternalVariants).variant(),
        ExternalVariant::Fib6
    );
}

#[test]
fn provides_write_api() {
    unsafe {
        GLOBAL_TEST = 0b1101_0011_0111;
    }
    let mut val = ViaTests::fetch();

    assert_eq!(val.read(ViaTests::BitZero).value(), 1);
    assert_eq!(val.read(ViaTests::BitOne).value(), 1);
    assert_eq!(val.read(ViaTests::BitsTwoThree).value(), 0b01);
    assert_eq!(
        val.read(ViaTests::InlineVariants).variant(),
        InlineVariants::Fib3
    );
    assert_eq!(
        val.read(ViaTests::ExternalVariants).variant(),
        ExternalVariant::Fib6
    );
    val.write_to_cache(ViaTests::BitZero, 0);
    assert_eq!(val.read(ViaTests::BitZero).value(), 0);

    let val = ViaTests::fetch();
    assert_eq!(val.read(ViaTests::BitZero).value(), 1);
}
