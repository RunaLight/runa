use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, Data, DeriveInput, Expr, Field, Fields,
    FnArg, Ident, ItemFn, ItemStruct, LitStr, Pat, ReturnType, Token, Type,
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
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let name = &sig.ident;

    let (stage_ident, crate_path) = parse_sys_attr(proc_macro2::TokenStream::from(attr));
    let crate_path_ts: proc_macro2::TokenStream = crate_path
        .parse()
        .unwrap_or_else(|_| "::runa_engine".parse().unwrap());

    TokenStream::from(quote! {
        #vis #sig #block

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
/// Field/struct attributes:
/// - `#[script(skip)]` — exclude the field from scripting (e.g. `OnceLock`
///   handles, interpolation state). It is left untouched on apply-back.
/// - `#[script(addable)]` — (default) opt into runtime `AddComponent`/`RemoveComponent`.
///   Requires the component to be `Default`. Components that are not `Default` should
///   pass `#[script(not_addable)]`.
/// - `#[script(not_addable)]` — opt out of runtime `AddComponent`/`RemoveComponent`
///   (required when the component is not `Default`).
/// - `#[script(crate = "...")]` — point generated code at the public entry-point crate
///   (e.g. `runa_engine`); external game crates can usually omit this.
#[proc_macro_derive(Scriptable, attributes(script))]
pub fn scriptable_derive(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string();

    // `#[script(addable)]` opts the type into runtime `AddComponent`/`RemoveComponent`.
    // It is now the DEFAULT (so a plain `#[derive(Scriptable)]` is already addable in
    // Lua). Components that are not `Default` cannot be insert-or-created by
    // `AddComponent`, so they should opt out with `#[script(not_addable)]`.
    //
    // `#[script(crate = "...")]` points the generated code at the public entry-point
    // crate (e.g. `runa_engine`) instead of the internal `runa_script_api`/`runa_ecs`.
    // External game crates depend only on `runa_engine` and can just write
    // `#[script(addable)]` (or nothing) — the default already resolves to
    // `runa_engine::scripting_api` / `runa_engine::ecs`.
    let mut addable = true;
    let mut crate_path: Option<String> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("script") {
            let _ = attr.parse_nested_meta(|m| {
                if m.path.is_ident("addable") {
                    addable = true;
                } else if m.path.is_ident("not_addable") {
                    addable = false;
                } else if m.path.is_ident("crate") {
                    let s: LitStr = m.value()?.parse()?;
                    crate_path = Some(s.value());
                }
                Ok(())
            });
        }
    }

    // Resolve the scripting-API / ECS crate paths used in the generated code.
    //
    // - No `#[script(crate = ...)]`: default to the public entry-point crate
    //   (`runa_engine::scripting_api` / `runa_engine::ecs`) so external game crates
    //   that depend only on `runa_engine` can write `#[script(addable)]` with no
    //   extra configuration.
    // - `crate = "runa_engine"` (or any path containing it): use the re-exported
    //   `scripting_api` / `ecs` submodules of that crate.
    // - Internal crates (e.g. `runa_core`) pass the low-level crates directly:
    //   `#[script(crate = "::runa_script_api")]` → `::runa_script_api` (with
    //   `::runa_ecs` for the ECS side).
    let (api_path, ecs_path): (proc_macro2::TokenStream, proc_macro2::TokenStream) =
        match crate_path.as_deref() {
            None => (
                "::runa_engine::scripting_api".parse().unwrap(),
                "::runa_engine::ecs".parse().unwrap(),
            ),
            Some(c) if c.contains("runa_engine") => (
                format!("{c}::scripting_api").parse().unwrap(),
                format!("{c}::ecs").parse().unwrap(),
            ),
            Some(c) => (c.parse().unwrap(), "::runa_ecs".parse().unwrap()),
        };

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
                            __at.set(__i + 1, #api_path::luau::IntoLua::into_lua(__val.clone(), lua)?)?;
                        }
                        #api_path::luau::Value::Table(__at)
                    }
                };
                let get = quote! {
                    {
                        let __av: #api_path::luau::Value = table.get(#fstr)?;
                        match __av {
                            #api_path::luau::Value::Table(__at) => {
                                let mut __v = ::std::vec::Vec::new();
                                for __i in 0..#n {
                                    __v.push(#api_path::luau::FromLua::from_lua(__at.get(__i + 1)?, lua)?);
                                }
                                match <_ as ::std::convert::TryInto<[_; #n]>>::try_into(__v) {
                                    Ok(__a) => __a,
                                    Err(_) => return Err(#api_path::luau::Error::runtime(concat!("scriptable: bad array ", #fstr))),
                                }
                            }
                            _ => return Err(#api_path::luau::Error::runtime(concat!("scriptable: expected table for ", #fstr))),
                        }
                    }
                };
                let merge = quote! {
                    {
                        let __av: #api_path::luau::Value = table.get(#fstr)?;
                        match __av {
                            #api_path::luau::Value::Table(__at) => {
                                let mut __v = ::std::vec::Vec::new();
                                for __i in 0..#n {
                                    __v.push(lua.unpack::<#elem>(__at.get(__i + 1)?)?);
                                }
                                match <_ as ::std::convert::TryInto<[_; #n]>>::try_into(__v) {
                                    Ok(__a) => __a,
                                    Err(_) => return Err(#api_path::luau::Error::runtime(concat!("scriptable: bad array ", #fstr))),
                                }
                            }
                            _ => return Err(#api_path::luau::Error::runtime(concat!("scriptable: expected table for ", #fstr))),
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
                        #api_path::luau::Value::Table(#api_path::math::#to(lua, self.#fname)?)
                    };
                    let get = quote! {
                        {
                            let __v: #api_path::luau::Value = table.get(#fstr)?;
                            match __v {
                                #api_path::luau::Value::Table(__t) => #api_path::math::#from(&__t),
                                _ => ::std::default::Default::default(),
                            }
                        }
                    };
                    (set, get.clone(), get)
                }
                None => {
                    let set = quote! {
                        #api_path::luau::IntoLua::into_lua(self.#fname, lua)?
                    };
                    let get = quote! {
                        #api_path::luau::FromLua::from_lua({ let __v: #api_path::luau::Value = table.get(#fstr)?; __v }, lua)?
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
                #api_path::luau::Value::Table(t) => t,
                _ => return Err(#api_path::luau::Error::runtime(concat!("scriptable: expected table for ", #name_str))),
            };
            Ok(Self {
                #(#get_stmts)*
                #(#skip_stmts)*
            })
        }
    };

    let type_def = format!("export type {} = {{\n{}}}", name_str, def_body);
    let type_def_lit = proc_macro2::Literal::string(&type_def);

    let add_fn: proc_macro2::TokenStream = if addable {
        quote! {
            fn __scriptable_add_luau<'lua>(lua: #api_path::luau::LuaRef<'lua>, v: #api_path::luau::Value<'lua>, world: &mut #ecs_path::World, e: #ecs_path::Entity)
            where #ident: Default {
                if world.get_mut::<#ident>(e).is_some() {
                    __scriptable_from_luau(lua, v, world, e);
                } else {
                    let mut __c = #ident::default();
                    if let #api_path::luau::Value::Table(__t) = v {
                        let _ = __scriptable_merge_luau(&mut __c, lua, &__t);
                    }
                    world.add_component(e, __c);
                }
            }
        }
    } else {
        quote! {}
    };
    let addable_arm: proc_macro2::TokenStream = if addable {
        quote! { __scriptable_add_luau }
    } else {
        quote! { __scriptable_noop_add }
    };
    let removeable_arm: proc_macro2::TokenStream = quote! { __scriptable_remove_luau };

    let out = quote! {
        const _: () = {
            impl<'lua> #api_path::luau::IntoLua<'lua> for #ident {
                fn into_lua(self, lua: #api_path::luau::LuaRef<'lua>) -> #api_path::luau::Result<#api_path::luau::Value<'lua>> {
                    let table = lua.create_table()?;
                    #(#set_stmts)*
                    Ok(#api_path::luau::Value::Table(table))
                }
            }

            impl<'lua> #api_path::luau::FromLua<'lua> for #ident {
                fn from_lua(value: #api_path::luau::Value<'lua>, lua: #api_path::luau::LuaRef<'lua>) -> #api_path::luau::Result<Self> {
                    let _ = value;
                    let _ = lua;
                    #scriptable_from_body
                }
            }

            fn __scriptable_merge_luau<'lua>(c: &mut #ident, lua: #api_path::luau::LuaRef<'lua>, table: &'lua #api_path::luau::Table<'lua>) -> #api_path::luau::Result<()> {
                #(#merge_stmts)*
                Ok(())
            }

            fn __scriptable_to_luau<'lua>(lua: #api_path::luau::LuaRef<'lua>, world: &#ecs_path::World, e: #ecs_path::Entity) -> Option<#api_path::luau::Table<'lua>> {
                let __c = world.get::<#ident>(e)?;
                let __v = lua.pack(::std::clone::Clone::clone(__c)).ok()?;
                match __v {
                    #api_path::luau::Value::Table(__t) => Some(__t),
                    _ => None,
                }
            }

            fn __scriptable_from_luau<'lua>(lua: #api_path::luau::LuaRef<'lua>, v: #api_path::luau::Value<'lua>, world: &mut #ecs_path::World, e: #ecs_path::Entity) {
                if let Some(__c) = world.get_mut::<#ident>(e) {
                    if let #api_path::luau::Value::Table(__t) = v {
                        let _ = __scriptable_merge_luau(__c, lua, &__t);
                    }
                }
            }

            fn __scriptable_remove_luau(world: &mut #ecs_path::World, e: #ecs_path::Entity) {
                world.remove_component::<#ident>(e);
            }

            #add_fn

            // No-op fallback for types that are not `#[script(addable)]`.
            fn __scriptable_noop_add<'lua>(
                _lua: #api_path::luau::LuaRef<'lua>,
                _v: #api_path::luau::Value<'lua>,
                _world: &mut #ecs_path::World,
                _e: #ecs_path::Entity,
            ) {}

            #api_path::submit! {
                #api_path::ScriptType {
                    name: #name_str,
                    type_def: #type_def_lit,
                    to_luau: __scriptable_to_luau,
                    from_luau: __scriptable_from_luau,
                    add: #addable_arm,
                    remove: #removeable_arm,
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

/// `#[script_fn]` / `#[script_fn(crate = "runa_engine")]` — expose a free Rust
/// function to Luau. After the engine builds a VM, the function is registered on the
/// `runa` module and as a bare global, so scripts call `runa.my_func(...)` or simply
/// `my_func(...)`.
///
/// Arguments and the return value are converted with `FromLua` / `IntoLua`, so use
/// conversion-friendly types (`f32`, `f64`, `i32`, `i64`, `bool`, `String`, ...).
///
/// Example:
/// ```ignore
/// #[script_fn]
/// fn add(a: f32, b: f32) -> f32 { a + b }
/// // In Luau: `local r = runa.add(2, 3)`  -> 5
/// ```
#[proc_macro_attribute]
pub fn script_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let name = &sig.ident;
    let name_str = name.to_string();

    // `#[script_fn(crate = "...")]` points the generated glue at the public
    // entry-point crate (e.g. `runa_engine`); external game crates can omit it.
    let crate_arg = parse_macro_input!(attr as FnCrateArg);
    let api_path: proc_macro2::TokenStream = match crate_arg.crate_path.map(|s| s.value()) {
        None => "::runa_engine::scripting_api".parse().unwrap(),
        Some(c) if c.contains("runa_engine") => format!("{c}::scripting_api").parse().unwrap(),
        Some(c) => c.parse().unwrap(),
    };

    // Collect parameter idents + types (skip `self`).
    let mut arg_idents: Vec<Ident> = Vec::new();
    let mut arg_types: Vec<&Type> = Vec::new();
    for arg in &sig.inputs {
        if let FnArg::Typed(pt) = arg {
            let ident = if let Pat::Ident(pi) = &*pt.pat {
                pi.ident.clone()
            } else {
                Ident::new(
                    &format!("__arg_{}", arg_types.len()),
                    proc_macro2::Span::call_site(),
                )
            };
            arg_idents.push(ident);
            arg_types.push(&pt.ty);
        }
    }

    let extracts = arg_idents
        .iter()
        .zip(arg_types.iter())
        .enumerate()
        .map(|(i, (ident, ty))| {
            quote! {
                let #ident: #ty = {
                    let __v = std::mem::replace(
                        &mut __args[#i],
                        #api_path::luau::Value::Nil,
                    );
                    #api_path::luau::FromLua::from_lua(__v, lua)?
                };
            }
        });

    let ret_expr = match &sig.output {
        ReturnType::Default => quote! { Ok(#api_path::luau::Value::Nil) },
        ReturnType::Type(_, ty) if is_unit(ty) => {
            quote! { let _ = __r; Ok(#api_path::luau::Value::Nil) }
        }
        _ => quote! { #api_path::luau::IntoLua::into_lua(__r, lua) },
    };

    let out = quote! {
        #vis #sig #block

        #api_path::submit!(#api_path::ScriptFunction {
            name: #name_str,
            func: |lua: #api_path::luau::LuaRef<'_>,
                   __args: #api_path::luau::Variadic<#api_path::luau::Value<'_>>|
                   -> #api_path::luau::Result<#api_path::luau::Value<'_>> {
                let mut __args: Vec<#api_path::luau::Value<'_>> =
                    __args.into_iter().collect();
                #(#extracts)*
                let __r = #name(#(#arg_idents),*);
                #ret_expr
            }
        });
    };

    out.into()
}

/// `#[script_fn(crate = "...")]` argument.
struct FnCrateArg {
    crate_path: Option<LitStr>,
}

impl Parse for FnCrateArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut crate_path = None;
        if input.peek(Token![crate]) {
            input.parse::<Token![crate]>()?;
            input.parse::<Token![=]>()?;
            crate_path = Some(input.parse::<LitStr>()?);
        }
        Ok(FnCrateArg { crate_path })
    }
}

/// True for the unit type `()`.
fn is_unit(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(t) if t.elems.is_empty())
}
