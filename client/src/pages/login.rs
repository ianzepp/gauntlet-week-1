//! Login page supporting GitHub OAuth and email + access-code auth.

use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let code = RwSignal::new(String::new());
    let info = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let echoed_code = RwSignal::new(None::<String>);

    let on_request_code = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if busy.get() {
            return;
        }
        let email_value = email.get().trim().to_owned();
        if email_value.is_empty() {
            info.set("Enter an email first.".to_owned());
            return;
        }
        busy.set(true);
        info.set("Requesting code...".to_owned());
        echoed_code.set(None);

        #[cfg(feature = "hydrate")]
        leptos::task::spawn_local(async move {
            match crate::net::api::request_email_login_code(&email_value).await {
                Ok(code_opt) => {
                    echoed_code.set(code_opt);
                    info.set("Code generated. Check your email or use the echoed code below.".to_owned());
                }
                Err(e) => info.set(format!("Code request failed: {e}")),
            }
            busy.set(false);
        });
    };

    let on_verify_code = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if busy.get() {
            return;
        }
        let email_value = email.get().trim().to_owned();
        let code_value = code.get().trim().to_owned();
        if email_value.is_empty() || code_value.is_empty() {
            info.set("Enter both email and 6-char code.".to_owned());
            return;
        }
        busy.set(true);
        info.set("Verifying code...".to_owned());

        #[cfg(feature = "hydrate")]
        leptos::task::spawn_local(async move {
            match crate::net::api::verify_email_login_code(&email_value, &code_value).await {
                Ok(()) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/");
                    }
                }
                Err(e) => {
                    info.set(format!("Verification failed: {e}"));
                    busy.set(false);
                }
            }
        });
    };

    view! {
        <div class="login-page">
            <div class="login-card">
                <h1>"Gauntlet AI"</h1>
                <p class="login-card__subtitle">"Email Access Code"</p>
                <form class="login-form" on:submit=on_request_code>
                    <input
                        class="login-input"
                        type="email"
                        placeholder="you@example.com"
                        prop:value=move || email.get()
                        on:input=move |ev| email.set(event_target_value(&ev))
                    />
                    <button class="login-button" type="submit" disabled=move || busy.get()>
                        "Send 6-char Code"
                    </button>
                </form>
                <form class="login-form" on:submit=on_verify_code>
                    <input
                        class="login-input login-input--code"
                        type="text"
                        maxlength="6"
                        placeholder="ABC123"
                        prop:value=move || code.get()
                        on:input=move |ev| code.set(event_target_value(&ev).to_ascii_uppercase())
                    />
                    <button class="login-button" type="submit" disabled=move || busy.get()>
                        "Sign In With Code"
                    </button>
                </form>
                <Show when=move || !info.get().is_empty()>
                    <p class="login-message">{move || info.get()}</p>
                </Show>
                <Show when=move || echoed_code.get().is_some()>
                    <p class="login-message login-message--code">
                        "Code: "
                        <span>{move || echoed_code.get().unwrap_or_default()}</span>
                    </p>
                </Show>
                <div class="login-divider"></div>
                <p class="login-card__subtitle">"Or"</p>
                <a
                    href="/auth/github"
                    class="login-button"
                    on:click=move |ev| {
                        ev.prevent_default();
                        #[cfg(feature = "hydrate")]
                        {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href("/auth/github");
                            }
                        }
                    }
                >
                    "Sign in with GitHub"
                </a>
            </div>
        </div>
    }
}
