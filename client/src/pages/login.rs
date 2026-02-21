//! Login page supporting GitHub OAuth and email + access-code auth.

#[cfg(test)]
#[path = "login_test.rs"]
mod login_test;

use leptos::prelude::*;

fn normalize_code_input(raw: &str) -> String {
    raw.to_ascii_uppercase()
}

fn validate_request_code_input(email: &str) -> Result<String, &'static str> {
    let email = email.trim().to_owned();
    if email.is_empty() {
        return Err("Enter an email first.");
    }
    Ok(email)
}

fn validate_verify_code_input(email: &str, code: &str) -> Result<(String, String), &'static str> {
    let email = email.trim().to_owned();
    let code = code.trim().to_owned();
    if email.is_empty() || code.len() != 6 {
        return Err("Enter both email and 6-char code.");
    }
    Ok((email, code))
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let code_slots = RwSignal::new(vec![String::new(); 6]);
    let info = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let echoed_code = RwSignal::new(None::<String>);
    let code_refs = (0..6)
        .map(|_| NodeRef::<leptos::html::Input>::new())
        .collect::<Vec<_>>();

    let on_request_code = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if busy.get() {
            return;
        }
        let _email_value = match validate_request_code_input(&email.get()) {
            Ok(email) => email,
            Err(msg) => {
                info.set(msg.to_owned());
                return;
            }
        };
        busy.set(true);
        info.set("Requesting code...".to_owned());
        echoed_code.set(None);

        #[cfg(feature = "hydrate")]
        leptos::task::spawn_local(async move {
            match crate::net::api::request_email_login_code(&_email_value).await {
                Ok(code_opt) => {
                    echoed_code.set(code_opt);
                    info.set("Code generated. Check your email or use the echoed code below.".to_owned());
                }
                Err(e) => info.set(format!("Code request failed: {e}")),
            }
            busy.set(false);
        });
    };

    let submit_verify = Callback::new(move |_| {
        if busy.get() {
            return;
        }
        let joined_code = code_slots.with(|slots| slots.join(""));
        let (_email_value, _code_value) = match validate_verify_code_input(&email.get(), &joined_code) {
            Ok(inputs) => inputs,
            Err(msg) => {
                info.set(msg.to_owned());
                return;
            }
        };
        busy.set(true);
        info.set("Verifying code...".to_owned());

        #[cfg(feature = "hydrate")]
        leptos::task::spawn_local(async move {
            match crate::net::api::verify_email_login_code(&_email_value, &_code_value).await {
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
    });

    let on_verify_code = {
        let submit_verify = submit_verify.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            submit_verify.run(());
        }
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
                    <Show when=move || echoed_code.get().is_some()>
                        <div class="login-dev-code">
                            <p class="login-dev-code__title">"Developer Mode - Use This Code"</p>
                            <div class="login-dev-code__letters" aria-label="Developer login code">
                                {move || {
                                    echoed_code
                                        .get()
                                        .unwrap_or_default()
                                        .chars()
                                        .take(6)
                                        .map(|ch| view! { <span class="login-dev-code__char">{ch.to_string()}</span> })
                                        .collect::<Vec<_>>()
                                }}
                            </div>
                        </div>
                    </Show>
                </form>
                <form class="login-form" on:submit=on_verify_code>
                    <label class="dialog__label">
                        "Access Code"
                        <div class="login-code-inputs">
                            {(0..6)
                                .map(|idx| {
                                    let code_refs_input = code_refs.clone();
                                    let code_refs_keydown = code_refs.clone();
                                    let submit_verify = submit_verify.clone();
                                    view! {
                                        <input
                                            class="login-code-input login-input--code"
                                            type="text"
                                            maxlength="1"
                                            inputmode="text"
                                            autocomplete=if idx == 0 { "one-time-code" } else { "off" }
                                            node_ref=code_refs[idx].clone()
                                            prop:value=move || code_slots.with(|slots| slots[idx].clone())
                                            on:input=move |ev| {
                                                let normalized = normalize_code_input(&event_target_value(&ev));
                                                let chars = normalized
                                                    .chars()
                                                    .filter(|ch| ch.is_ascii_alphanumeric())
                                                    .collect::<Vec<_>>();

                                                if chars.is_empty() {
                                                    code_slots.update(|slots| slots[idx].clear());
                                                    return;
                                                }

                                                let applied = chars.len().min(6 - idx);
                                                code_slots.update(|slots| {
                                                    for (offset, ch) in chars.iter().take(applied).enumerate() {
                                                        slots[idx + offset] = ch.to_string();
                                                    }
                                                });

                                                if code_slots.with(|slots| slots.iter().all(|slot| slot.len() == 1)) {
                                                    submit_verify.run(());
                                                    return;
                                                }

                                                let next_idx = idx + applied;
                                                if next_idx < 6
                                                    && let Some(next) = code_refs_input
                                                        .get(next_idx)
                                                        .and_then(NodeRef::get)
                                                {
                                                    let _ = next.focus();
                                                    let _ = next.select();
                                                }
                                            }
                                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                                if ev.key() != "Backspace" {
                                                    return;
                                                }
                                                if idx == 0 {
                                                    return;
                                                }

                                                let current_empty = code_slots.with(|slots| slots[idx].is_empty());
                                                if !current_empty {
                                                    return;
                                                }

                                                ev.prevent_default();
                                                code_slots.update(|slots| slots[idx - 1].clear());
                                                if let Some(prev) = code_refs_keydown
                                                    .get(idx - 1)
                                                    .and_then(NodeRef::get)
                                                {
                                                    let _ = prev.focus();
                                                    let _ = prev.select();
                                                }
                                            }
                                        />
                                    }
                                })
                                .collect::<Vec<_>>()}
                        </div>
                    </label>
                    <button class="login-button" type="submit" disabled=move || busy.get()>
                        "Sign In With Code"
                    </button>
                </form>
                <Show when=move || !info.get().is_empty()>
                    <p class="login-message">{move || info.get()}</p>
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
            <a
                href="https://github.com/ianzepp/gauntlet-week-1/"
                class="login-repo-link"
                target="_blank"
                rel="noopener noreferrer"
            >
                <span class="login-repo-link__label">"Repository"</span>
                <span class="login-repo-link__url">"github.com/ianzepp/gauntlet-week-1"</span>
            </a>
        </div>
    }
}
