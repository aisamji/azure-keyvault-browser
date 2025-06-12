use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{FnArg, ItemFn, parse_macro_input, parse_quote, parse2, DeriveInput, Data, Fields, Variant, Attribute, Meta};
use darling::{FromMeta, ast::NestedMeta};

#[derive(FromMeta)]
struct MacroArgs {
    event_type: String,
    error_event: String,
}

#[proc_macro_attribute]
pub fn background_task_callback(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let attr_args = NestedMeta::parse_meta_list(args.into()).expect("Invalid attribute arguments");
    let args = MacroArgs::from_list(&attr_args).expect("Invalid macro arguments");
    background_task_callback_impl(input, args).into()
}

fn background_task_callback_impl(mut function: ItemFn, args: MacroArgs) -> TokenStream2 {
    function.sig.inputs.insert(0, event_sender_arg(&args.event_type));
    let update_progress_macro = update_progress_macro();
    let abort_macro = abort_macro(&args.error_event);
    let block = function.block.clone();
    function.block = parse2(quote! {
        {
            #update_progress_macro
            #abort_macro
            #block
        }
    })
    .unwrap();
    quote! { #function }
}


fn event_sender_arg(event_type: &str) -> FnArg {
    let event_type_ident: syn::Type = syn::parse_str(event_type).unwrap();
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

fn abort_macro(error_event: &str) -> TokenStream2 {
    let error_event_path: syn::Path = syn::parse_str(error_event).expect("Invalid error event variant");
    quote! {
        macro_rules! abort {
            ($($arg:tt)*) => {
                let message = format!($($arg)*);
                let _ = tx.send(#error_event_path(message.clone())).await.inspect_err(|_| eprintln!("{}", message));
                return;
            }
        }
    }
}

#[derive(FromMeta)]
struct TaskSpecArgs {
    #[darling(default)]
    callback: Option<String>,
    #[darling(default)]
    event_type: Option<String>,
}

#[proc_macro_derive(BackgroundTaskSpec, attributes(taskspec))]
pub fn background_task_spec_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    background_task_spec_impl(input).into()
}

fn background_task_spec_impl(input: DeriveInput) -> TokenStream2 {
    let enum_name = &input.ident;
    
    // Parse the event_type from the derive macro arguments
    let event_type = extract_event_type(&input.attrs);
    let event_type_ident: syn::Type = syn::parse_str(&event_type).unwrap();
    
    let variants = match input.data {
        Data::Enum(data_enum) => data_enum.variants,
        _ => panic!("BackgroundTaskSpec can only be derived for enums"),
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
            quote! {
                Self::#variant_name => {
                    let tx_clone = tx.clone();
                    tokio::task::spawn(async move {
                        #callback_name(tx_clone).await;
                    })
                }
            }
        }
        Fields::Unnamed(fields) => {
            let field_names: Vec<syn::Ident> = (0..fields.unnamed.len())
                .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                .collect();
            
            let pattern = quote! { Self::#variant_name(#(#field_names),*) };
            let args = quote! { tx_clone, #(#field_names),* };
            
            quote! {
                #pattern => {
                    let tx_clone = tx.clone();
                    tokio::task::spawn(async move {
                        #callback_name(#args).await;
                    })
                }
            }
        }
        Fields::Named(fields) => {
            let field_names: Vec<&syn::Ident> = fields.named.iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect();
            
            let pattern = quote! { Self::#variant_name { #(#field_names),* } };
            let args = quote! { tx_clone, #(#field_names),* };
            
            quote! {
                #pattern => {
                    let tx_clone = tx.clone();
                    tokio::task::spawn(async move {
                        #callback_name(#args).await;
                    })
                }
            }
        }
    }
}

fn extract_event_type(attrs: &[Attribute]) -> String {
    for attr in attrs {
        if attr.path().is_ident("taskspec") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested = NestedMeta::parse_meta_list(meta_list.tokens.clone())
                    .expect("Invalid taskspec attribute");
                let args = TaskSpecArgs::from_list(&nested)
                    .expect("Invalid taskspec attribute");
                if let Some(event_type) = args.event_type {
                    return event_type;
                }
            }
        }
    }
    panic!("Missing taskspec attribute with event_type parameter on enum");
}

fn extract_callback_name(attrs: &[Attribute]) -> syn::Ident {
    for attr in attrs {
        if attr.path().is_ident("taskspec") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested = NestedMeta::parse_meta_list(meta_list.tokens.clone())
                    .expect("Invalid taskspec attribute");
                let args = TaskSpecArgs::from_list(&nested)
                    .expect("Invalid taskspec attribute");
                if let Some(callback) = args.callback {
                    return syn::Ident::new(&callback, proc_macro2::Span::call_site());
                }
            }
        }
    }
    panic!("Missing taskspec attribute with callback parameter on variant");
}

#[cfg(test)]
mod tests {}
