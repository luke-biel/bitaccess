use convert_case::{Case, Casing};
use proc_macro2::{TokenStream as TokenStream2, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse2, Attribute, Ident, ItemEnum, Visibility};

use crate::{
    bit_field::BitField,
    top_level_macro_arguments::{
        GlobalReadOnly,
        GlobalReadWrite,
        Implementation,
        KindArg,
        TopLevelMacroArguments,
    },
};

pub struct BitAccess {
    top_level_arguments: TopLevelMacroArguments,
    struct_identifier: Ident,
    struct_visibility: Visibility,
    fields: Vec<BitField>,
    attributes: Vec<Attribute>,
}

impl BitAccess {
    pub fn new(args: TokenStream2, item: ItemEnum) -> syn::Result<Self> {
        Ok(Self {
            top_level_arguments: parse2::<TopLevelMacroArguments>(args)?,
            struct_identifier: item.ident,
            struct_visibility: item.vis,
            fields: BitField::many(item.variants)?,
            attributes: item.attrs,
        })
    }

    pub fn into_token_stream(self) -> TokenStream2 {
        let enum_field_names = self.enum_field_names();
        let private_module_ident = self.private_module_ident();
        let private_struct_ident = self.private_struct_ident();
        let main_struct_const_fields = self.main_struct_const_fields();
        let field_inline_variant_enums = self.field_inline_variant_enums();
        let main_struct_accessors = self.main_struct_accessors(&enum_field_names);
        let representation_ident = self.representation_struct_ident();
        let private_struct = self.private_struct_definition(&private_struct_ident);
        let main_struct_constructors =
            self.constructors(&private_struct_ident, &representation_ident);
        let immutable_representation_struct =
            self.immutable_representation_struct(&representation_ident);
        let immutable_representation_private =
            self.immutable_representation_private(&representation_ident, &enum_field_names);

        let read_raw_fn = self.read_raw_fn();
        let write_raw_fn = self.write_raw_fn();

        let vis = &self.struct_visibility;
        let ident = &self.struct_identifier;
        let attributes = &self.attributes;

        let private_api = quote! {
            #vis mod #private_module_ident {
                #private_struct

                impl super::#ident {
                    #main_struct_constructors
                    #main_struct_accessors

                    #read_raw_fn
                    #write_raw_fn
                }

                #immutable_representation_private
            }
        };

        let public_api = quote! {
            #(#attributes)*
            #vis struct #ident {
                inner: #private_module_ident::#private_struct_ident,
            }

            #[allow(non_upper_case_globals)]
            impl #ident {
                #(#main_struct_const_fields)*
            }

            #(#field_inline_variant_enums)*

            #immutable_representation_struct
        };

        quote! {
            #public_api
            #private_api
        }
    }

    fn private_struct_definition(&self, private_struct_ident: &Ident) -> TokenStream {
        let base_type = &self.top_level_arguments.base_type;
        let private_value_holder =
            if self.top_level_arguments.is_read() && self.top_level_arguments.is_write() {
                quote! { value: #base_type }
            } else {
                TokenStream2::new()
            };

        let attributes = &self.attributes;

        quote! {
            #[allow(non_camel_case_types)]
            #(#attributes)*
            pub(super) struct #private_struct_ident {
                #private_value_holder
            }
        }
    }

    fn representation_struct_ident(&self) -> Ident {
        Ident::new(
            &format!(
                "{}Representation",
                self.struct_identifier.to_string().to_case(Case::Pascal)
            ),
            self.struct_identifier.span(),
        )
    }

    fn main_struct_accessors(&self, enum_field_names: &[Ident]) -> TokenStream2 {
        let read_impl = self.read_impl(enum_field_names.iter());
        let write_impl = self.write_impl(enum_field_names.iter());

        quote! {
            #read_impl
            #write_impl
        }
    }

    fn field_inline_variant_enums(&self) -> Vec<TokenStream2> {
        self.fields
            .iter()
            .map(|field| {
                field
                    .extra_enum_access(&self.struct_visibility, &self.top_level_arguments.base_type)
            })
            .collect()
    }

    fn main_struct_const_fields(&self) -> Vec<TokenStream2> {
        self.fields
            .iter()
            .map(|field| {
                field.const_enum(&self.struct_visibility, &self.top_level_arguments.base_type)
            })
            .collect()
    }

    fn private_struct_ident(&self) -> Ident {
        Ident::new(
            &format!("{}Private", &self.struct_identifier),
            self.struct_identifier.span(),
        )
    }

    fn private_module_ident(&self) -> Ident {
        Ident::new(
            &self.struct_identifier.to_string().to_case(Case::Snake),
            self.struct_identifier.span(),
        )
    }

    fn enum_field_names(&self) -> Vec<Ident> {
        self.fields
            .iter()
            .map(|field| Ident::new(&field.ident.to_string(), field.ident.span()))
            .collect()
    }

    fn read_impl<'a>(&self, enum_field_names: impl Iterator<Item = &'a Ident>) -> TokenStream2 {
        if self.top_level_arguments.is_read() {
            let readers: Vec<_> = self.fields.iter().map(|item| item.reader()).collect();
            let vis = &self.struct_visibility;
            let base_type = &self.top_level_arguments.base_type;

            quote! {
                #vis fn read<F: bitaccess::FieldAccess<#base_type>>(
                    &self,
                    bits: bitaccess::FieldDefinition<#base_type, F>
                ) -> bitaccess::Field<#base_type, F> {
                    let value = self.read_raw();
                    bitaccess::Field::new(match bits.mask() {
                        #(_ if Self::#enum_field_names.mask() == bits.mask() => #readers,)*
                        _ => panic!("use provided consts to read from register"),
                    })
                }
            }
        } else {
            TokenStream2::new()
        }
    }

    fn immutable_representation_read_impl<'a>(
        &self,
        enum_field_names: impl Iterator<Item = &'a Ident>,
    ) -> TokenStream2 {
        if self.top_level_arguments.is_read() {
            let readers: Vec<_> = self.fields.iter().map(|item| item.reader()).collect();
            let vis = &self.struct_visibility;
            let base_type = &self.top_level_arguments.base_type;
            let ident = &self.struct_identifier;

            quote! {
                #vis fn read<F: bitaccess::FieldAccess<#base_type>>(
                    &self,
                    bits: bitaccess::FieldDefinition<#base_type, F>
                ) -> bitaccess::Field<#base_type, F> {
                    let value = self.read_raw();
                    bitaccess::Field::new(match bits.mask() {
                        #(_ if super::#ident::#enum_field_names.mask() == bits.mask() => #readers,)*
                        _ => panic!("use provided consts to read from register"),
                    })
                }
            }
        } else {
            TokenStream2::new()
        }
    }

    fn write_impl<'a>(&self, enum_field_names: impl Iterator<Item = &'a Ident>) -> TokenStream2 {
        if self.top_level_arguments.is_write() {
            let writers: Vec<_> = self.fields.iter().map(|item| item.writer()).collect();
            let vis = &self.struct_visibility;
            let base_type = &self.top_level_arguments.base_type;

            quote! {
                #vis fn write<F: bitaccess::FieldAccess<#base_type>>(
                    &mut self,
                    bits: bitaccess::FieldDefinition<#base_type, F>,
                    new_value: impl Into<bitaccess::Field<#base_type, F>>
                ) {
                    let new_value: bitaccess::Field<#base_type, F> = new_value.into();
                    match bits.mask() {
                        #(_ if Self::#enum_field_names.mask() == bits.mask() => #writers,)*
                        _ => panic!("use provided consts to write to register"),
                    }
                }
            }
        } else {
            TokenStream2::new()
        }
    }
    fn constructors(
        &self,
        private_struct_ident: &Ident,
        representation_ident: &Ident,
    ) -> TokenStream2 {
        let vis = &self.struct_visibility;

        match &self.top_level_arguments.implementation {
            Implementation::Inline(_) => {
                let base_type = &self.top_level_arguments.base_type;
                quote! {
                    #vis fn new() -> Self {
                        Self { inner: #private_struct_ident { value: 0 } }
                    }

                    #vis fn from_value(value: #base_type) -> Self {
                        Self { inner: #private_struct_ident { value, }, }
                    }
                }
            }
            _ => {
                quote! {
                    #vis fn new() -> Self {
                        Self { inner: #private_struct_ident {}, }
                    }

                    #vis fn fetch() -> super::#representation_ident {
                        let me = Self::new_global();
                        super::#representation_ident::new(me.read_raw())
                    }
                }
            }
        }
    }
    fn immutable_representation_struct(
        &self,
        representation_ident: &Ident,
    ) -> Option<TokenStream2> {
        let vis = &self.struct_visibility;
        let base_type = &self.top_level_arguments.base_type;

        match self.top_level_arguments.implementation {
            Implementation::Inline(_) => None,
            _ => Some(quote! {
                #vis struct #representation_ident {
                    value: #base_type,
                }
            }),
        }
    }
    fn immutable_representation_private(
        &self,
        representation_ident: &Ident,
        enum_field_names: &[Ident],
    ) -> Option<TokenStream2> {
        let base_type = &self.top_level_arguments.base_type;
        let read_impl = self.immutable_representation_read_impl(enum_field_names.iter());

        match self.top_level_arguments.implementation {
            Implementation::Inline(_) => None,
            _ => Some(quote! {
                    impl super::#representation_ident {
                        pub fn new(value: #base_type) -> Self {
                            Self {
                                value,
                            }
                        }

                        fn read_raw(&self) -> #base_type {
                            self.value
                        }

                        #read_impl
                    }
            }),
        }
    }
    fn read_raw_fn(&self) -> Option<TokenStream2> {
        let read_via = match &self.top_level_arguments.implementation {
            Implementation::Inline(KindArg { read, .. }) if *read => {
                Some(quote! { value = self.inner.value })
            }
            Implementation::GlobalReadOnly(box GlobalReadOnly { read_via }) => {
                Some(read_via.to_token_stream())
            }
            Implementation::GlobalReadWrite(box GlobalReadWrite { read_via, .. }) => {
                Some(read_via.to_token_stream())
            }
            Implementation::Inline(_) => None,
        };
        read_via.map(|read_via| {
            let base_type = &self.top_level_arguments.base_type;
            let vis = &self.struct_visibility;

            quote! {
                fn read_raw(&self) -> #base_type {
                    let value: #base_type;
                    #read_via;
                    value
                }

                #vis fn get(&self) -> #base_type {
                    self.read_raw()
                }
            }
        })
    }

    fn write_raw_fn(&self) -> Option<TokenStream2> {
        let write_via = match &self.top_level_arguments.implementation {
            Implementation::Inline(KindArg { write, .. }) if *write => {
                Some(quote! { self.inner.value = value })
            }
            Implementation::GlobalReadWrite(box GlobalReadWrite { write_via, .. }) => {
                Some(write_via.to_token_stream())
            }
            Implementation::Inline(_)
            | Implementation::GlobalReadOnly(box GlobalReadOnly { .. }) => None,
        };
        write_via.map(|write_via| {
            let base_type = &self.top_level_arguments.base_type;
            let vis = &self.struct_visibility;

            quote! {
                fn write_raw(&mut self, new_value: #base_type) {
                    let old_value = self.read_raw();
                    let value = old_value | new_value;
                    #write_via
                }

                #vis fn set(&mut self, new_value: #base_type) {
                    self.write_raw(new_value)
                }
            }
        })
    }
}
