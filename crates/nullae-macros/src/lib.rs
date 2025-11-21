use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Indexable, attributes(index))]
pub fn derive_indexable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut field_idents = Vec::new();

    if let Data::Struct(data_struct) = &input.data
        && let Fields::Named(fields_named) = &data_struct.fields
    {
        for field in &fields_named.named {
            for attr in &field.attrs {
                if attr.path().is_ident("index")
                    && let Some(ident) = &field.ident
                {
                    field_idents.push(ident.clone());
                }
            }
        }
    }


    let field_names: Vec<_> = field_idents.iter().map(|i| i.to_string()).collect();

    let expanded = quote! {
        impl Indexable for #name {
            fn index(&self) -> anyhow::Result<Index> {
                let mut index = Index::new();
                #(
                    let item = Item::new(#field_names, &self.#field_idents, &self.hash)?;
                    index.push(item);
                )*
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
                    inner: value,
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
                    Ok(inner)
                } else {
                    anyhow::bail!("Invalid entity type, expected {}", stringify!(#variant_name))
                }
            }
        }
    };

    TokenStream::from(expanded)
}
