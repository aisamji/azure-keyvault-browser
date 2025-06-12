use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{FnArg, ItemFn, parse_macro_input, parse_quote, parse2};
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

#[cfg(test)]
mod tests {}
