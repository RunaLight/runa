use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct, LitStr};

#[proc_macro_attribute]
pub fn system(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let sig = &input.sig;
    let block = &input.block;
    let name = &sig.ident;

    let crate_path: proc_macro2::TokenStream = if attr.is_empty() {
        "::runa_engine".parse().unwrap()
    } else {
        let lit: LitStr = syn::parse(attr)
            .expect("expected optional crate path argument, e.g. #[system(\"::my_crate\")]");
        lit.value().parse().expect("invalid crate path")
    };

    TokenStream::from(quote! {
        #sig #block

        #crate_path::ecs::inventory::submit! {
            #crate_path::ecs::SystemDescriptor {
                name: stringify!(#name),
                func: #name,
            }
        }
    })
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
