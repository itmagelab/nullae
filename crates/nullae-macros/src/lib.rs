use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Indexable, attributes(index))]
pub fn derive_indexable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut field_data = Vec::new();

    if let Data::Struct(data_struct) = &input.data
        && let Fields::Named(fields_named) = &data_struct.fields
    {
        for field in &fields_named.named {
            for attr in &field.attrs {
                if attr.path().is_ident("index")
                    && let Some(ident) = &field.ident
                {
                    // Check if field type is Option
                    let is_option = if let syn::Type::Path(type_path) = &field.ty {
                        type_path
                            .path
                            .segments
                            .last()
                            .map(|seg| seg.ident == "Option")
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    field_data.push((ident.clone(), ident.to_string(), is_option));
                }
            }
        }
    }

    // Generate code for each field based on its type
    let index_code = field_data.iter().map(|(ident, name, is_option)| {
        if *is_option {
            quote! {
                if let Some(value) = &self.#ident {
                    let item = Item::new(#name, value, &self.hash)?;
                    index.push(item);
                }
            }
        } else {
            quote! {
                let item = Item::new(#name, &self.#ident, &self.hash)?;
                index.push(item);
            }
        }
    });

    let expanded = quote! {
        impl Indexable for #name {
            fn index(&self) -> anyhow::Result<Index> {
                let mut index = Index::new();
                #(#index_code)*
                Ok(index)
            }
        }
    };

    expanded.into()
}

#[proc_macro_derive(Entity)]
pub fn entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let variant_name = name;

    let expanded = quote! {
        impl From<#name> for Entity {
            fn from(value: #name) -> Self {
                let metadata = Metadata::new();
                let kind = EntityKind::#variant_name {
                    inner: Box::new(value),
                };
                Self {
                    metadata, kind
                }
            }
        }

        impl TryFrom<Entity> for #name {
            type Error = anyhow::Error;

            fn try_from(entity: Entity) -> Result<Self, Self::Error> {
                if let EntityKind::#variant_name { inner } = entity.kind {
                    Ok(*inner)
                } else {
                    anyhow::bail!("Invalid entity type, expected {}", stringify!(#variant_name))
                }
            }
        }
    };

    TokenStream::from(expanded)
}
