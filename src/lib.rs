#![feature(const_fn_trait_bound)]

use std::marker::PhantomData;

pub use bitaccess_macros::bitaccess;

pub struct FieldDefinition<B, F: FieldAccess<B>> {
    mask: B,
    _phantom: PhantomData<F>,
}

pub struct Field<B, F: FieldAccess<B>> {
    value: B,
    _phantom: PhantomData<F>,
}

pub trait FieldAccess<B> {
    fn to_raw(&self) -> B;
}

impl<B, F: FieldAccess<B>> FieldDefinition<B, F> {
    pub const fn new(mask: B) -> Self {
        Self {
            mask,
            _phantom: PhantomData,
        }
    }
}

impl<B, F: FieldAccess<B>> FieldDefinition<B, F>
where
    B: Copy,
{
    pub fn mask(&self) -> B {
        self.mask
    }
}

impl<B: Copy> FieldAccess<B> for B {
    fn to_raw(&self) -> B {
        *self
    }
}

impl<B, F: FieldAccess<B>> Field<B, F> {
    pub fn new(value: B) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }
}

impl<B, F: FieldAccess<B>> Field<B, F>
where
    B: Copy,
    F: From<B>,
{
    pub fn variant(&self) -> F {
        self.value.into()
    }
}

impl<B, F: FieldAccess<B>> Field<B, F>
where
    B: Copy,
{
    pub fn value(&self) -> B {
        self.value
    }
}

impl<B> From<B> for Field<B, B>
where
    B: Copy,
{
    fn from(base: B) -> Self {
        Self {
            value: base,
            _phantom: Default::default(),
        }
    }
}
