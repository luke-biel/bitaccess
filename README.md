# Bitaccess

Small crate that substitutes for lack of bitfield accessors in rust language.

Example usage:

```rust
#![feature(asm)]

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
    #[bit(9)]
    #[variants(Mask)] D,
    #[bit(8)]
    #[variants(Mask)] A,
    #[bit(7)]
    #[variants(Mask)] I,
    #[bit(6)]
    #[variants(Mask)] F,
}

/// DAIF is an ARM register, so this example is not really suited for running on all machines.
/// It's here just to show power of the macro.
fn alter_daif() {
    println!("Daif IRQ: {:?}", Daif.read(Daif::I).variant());
    Daif.write(Daif::I, Mask::Unmasked);
    println!("Daif IRQ: {:?}", Daif.read(Daif::I).variant());
}
```

# Features

## Inline register
Allows you to create an object stored in normal memory, be it stack or heap (likely via some dereference wrapper).

To create such structure, you have to create an enum, like so:

```rust
use bitaccess::{bitaccess, FieldAccess};

#[derive(FieldAccess)]
#[field_access(u64)]
enum VariantThird {
    Val1 = 0x1,
    Val2 = 0x3,
    Val3 = 0xf,
}

#[bitaccess(
    base_type = u64,
    kind = read_write
)]
#[repr(C)]
enum MyRegister {
    #[bits(0..=3)]
    FirstDescriptor,
    #[bit(4)]
    #[variants(On => 1, Off => 0)]
    SecondDescriptor,
    #[bitaccess(5..9)]
    #[variants(VariantThird)]
    ThirdDescriptor,
}
```

### Base_type
Any integer type.
> putting any other type than basic integer types is unstable, however may work

### Kind
Allowed options:

* read_only
* write_only
* read_write | write_read | default

Depending on the chosen option resulting code may provide ReadBits, WriteBits or both implementations. Field can be
skipped, which will result in read_write register.

### Additional attributes on main enum
All attributes past bitaccess will be copied to resulting **struct**
(yeah, this enum transforms into struct under the hood)

### Bits / Bit / Bitaccess
Field attribute that declares which bits are part of given field.

Accepts 3 forms of declaration:
#### explicit
`#[bits(offset = N, size = S)]`
where both N and S are expressions evaluable to *base_type*

#### range
Both Range and RangeInclusive are acceptable: `0..4` & `0..=3`

#### single
For single bit accessors `#[bit(N)]` is allowed.

### Variants
Fields may come in automatically cast variants (like `VariantThird` above). Bitaccess supports two ways of declaring
such access:

#### inline
Comma separated list of `Identifier => Value` pairs. Variants will be accessible from enum with field identifier for a
name, eg. in case from above, we'd call `SecondDescriptor::On`.

#### external
Specifying just type in `#[variants(Type)]` will use that type for field access.
`Type` has to derive `FieldAccess` trait and specify `#[field_access(N)]` attribute, where `N` has to match `base_type`
on main enum.

## Global register

```rust
#[derive(FieldAccess)]
#[field_access(u64)]
pub enum ExceptionLevel {
    EL0 = 0b00,
    EL1 = 0b01,
    EL2 = 0b10,
    EL3 = 0b11,
}

#[bitaccess(
    base_type = u64,
    kind = read_only,
    read_via = r#"unsafe { asm!("mrs {}, currentel", out(reg) value, options(nomem, nostack)) }"#
)]
pub enum CurrentEl {
    #[bits(2..4)]
    #[variants(ExceptionLevel)]
    Value,
}
```

Global registers are created when `read_via` or `write_via` attributes are provided to bitaccess macro. All other
attributes behave as in `Inline register`.

### Read_via | write_via
Rust instructions provided within string. For some other use examples, you may check tests.
