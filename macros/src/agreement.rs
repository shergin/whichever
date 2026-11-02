//! The checks that every variant agrees with the first on the associated
//! types and consts the impl takes from it.
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{GenericArgument, GenericParam, Ident, Path, PathArguments, Type};

use crate::shape::bare_ident;
use crate::subject::Target;

/// Returns every variant but `first`, the one the items were taken from.
fn others<'a>(
    variants: &'a [&'a Target],
    first: &'a Target,
) -> impl Iterator<Item = &'a &'a Target> {
    variants
        .iter()
        .filter(move |variant| !std::ptr::eq(**variant, first))
}

/// How two values of an associated const's type are compared at compile
/// time: with `==` for the primitives that allow it in a `const`, or byte
/// by byte for `&str`.
pub(crate) enum ConstComparison {
    Primitive,
    Str,
}

pub(crate) fn const_comparison(ty: &Type) -> Option<ConstComparison> {
    const PRIMITIVES: [&str; 15] = [
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
        "bool", "char", "()",
    ];
    match ty {
        Type::Tuple(tuple) if tuple.elems.is_empty() => Some(ConstComparison::Primitive),
        Type::Reference(reference) if reference.mutability.is_none() => {
            match bare_ident(&reference.elem) {
                Some(ident) if ident == "str" => Some(ConstComparison::Str),
                _ => None,
            }
        }
        _ => bare_ident(ty)
            .filter(|ident| PRIMITIVES.contains(&ident.to_string().as_str()))
            .map(|_| ConstComparison::Primitive),
    }
}

/// Checks at compile time that every variant's associated consts of a
/// comparable type equal the first variant's, with the failure reported on
/// the variant. Const evaluation happens after monomorphisation, so a
/// generic enum gets no check.
pub(crate) fn associated_const_check(
    consts: &[(&Ident, Type)],
    variants: &[&Target],
    first: &Target,
    trait_path: &Path,
    first_payload: &TokenStream2,
    generics: &syn::Generics,
    self_type: &Type,
) -> TokenStream2 {
    if variants.len() < 2 || !generics.params.is_empty() {
        return TokenStream2::new();
    }
    let mut checks = TokenStream2::new();
    let mut needs_str_eq = false;
    for (name, ty) in consts {
        let Some(comparison) = const_comparison(ty) else {
            continue;
        };
        for variant in others(variants, first) {
            let payload = variant.target_type_outside(self_type);
            let cfgs = &variant.cfgs;
            let message = format!(
                "`{name}` of variant `{}` differs from the first variant's; set it on the enum with `{name} = value`",
                variant.ident
            );
            let equal = match comparison {
                ConstComparison::Primitive => quote! {
                    <#first_payload as #trait_path>::#name == <#payload as #trait_path>::#name
                },
                ConstComparison::Str => {
                    needs_str_eq = true;
                    quote! {
                        __whichever_str_eq(<#first_payload as #trait_path>::#name, <#payload as #trait_path>::#name)
                    }
                }
            };
            let span = variant.payload.span();
            checks.extend(quote_spanned! { span =>
                #(#cfgs)* assert!(#equal, #message);
            });
        }
    }
    if checks.is_empty() {
        return TokenStream2::new();
    }
    let str_eq = needs_str_eq.then(|| {
        quote! {
            const fn __whichever_str_eq(a: &str, b: &str) -> bool {
                let (a, b) = (a.as_bytes(), b.as_bytes());
                if a.len() != b.len() {
                    return false;
                }
                let mut i = 0;
                while i < a.len() {
                    if a[i] != b[i] {
                        return false;
                    }
                    i += 1;
                }
                true
            }
        }
    });
    quote! {
        const _: () = {
            #str_eq
            #checks
        };
    }
}

/// Checks at compile time that every variant's associated types equal the
/// first variant's, so a disagreement is reported on the variant instead of
/// inside a generated method body.
///
/// The helper carries the enum's own generics, since a nested function
/// could not name them; the outer function passes them on to each call.
pub(crate) fn associated_type_check(
    names: &[&Ident],
    variants: &[&Target],
    first: &Target,
    trait_path: &Path,
    first_payload: &TokenStream2,
    generics: &syn::Generics,
    self_type: &Type,
) -> TokenStream2 {
    if names.is_empty() || variants.len() < 2 {
        return TokenStream2::new();
    }
    let bindings: Vec<GenericArgument> = names
        .iter()
        .map(|name| syn::parse_quote!(#name = <#first_payload as #trait_path>::#name))
        .collect();
    let mut bound = trait_path.clone();
    let last = bound
        .segments
        .last_mut()
        .expect("a path has at least one segment");
    match &mut last.arguments {
        PathArguments::AngleBracketed(arguments) => arguments.args.extend(bindings),
        arguments => {
            *arguments = PathArguments::AngleBracketed(syn::parse_quote!(<#(#bindings),*>))
        }
    }
    let mut helper_generics = generics.clone();
    helper_generics
        .params
        .push(syn::parse_quote!(__Variant: ?Sized + #bound));
    let (helper_impl_generics, _, helper_where_clause) = helper_generics.split_for_impl();
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let enum_arguments: Vec<TokenStream2> = generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Lifetime(param) => param.lifetime.to_token_stream(),
            GenericParam::Type(param) => param.ident.to_token_stream(),
            GenericParam::Const(param) => param.ident.to_token_stream(),
        })
        .collect();
    let checks = others(variants, first).map(|variant| {
        let payload = variant.target_type_outside(self_type);
        let cfgs = &variant.cfgs;
        quote! { #(#cfgs)* __same_as_first_variant::<#(#enum_arguments,)* #payload>(); }
    });
    quote! {
        const _: () = {
            #[allow(dead_code, non_snake_case)]
            fn __same_as_first_variant #helper_impl_generics () #helper_where_clause {}
            #[allow(dead_code, non_snake_case)]
            fn __whichever_associated_types_agree #impl_generics () #where_clause {
                #(#checks)*
            }
        };
    }
}
