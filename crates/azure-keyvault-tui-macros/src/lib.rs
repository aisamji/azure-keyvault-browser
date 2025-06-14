use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{FnArg, ItemFn, parse_macro_input, parse_quote, parse2, DeriveInput, Data, Fields, Variant, Attribute, Meta, spanned::Spanned};
use darling::{FromMeta, ast::NestedMeta};
use proc_macro_error::{proc_macro_error, abort};

#[derive(FromMeta)]
struct CallbackArgs {
    event_enum: syn::Path,
    error_variant: syn::Expr,
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn background_task(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(args) => args,
        Err(e) => abort!(e.span(), "Invalid attribute arguments: {}", e),
    };
    let args = match CallbackArgs::from_list(&attr_args) {
        Ok(args) => args,
        Err(e) => return TokenStream::from(e.write_errors()),
    };
    background_task_impl(input, args).into()
}

fn background_task_impl(mut function: ItemFn, args: CallbackArgs) -> TokenStream2 {
    function.sig.inputs.insert(0, event_sender_arg(&args.event_enum));
    let update_progress_macro = update_progress_macro();
    let abort_macro = abort_macro(&args.error_variant);
    let block = function.block.clone();
    function.block = parse2(quote! {
        {
            #update_progress_macro
            #abort_macro
            #block
        }
    })
    .unwrap_or_else(|e| {
        abort!(proc_macro2::Span::mixed_site(), "Failed to parse function block: {}", e);
    });
    quote! { #function }
}


fn event_sender_arg(event_type: &syn::Path) -> FnArg {
    let event_type_ident: syn::Type = syn::parse_quote!(#event_type);
    parse_quote!(tx: tokio::sync::mpsc::Sender<#event_type_ident>)
}

fn update_progress_macro() -> TokenStream2 {
    quote! {
        macro_rules! update_progress {
            ($variant: expr) => {
                if let Err(_) = tx.send($variant).await {
                    return;
                }
            }
        }
    }
}

fn abort_macro(error_variant: &syn::Expr) -> TokenStream2 {
    quote! {
        macro_rules! abort {
            ($($arg:tt)*) => {
                let message = format!($($arg)*);
                let _ = tx.send(#error_variant(message.clone())).await.inspect_err(|_| eprintln!("{}", message));
                return;
            }
        }
    }
}

#[derive(FromMeta)]
struct EnumArgs {
    event_enum: syn::Path,
}

#[derive(FromMeta)]
struct VariantArgs {
    callback: syn::Ident,
}

#[proc_macro_error]
#[proc_macro_derive(BackgroundTaskSpec, attributes(taskspec))]
pub fn background_task_spec_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    background_task_spec_impl(input).into()
}

fn background_task_spec_impl(input: DeriveInput) -> TokenStream2 {
    let enum_name = &input.ident;
    
    // Parse the event_type from the derive macro arguments
    let event_type = extract_event_enum(&input.attrs);
    let event_type_ident: syn::Type = syn::parse_quote!(#event_type);
    
    let variants = match input.data {
        Data::Enum(data_enum) => data_enum.variants,
        _ => abort!(input.ident.span(), "BackgroundTaskSpec can only be derived for enums, found {}", 
                   match input.data {
                       Data::Struct(_) => "struct",
                       Data::Union(_) => "union",
                       _ => "unknown type",
                   }),
    };
    
    let spawn_arms = variants.iter().map(|variant| {
        generate_spawn_arm(variant)
    }).collect::<Vec<_>>();
    
    quote! {
        impl #enum_name {
            /// Spawns a background task for this task specification.
            /// 
            /// This method consumes `self` to move the contained data into the spawned task.
            /// Returns a `JoinHandle` that can be used to await completion of the background task.
            pub fn spawn_task(self, tx: &tokio::sync::mpsc::Sender<#event_type_ident>) -> tokio::task::JoinHandle<()> {
                match self {
                    #(#spawn_arms)*
                }
            }
        }
    }
}

fn generate_spawn_arm(variant: &Variant) -> TokenStream2 {
    let variant_name = &variant.ident;
    let callback_name = extract_callback_name(&variant.attrs);
    
    match &variant.fields {
        Fields::Unit => {
            if let Some(callback) = callback_name {
                quote! {
                    Self::#variant_name => {
                        let tx_clone = tx.clone();
                        tokio::task::spawn(async move {
                            #callback(tx_clone).await;
                        })
                    }
                }
            } else {
                quote! {
                    Self::#variant_name => {}
                }
            }
        }
        Fields::Unnamed(fields) => {
            let field_names: Vec<syn::Ident> = (0..fields.unnamed.len())
                .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::mixed_site()))
                .collect();
            
            let pattern = quote! { Self::#variant_name(#(#field_names),*) };
            
            if let Some(callback) = callback_name {
                let args = quote! { tx_clone, #(#field_names),* };
                quote! {
                    #pattern => {
                        let tx_clone = tx.clone();
                        tokio::task::spawn(async move {
                            #callback(#args).await;
                        })
                    }
                }
            } else {
                quote! {
                    #pattern => {}
                }
            }
        }
        Fields::Named(fields) => {
            let field_names: Vec<&syn::Ident> = fields.named.iter()
                .filter_map(|f| f.ident.as_ref())
                .collect();
            
            if field_names.len() != fields.named.len() {
                abort!(variant.ident.span(), "All named fields must have identifiers");
            }
            
            let pattern = quote! { Self::#variant_name { #(#field_names),* } };
            
            if let Some(callback) = callback_name {
                let args = quote! { tx_clone, #(#field_names),* };
                quote! {
                    #pattern => {
                        let tx_clone = tx.clone();
                        tokio::task::spawn(async move {
                            #callback(#args).await;
                        })
                    }
                }
            } else {
                quote! {
                    #pattern => {}
                }
            }
        }
    }
}

fn extract_event_enum(attrs: &[Attribute]) -> syn::Path {
    for attr in attrs {
        if attr.path().is_ident("taskspec") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested = match NestedMeta::parse_meta_list(meta_list.tokens.clone()) {
                    Ok(nested) => nested,
                    Err(e) => abort!(attr.span(), "Invalid taskspec attribute syntax: {}", e),
                };
                let args = match EnumArgs::from_list(&nested) {
                    Ok(args) => args,
                    Err(e) => abort!(attr.span(), "Invalid taskspec attribute: {}", e),
                };
                return args.event_enum;
            }
        }
    }
    abort!(proc_macro2::Span::mixed_site(), "Missing #[taskspec(event_enum = \"YourEventType\")] attribute on enum");
}

fn extract_callback_name(attrs: &[Attribute]) -> Option<syn::Ident> {
    for attr in attrs {
        if attr.path().is_ident("taskspec") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested = match NestedMeta::parse_meta_list(meta_list.tokens.clone()) {
                    Ok(nested) => nested,
                    Err(e) => abort!(attr.span(), "Invalid taskspec attribute syntax: {}", e),
                };
                let args = match VariantArgs::from_list(&nested) {
                    Ok(args) => args,
                    Err(e) => abort!(attr.span(), "Invalid taskspec attribute: {}", e),
                };
                return Some(args.callback);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {}
