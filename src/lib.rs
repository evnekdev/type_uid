//! This crate defines a `TypUid` macro which can be applied to any Rust structure to generate a unique identifier which depends only on the struct contents and its type. It is useful when creating a serialization/deserialization pipeline, especially using binary formats. During project development, when a type can be modified, it is useful to stamp the serialized data to avoid deserialization errors.
//! 
//! Disclaimer - the current module was created with the help of an LLM.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, GenericParam, Ident,
};

/// The main macro definition
#[proc_macro_derive(TypeUid)]
pub fn derive_type_uid(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();

    // Collect repr(...) attributes to fold into the structure hash.
    let reprs = collect_repr_attrs(&input.attrs);

    // Only structs for this version.
    let ds = match &input.data {
        Data::Struct(s) => s.clone(),
        _ => panic!("#[derive(TypeUid)] supports structs only"),
    };

    // Build structure signature including generics (type/const/lifetime) + fields in order.
    let structure_sig =
        build_structure_signature_with_generics(&name, &input.generics, &reprs, &ds);

    // Macro-time hash (blake3), keep first 128 bits as u128 (LE).
    let structure_hash_u128 = {
        let hash = blake3::hash(structure_sig.as_bytes());
        let bytes = hash.as_bytes();
        let mut acc: u128 = 0;
        for (i, b) in bytes.iter().take(16).enumerate() {
            acc |= (*b as u128) << (8 * i);
        }
        acc
    };

    // Tokens for per-field offset_of!(Self, field) and position tags.
    let (offset_tokens, field_tags): (Vec<_>, Vec<_>) =
        fields_offset_tokens(&ds).into_iter().unzip();

    // Mix const generics values at compile time so each instantiation differs.
    let const_params_mix: Vec<_> = input
        .generics
        .params
        .iter()
        .filter_map(|gp| match gp {
            GenericParam::Const(c) => {
                let ident = &c.ident;
                Some(quote! {
                    h = mix(h, #ident as u64);
                })
            }
            _ => None,
        })
        .collect();

    // Split generics for impl.
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let repr_debug = reprs.join(",");

    // Generate: inherent consts on the type
    let expanded = quote! {
        // We implement inherent consts, parameterized over the same generics.
        impl #impl_generics #name #ty_generics #where_clause {
            /// Macro-time hash of the structure *shape* (names, types, order, generics signature, repr).
            pub const __TYPE_UID_STRUCTURE_HASH: u128 = #structure_hash_u128;

            /// Final type UID mixing in:
            /// - const generic values (if any)
            /// - actual layout: size, align, and per-field offsets
            pub const TYPE_UID: u128 = {
                const fn mix(mut h: u128, x: u64) -> u128 {
                    // Simple, const-friendly 128-bit mixer (good avalanche).
                    let k = (x as u128).wrapping_mul(0x9E3779B97F4A7C15_94D049BB133111EBu128);
                    h ^= k;
                    h = h.rotate_left(31).wrapping_mul(0xBF58476D1CE4E5B9_B5297A4D9AEDE1EFu128);
                    h
                }

                let mut h = Self::__TYPE_UID_STRUCTURE_HASH;

                // Mix const generic values so each instantiation differs.
                #(#const_params_mix)*

                // Mix actual target layout:
                h = mix(h, core::mem::size_of::<Self>() as u64);
                h = mix(h, core::mem::align_of::<Self>() as u64);

                // Mix each field's position tag and its byte offset.
                #(
                    {
                        let _tag: u64 = (#field_tags as u64);
                        let off = std::mem::offset_of!(Self, #offset_tokens) as u64;
                        let mut hh = mix(h, _tag);
                        h = mix(hh, off);
                    }
                )*

                h
            };

            /// Optional: expose repr info for debugging
            pub const __TYPE_UID_REPR: &'static str = #repr_debug;

            /// Optional: expose the exact structure signature that was hashed (for debugging)
            pub const __TYPE_UID_STRUCT_SIG: &'static str = {
                // Storing the whole string increases binary size; comment out if undesired.
                #structure_sig
            };
        }
    };

    TokenStream::from(expanded)
}

fn collect_repr_attrs(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("repr"))
        .map(|a| a.to_token_stream().to_string())
        .collect()
}

fn build_structure_signature_with_generics(
    _name: &Ident,
    generics: &syn::Generics,
    reprs: &[String],
    ds: &DataStruct,
) -> String {
    let mut s = String::new();
    //s.push_str("struct ");
    //s.push_str(&name.to_string());

    if !generics.params.is_empty() {
        s.push('<');
        for gp in &generics.params {
            match gp {
                GenericParam::Type(t) => {
                    s.push_str(&format!("type {} ", t.ident));
                    // Bounds intentionally omitted from the signature; add if you want bounds-sensitive IDs.
                }
                GenericParam::Lifetime(lt) => {
                    s.push('\'');
                    s.push_str(&lt.lifetime.ident.to_string());
                    s.push(' ');
                }
                GenericParam::Const(c) => {
                    s.push_str("const ");
                    s.push_str(&c.ident.to_string());
                    s.push_str(": ");
                    s.push_str(&c.ty.to_token_stream().to_string());
                    s.push(' ');
                }
            }
            s.push(',');
        }
        s.push('>');
    }

    if !reprs.is_empty() {
        s.push_str(" [");
        s.push_str(&reprs.join(";"));
        s.push(']');
    }

    s.push_str(" {");

    match &ds.fields {
        Fields::Named(named) => {
            for (i, f) in named.named.iter().enumerate() {
                let fname = f.ident.as_ref().unwrap().to_string();
                let fty = f.ty.to_token_stream().to_string();
                s.push_str(&format!("#{} {}: {},", i, fname, fty));
            }
        }
        Fields::Unnamed(unnamed) => {
            for (i, f) in unnamed.unnamed.iter().enumerate() {
                let fty = f.ty.to_token_stream().to_string();
                s.push_str(&format!("#{} {}", i, fty));
                s.push(',');
            }
        }
        Fields::Unit => {
            s.push_str("unit");
        }
    }

    s.push('}');
    s
}

fn fields_offset_tokens(ds: &DataStruct) -> Vec<(proc_macro2::TokenStream, usize)> {
    match &ds.fields {
        Fields::Named(named) => named
            .named
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let ident = f.ident.as_ref().unwrap();
                (quote!( #ident ), i)
            })
            .collect(),
        Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, _f)| {
                let idx = syn::Index::from(i);
                // offset_of!(Self, 0) style for tuple fields
                (quote!( #idx ), i)
            })
            .collect(),
        Fields::Unit => vec![],
    }
}
