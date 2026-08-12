// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Toast feedback helpers over `thaw`'s toaster: every mutation (upload, save,
//! delete, commit, create) reports its outcome as a transient toast instead of
//! scattered inline text.
//!
//! The shell mounts the `thaw::ToasterProvider`; screens call these with
//! `thaw::ToasterInjection::expect_context()`.

use leptos::prelude::*;

/// Dispatch a success toast.
pub fn toast_success(toaster: thaw::ToasterInjection, title: &str, body: &str) {
    dispatch(toaster, thaw::ToastIntent::Success, title, body);
}

/// Dispatch an error toast.
pub fn toast_error(toaster: thaw::ToasterInjection, title: &str, body: &str) {
    dispatch(toaster, thaw::ToastIntent::Error, title, body);
}

fn dispatch(toaster: thaw::ToasterInjection, intent: thaw::ToastIntent, title: &str, body: &str) {
    let title = title.to_owned();
    let body = body.to_owned();
    toaster.dispatch_toast(
        move || {
            view! {
                <thaw::Toast>
                    <thaw::ToastTitle>{title}</thaw::ToastTitle>
                    <thaw::ToastBody>{body}</thaw::ToastBody>
                </thaw::Toast>
            }
        },
        thaw::ToastOptions::default().with_intent(intent),
    );
}
