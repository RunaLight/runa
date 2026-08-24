use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemFn, ItemStruct};

#[proc_macro_attribute]
pub fn system(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let sig = &input.sig;
    let block = &input.block;
    let name = &sig.ident;

    let stage_ident = parse_stage(proc_macro2::TokenStream::from(attr));

    TokenStream::from(quote! {
        #sig #block

        ::runa_ecs::inventory::submit! {
            ::runa_ecs::SystemDescriptor {
                name: stringify!(#name),
                func: #name,
                stage: ::runa_ecs::Stage::#stage_ident,
            }
        }
    })
}

/// Parse the `#[system(...)]` argument into a `Stage` variant identifier.
///
/// Accepted forms:
/// - `#[system]`            -> `Stage::Update`
/// - `#[system(Update)]`    -> `Stage::Update`
/// - `#[system(Start)]`     -> `Stage::Start`
fn parse_stage(attr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    if attr.is_empty() {
        return quote::quote! { Update };
    }
    match syn::parse2::<Ident>(attr) {
        Ok(ident) => quote::quote! { #ident },
        Err(_) => panic!(
            "invalid #[system] argument. Use #[system], #[system(Update)], or #[system(Start)]"
        ),
    }
}

#[proc_macro_attribute]
pub fn scene(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let factory_name = quote::format_ident!("__scene_factory_{}", name);

    TokenStream::from(quote! {
        #input

        #[doc(hidden)]
        fn #factory_name() -> ::std::boxed::Box<dyn ::runa_engine::Scene> {
            ::std::boxed::Box::new(#name::default())
        }

        ::runa_engine::ecs::inventory::submit! {
            ::runa_engine::SceneDescriptor {
                name: stringify!(#name),
                factory: #factory_name,
            }
        }
    })
}
