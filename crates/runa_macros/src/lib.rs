use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, Ident, ItemFn, ItemStruct, LitStr, Token,
};

/// `#[system(Stage)]` / `#[system(Stage, "crate")]` argument.
struct SysArg {
    stage: Ident,
    crate_path: Option<LitStr>,
}

impl Parse for SysArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let stage = input.parse::<Ident>()?;
        let crate_path = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse::<LitStr>()?)
        } else {
            None
        };
        Ok(SysArg { stage, crate_path })
    }
}

#[proc_macro_attribute]
pub fn system(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let sig = &input.sig;
    let block = &input.block;
    let name = &sig.ident;

    let (stage_ident, crate_path) = parse_sys_attr(proc_macro2::TokenStream::from(attr));
    let crate_path_ts: proc_macro2::TokenStream = crate_path
        .parse()
        .unwrap_or_else(|_| "::runa_engine".parse().unwrap());

    TokenStream::from(quote! {
        #sig #block

        #crate_path_ts::ecs::inventory::submit! {
            #crate_path_ts::ecs::SystemDescriptor {
                name: stringify!(#name),
                func: #name,
                stage: #crate_path_ts::ecs::Stage::#stage_ident,
            }
        }
    })
}

/// Parse the `#[system(...)]` argument into `(stage_ident, crate_path)`.
///
/// Accepted forms:
/// - `#[system]`                          -> Stage::Update, crate `::runa_engine`
/// - `#[system(Update)]` / `(Start)`      -> that stage, crate `::runa_engine`
/// - `#[system("::my_crate")]`            -> Stage::Update, custom crate (legacy)
/// - `#[system(Update, "::my_crate")]`    -> stage + custom crate
///
/// Default crate is `::runa_engine`, which is reachable from any crate that
/// depends on `runa_engine` (the public entry point). Crates inside the runa
/// workspace that cannot see `runa_engine` (e.g. `runa_core`) must pass their
/// own path, e.g. `#[system(Update, "crate")]`.
fn parse_sys_attr(attr: proc_macro2::TokenStream) -> (proc_macro2::TokenStream, String) {
    if attr.is_empty() {
        return (quote::quote! { Update }, "::runa_engine".to_string());
    }

    // New form: `Stage` or `Stage, "crate"`.
    if let Ok(sys_arg) = syn::parse2::<SysArg>(attr.clone()) {
        let stage_ident = sys_arg.stage;
        let crate_path = sys_arg
            .crate_path
            .map(|s| s.value())
            .unwrap_or_else(|| "::runa_engine".to_string());
        return (quote::quote! { #stage_ident }, crate_path);
    }

    // Legacy form: a single string literal = crate path, stage Update.
    if let Ok(s) = syn::parse2::<LitStr>(attr) {
        return (quote::quote! { Update }, s.value());
    }

    panic!(
        "invalid #[system] argument. Use #[system], #[system(Update)], #[system(Start)], \
         #[system(\"::crate\")], or #[system(Update, \"::crate\")]"
    );
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
