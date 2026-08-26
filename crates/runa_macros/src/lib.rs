use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, Data, DeriveInput, Expr, Field, Fields,
    Ident, ItemFn, ItemStruct, LitStr, Token, Type,
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

/// `#[derive(Scriptable)]` — automatically mirrors a Rust struct into Luau.
///
/// For each named field it generates:
/// - a `luau::IntoLua` / `luau::FromLua` implementation (with `glam` math
///   types and fixed-size arrays handled specially),
/// - a `export type Name = { ... }` Luau definition string,
/// - a **merge** function that writes only the scripted fields back into the
///   live component (so engine-managed fields like interpolation bookkeeping
///   or GPU handles — marked `#[script(skip)]` — are preserved),
/// - and an `inventory` registration so `runa_engine::scripting` can wire it up at
///   runtime (component globals + `GetComponent`/`HasComponent`) and emit a
///   `.d.luau` for the editor — with no manual maintenance.
///
/// Field attribute:
/// - `#[script(skip)]` — exclude the field from scripting (e.g. `OnceLock`
///   handles, interpolation state). It is left untouched on apply-back.
#[proc_macro_derive(Scriptable, attributes(script))]
pub fn scriptable_derive(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string();

    let named = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(n) => Some(&n.named),
            Fields::Unit => None,
            _ => {
                panic!("#[derive(Scriptable)] requires a struct with named fields or a unit struct")
            }
        },
        _ => panic!("#[derive(Scriptable)] requires a struct"),
    };
    let is_unit = named.is_none();
    let fields_vec: Vec<&Field> = named.map(|n| n.iter().collect()).unwrap_or_default();
    let fields = &fields_vec;

    let mut set_stmts = Vec::new();
    let mut get_stmts = Vec::new();
    let mut merge_stmts = Vec::new();
    let mut skip_stmts = Vec::new();
    let mut def_body = String::new();

    for f in fields {
        let fname = f.ident.as_ref().unwrap();
        let fstr = fname.to_string();
        let fty = &f.ty;

        let mut skipped = false;
        for attr in &f.attrs {
            if attr.path().is_ident("script") {
                let _ = attr.parse_nested_meta(|m| {
                    if m.path.is_ident("skip") {
                        skipped = true;
                    }
                    Ok(())
                });
            }
        }
        if skipped {
            skip_stmts.push(quote! { #fname: ::std::default::Default::default(), });
            continue;
        }

        def_body.push_str(&format!("    {}: {},\n", fstr, luau_ty(&f.ty)));

        let (set_expr, get_expr, merge_expr) = match &f.ty {
            Type::Array(arr) => {
                let n = array_len(&arr.len);
                let elem = &arr.elem;
                let set = quote! {
                    {
                        let __arr = self.#fname;
                        let __at = lua.create_table()?;
                        for (__i, __val) in __arr.iter().enumerate() {
                            __at.set(__i + 1, ::runa_script_api::luau::IntoLua::into_lua(__val.clone(), lua)?)?;
                        }
                        ::runa_script_api::luau::Value::Table(__at)
                    }
                };
                let get = quote! {
                    {
                        let __av: ::runa_script_api::luau::Value = table.get(#fstr)?;
                        match __av {
                            ::runa_script_api::luau::Value::Table(__at) => {
                                let mut __v = ::std::vec::Vec::new();
                                for __i in 0..#n {
                                    __v.push(::runa_script_api::luau::FromLua::from_lua(__at.get(__i + 1)?, lua)?);
                                }
                                match <_ as ::std::convert::TryInto<[_; #n]>>::try_into(__v) {
                                    Ok(__a) => __a,
                                    Err(_) => return Err(::runa_script_api::luau::Error::runtime(concat!("scriptable: bad array ", #fstr))),
                                }
                            }
                            _ => return Err(::runa_script_api::luau::Error::runtime(concat!("scriptable: expected table for ", #fstr))),
                        }
                    }
                };
                let merge = quote! {
                    {
                        let __av: ::runa_script_api::luau::Value = table.get(#fstr)?;
                        match __av {
                            ::runa_script_api::luau::Value::Table(__at) => {
                                let mut __v = ::std::vec::Vec::new();
                                for __i in 0..#n {
                                    __v.push(lua.unpack::<#elem>(__at.get(__i + 1)?)?);
                                }
                                match <_ as ::std::convert::TryInto<[_; #n]>>::try_into(__v) {
                                    Ok(__a) => __a,
                                    Err(_) => return Err(::runa_script_api::luau::Error::runtime(concat!("scriptable: bad array ", #fstr))),
                                }
                            }
                            _ => return Err(::runa_script_api::luau::Error::runtime(concat!("scriptable: expected table for ", #fstr))),
                        }
                    }
                };
                (set, get, merge)
            }
            _ => match math_kind(&f.ty) {
                Some(kind) => {
                    let to = math_to_ident(kind);
                    let from = math_from_ident(kind);
                    let set = quote! {
                        ::runa_script_api::luau::Value::Table(::runa_script_api::math::#to(lua, self.#fname)?)
                    };
                    let get = quote! {
                        {
                            let __v: ::runa_script_api::luau::Value = table.get(#fstr)?;
                            match __v {
                                ::runa_script_api::luau::Value::Table(__t) => ::runa_script_api::math::#from(&__t),
                                _ => ::std::default::Default::default(),
                            }
                        }
                    };
                    (set, get.clone(), get)
                }
                None => {
                    let set = quote! {
                        ::runa_script_api::luau::IntoLua::into_lua(self.#fname, lua)?
                    };
                    let get = quote! {
                        ::runa_script_api::luau::FromLua::from_lua({ let __v: ::runa_script_api::luau::Value = table.get(#fstr)?; __v }, lua)?
                    };
                    let merge = quote! {
                        lua.unpack::<#fty>(table.get(#fstr)?)?
                    };
                    (set, get, merge)
                }
            },
        };

        set_stmts.push(quote! { table.set(#fstr, #set_expr)?; });
        get_stmts.push(quote! { #fname: #get_expr, });
        merge_stmts.push(quote! { c.#fname = #merge_expr; });
    }

    let scriptable_from_body = if is_unit {
        quote! { Ok(Self) }
    } else {
        quote! {
            let table = match value {
                ::runa_script_api::luau::Value::Table(t) => t,
                _ => return Err(::runa_script_api::luau::Error::runtime(concat!("scriptable: expected table for ", #name_str))),
            };
            Ok(Self {
                #(#get_stmts)*
                #(#skip_stmts)*
            })
        }
    };

    let type_def = format!("export type {} = {{\n{}}}", name_str, def_body);
    let type_def_lit = proc_macro2::Literal::string(&type_def);

    let out = quote! {
        const _: () = {
            impl<'lua> ::runa_script_api::luau::IntoLua<'lua> for #ident {
                fn into_lua(self, lua: ::runa_script_api::luau::LuaRef<'lua>) -> ::runa_script_api::luau::Result<::runa_script_api::luau::Value<'lua>> {
                    let table = lua.create_table()?;
                    #(#set_stmts)*
                    Ok(::runa_script_api::luau::Value::Table(table))
                }
            }

            impl<'lua> ::runa_script_api::luau::FromLua<'lua> for #ident {
                fn from_lua(value: ::runa_script_api::luau::Value<'lua>, lua: ::runa_script_api::luau::LuaRef<'lua>) -> ::runa_script_api::luau::Result<Self> {
                    let _ = value;
                    let _ = lua;
                    #scriptable_from_body
                }
            }

            fn __scriptable_merge_luau<'lua>(c: &mut #ident, lua: &'lua ::runa_script_api::luau::Lua, table: &'lua ::runa_script_api::luau::Table<'lua>) -> ::runa_script_api::luau::Result<()> {
                #(#merge_stmts)*
                Ok(())
            }

            fn __scriptable_to_luau<'lua>(lua: &'lua ::runa_script_api::luau::Lua, world: &::runa_ecs::World, e: ::runa_ecs::Entity) -> Option<::runa_script_api::luau::Table<'lua>> {
                let __c = world.get::<#ident>(e)?;
                let __v = lua.pack(::std::clone::Clone::clone(__c)).ok()?;
                match __v {
                    ::runa_script_api::luau::Value::Table(__t) => Some(__t),
                    _ => None,
                }
            }

            fn __scriptable_from_luau<'lua>(lua: &'lua ::runa_script_api::luau::Lua, v: ::runa_script_api::luau::Value<'lua>, world: &mut ::runa_ecs::World, e: ::runa_ecs::Entity) {
                if let Some(__c) = world.get_mut::<#ident>(e) {
                    if let ::runa_script_api::luau::Value::Table(__t) = v {
                        let _ = __scriptable_merge_luau(__c, lua, &__t);
                    }
                }
            }

            ::runa_script_api::submit! {
                ::runa_script_api::ScriptType {
                    name: #name_str,
                    type_def: #type_def_lit,
                    to_luau: __scriptable_to_luau,
                    from_luau: __scriptable_from_luau,
                }
            }
        };
    };

    out.into()
}

fn math_kind(ty: &Type) -> Option<&'static str> {
    if let Type::Path(tp) = ty {
        let id = tp.path.segments.last().unwrap().ident.to_string();
        match id.as_str() {
            "Vec2" => Some("vec2"),
            "Vec3" => Some("vec3"),
            "Vec4" => Some("vec4"),
            "Quat" => Some("quat"),
            _ => None,
        }
    } else {
        None
    }
}

fn math_to_ident(kind: &str) -> proc_macro2::Ident {
    proc_macro2::Ident::new(
        match kind {
            "vec2" => "vec2_to_luau",
            "vec3" => "vec3_to_luau",
            "vec4" => "vec4_to_luau",
            "quat" => "quat_to_luau",
            _ => unreachable!(),
        },
        proc_macro2::Span::call_site(),
    )
}

fn math_from_ident(kind: &str) -> proc_macro2::Ident {
    proc_macro2::Ident::new(
        match kind {
            "vec2" => "vec2_from_luau",
            "vec3" => "vec3_from_luau",
            "vec4" => "vec4_from_luau",
            "quat" => "quat_from_luau",
            _ => unreachable!(),
        },
        proc_macro2::Span::call_site(),
    )
}

fn array_len(len: &Expr) -> usize {
    if let Expr::Lit(lit) = len {
        if let syn::Lit::Int(li) = &lit.lit {
            if let Ok(n) = li.base10_parse::<usize>() {
                return n;
            }
        }
    }
    panic!(
        "#[derive(Scriptable)] requires a fixed-size array with a literal length, e.g. [f32; 4]"
    );
}

/// Map a Rust field type to its Luau type-name string (for the generated
/// `export type`).
fn luau_ty(ty: &Type) -> String {
    match ty {
        Type::Path(tp) => {
            let seg = tp.path.segments.last().unwrap();
            let id = seg.ident.to_string();
            match id.as_str() {
                "f32" | "f64" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16"
                | "u32" | "u64" | "u128" | "usize" => "number".into(),
                "bool" => "boolean".into(),
                "String" | "str" | "Cow" => "string".into(),
                "Vec3" => "Vec3".into(),
                "Vec4" => "Vec4".into(),
                "Vec2" => "Vec2".into(),
                "Quat" => "Quat".into(),
                "Option" => match first_generic(&seg.arguments) {
                    Some(inner) => luau_ty(inner),
                    None => "any".into(),
                },
                "Vec" | "HashSet" | "BTreeSet" => match first_generic(&seg.arguments) {
                    Some(inner) => format!("{{ [number]: {} }}", luau_ty(inner)),
                    None => "{ [number]: any }".into(),
                },
                "HashMap" | "BTreeMap" => match first_two_generics(&seg.arguments) {
                    Some((k, v)) => format!("{{ [{}]: {} }}", luau_ty(k), luau_ty(v)),
                    None => "{ [any]: any }".into(),
                },
                _ => id,
            }
        }
        Type::Array(arr) => format!("{{ [number]: {} }}", luau_ty(&arr.elem)),
        Type::Reference(r) => luau_ty(&r.elem),
        _ => "any".into(),
    }
}

fn first_generic(args: &syn::PathArguments) -> Option<&Type> {
    if let syn::PathArguments::AngleBracketed(ab) = args {
        for a in &ab.args {
            if let syn::GenericArgument::Type(t) = a {
                return Some(t);
            }
        }
    }
    None
}

fn first_two_generics(args: &syn::PathArguments) -> Option<(&Type, &Type)> {
    if let syn::PathArguments::AngleBracketed(ab) = args {
        let mut types = ab.args.iter().filter_map(|a| {
            if let syn::GenericArgument::Type(t) = a {
                Some(t)
            } else {
                None
            }
        });
        if let (Some(k), Some(v)) = (types.next(), types.next()) {
            return Some((k, v));
        }
    }
    None
}
