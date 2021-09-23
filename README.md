# Bitaccess

> Currently, works only on nightly rust :c

A small crate that helps with lack of bitfield accessors in rust language.
Example usage:
```rust
use bitaccess::{bitaccess, FieldAccess};

#[derive(FieldAccess, Debug)]
#[field_access(u64)]
pub enum Mask {
    Unmasked = 0,
    Masked = 1,
}

#[bitaccess(
    base_type = u64,
    kind = read_write,
    read_via = r#"unsafe { asm!("mrs {}, daif", out(reg) value, options(nostack, nomem)); }"#,
    write_via = r#"unsafe { asm!("msr daif, {}", in(reg) value, options(nostack, nomem)); }"#
)]
pub enum Daif {
    #[bit(9)] #[variants(Mask)] D,
    #[bit(8)] #[variants(Mask)] A,
    #[bit(7)] #[variants(Mask)] I,
    #[bit(6)] #[variants(Mask)] F,
}

/// DAIF is an ARM register, so this example is not really suited for running on all machines.
/// It's here just to show power of the macro.
fn main() {
    let mut daif = Daif::new_global();
    println!("Daif IRQ: {:?}", daif.read(Daif::I));
    daif.write(Daif::I, Mask::Unmasked);
    println!("Daif IRQ: {:?}", daif.read(Daif::I));
}
```
