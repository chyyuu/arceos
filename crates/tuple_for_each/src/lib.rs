use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DataStruct, DeriveInput, Error, Index};

#[proc_macro_attribute]
pub fn tuple_for_each(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(item as DeriveInput);
    if let Data::Struct(strct) = &ast.data {
        impl_for_each(attr, &ast, strct)
    } else {
        Error::new_spanned(
            ast,
            "attribute `tuple_for_each` can only be attached to tuple structs",
        )
        .to_compile_error()
    }
    .into()
}

fn impl_for_each(
    base: TokenStream,
    ast: &DeriveInput,
    strct: &DataStruct,
) -> proc_macro2::TokenStream {
    if base.is_empty() {
        return Error::new(
            proc_macro2::Span::call_site(),
            "expect a base trait in attribute `tuple_for_each`: e.g. #[tuple_for_each(BaseTrait)]",
        )
        .to_compile_error();
    }

    let struct_name = &ast.ident;
    let base_trait = format_ident!("{}", base.to_string());

    let mut field_num = vec![];
    let mut call_each_fields = vec![];
    let mut call_each_fields_mut = vec![];
    strct.fields.iter().enumerate().for_each(|(i, field)| {
        let idx = Index::from(i);
        let cfg_attrs = &field
            .attrs
            .iter()
            .filter(|attr| {
                attr.path
                    .get_ident()
                    .filter(|i| (*i).to_string().eq("cfg"))
                    .is_some()
            })
            .collect::<Vec<_>>();

        field_num.push(quote!( #(#cfg_attrs)* { l += 1; } ));
        call_each_fields.push(quote!( #(#cfg_attrs)* { f(&self.#idx); } ));
        call_each_fields_mut.push(quote!( #(#cfg_attrs)* { f(&mut self.#idx); } ));
    });

    quote! {
        #ast
        impl #struct_name {
            /// Number of fields in the tuple.
            pub const fn len(&self) -> usize {
                let mut l = 0;
                #(#field_num)*
                l
            }

            // Whether the tuple has no field.
            pub const fn is_empty(&self) -> bool {
                self.len() == 0
            }

            /// Calls a closure on each field of the tuple.
            pub fn for_each<F>(&self, f: F) where F: Fn(&dyn #base_trait) {
                #(#call_each_fields)*
            }

            /// Calls a closure on each item of the tuple with mutable reference.
            pub fn for_each_mut<F>(&mut self, f: F) where F: Fn(&mut dyn #base_trait) {
                #(#call_each_fields_mut)*
            }
        }
    }
}
