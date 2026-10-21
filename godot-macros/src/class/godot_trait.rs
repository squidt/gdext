use proc_macro2::TokenStream;
use quote::quote;
use venial::FnParam;

use crate::util::{self, bail};
use crate::ParseResult;

/// This macro takes a trait implementation and duplicates it into a Godot API block.
///
/// Example:
/// ```
/// #[godot_trait_bridge]
/// impl MyTrait for MyClass { ... }
/// ```
/// Expands to:
/// ```
/// impl MyTrait for MyClass { ... }
///
/// #[godot_api]
/// impl MyClass { ... } // Wrapper functions with #[func]
/// ```
pub fn attribute_godot_trait(input_decl: venial::Item) -> ParseResult<TokenStream> {
    let venial::Item::Impl(decl) = input_decl else {
        return bail!(
            input_decl,
            "#[godot_trait] can only be applied on impl blocks"
        );
    };

    if decl.impl_generic_params.is_some() {
        bail!(
            &decl,
            "This macro does not support lifetimes or generic parameters."
        )?;
    }

    let Some(trait_path) = decl.trait_ty.as_ref() else {
        return bail!(
            &decl,
            "#[godot_trait] requires a trait; it cannot be applied to inherent impl blocks",
        );
    };

    let mut godot_methods = Vec::new();

    // collect functions calls for the godot-side
    for decl_item in &decl.body_items {
        if let venial::ImplMember::AssocFunction(function) = decl_item {
            let fn_sig = util::reduce_to_signature(function);
            let fn_name = &fn_sig.name;
            let fn_params = &fn_sig
                .params
                .items()
                .map(|param| match param {
                    FnParam::Receiver(param) => {
                        let tk_self = &param.tk_self;
                        quote! { #tk_self }
                    }
                    FnParam::Typed(param) => {
                        let name = &param.name;
                        quote! { #name }
                    }
                })
                .collect::<Vec<_>>();
            godot_methods.push(quote! {
                #[func]
                #fn_sig {
                    <Self as #trait_path>::#fn_name(#(#fn_params),*)
                }
            });
        }
    }

    let self_ty = &decl.self_ty;
    let where_clause = &decl.where_clause;
    // combine rust trait and wrapper calls
    let expanded = quote! {
        // original
        #[godot_dyn]
        #decl

        // godot_api wrapper
        // #[godot_api]
        // impl Self where {
        //      godot_methods(...), ...
        // }
        #[godot_api]
        impl #self_ty #where_clause {
            #(#godot_methods)*
        }
    };

    Ok(expanded)
}
