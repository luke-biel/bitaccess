#![deny(warnings)]

use bitaccess::bitaccess;

#[bitaccess(base_type = u64, kind = default)]
pub enum Variantless {}

#[test]
fn can_set_and_get_value_with_no_warnings() {
    let mut v = Variantless::new();
    v.set(1);
    assert_eq!(v.get(), 1);
}
