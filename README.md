# Bitaccess

> Unfortunately, works only on nightly rust :c

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
    let mut daif = Daif::new();
    println!("Daif IRQ: {:?}", daif.read(Daif::I).variant());
    daif.write(Daif::I, Mask::Unmasked);
    println!("Daif IRQ: {:?}", daif.read(Daif::I).variant());
}
```

For more specific examples, please refer to [tests](bitaccess/tests).

## WIP status
This is pretty much experiment. I'm gonna try to keep breaking changes to a minimum, but it's possible that there will be more than one pre-1.0.
