//! User profile popover with statistics, shown on avatar click.

use leptos::prelude::*;

use crate::net::types::UserProfile;

/// Popover that displays a user's profile and statistics.
///
/// Fetches the profile from `/api/users/:id/profile` on mount.
#[component]
pub fn UserFieldReport(user_id: String) -> impl IntoView {
    let uid = user_id.clone();
    let profile = LocalResource::new(move || {
        let uid = uid.clone();
        async move { crate::net::api::fetch_user_profile(&uid).await }
    });

    view! {
        <div class="user-field-report">
            <Suspense fallback=move || view! { <span>"Loading..."</span> }>
                {move || {
                    profile
                        .get()
                        .map(|p| {
                            if let Some(p) = p {
                                render_profile(p).into_any()
                            } else {
                                view! { <span>"User not found"</span> }.into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

fn render_profile(p: UserProfile) -> impl IntoView {
    view! {
        <div class="user-field-report__card">
            <div class="user-field-report__header">
                <span class="user-field-report__color-dot" style:background=p.color.clone()></span>
                <strong>{p.name}</strong>
            </div>
            <dl class="user-field-report__stats">
                <dt>"Frames"</dt>
                <dd>{p.stats.total_frames}</dd>
                <dt>"Objects"</dt>
                <dd>{p.stats.objects_created}</dd>
                <dt>"Boards"</dt>
                <dd>{p.stats.boards_active}</dd>
            </dl>
            {if p.stats.top_syscalls.is_empty() {
                None
            } else {
                Some(
                    view! {
                        <div class="user-field-report__syscalls">
                            <strong>"Top syscalls"</strong>
                            <ul>
                                {p
                                    .stats
                                    .top_syscalls
                                    .iter()
                                    .map(|sc| {
                                        view! {
                                            <li>
                                                {sc.syscall.clone()} " (" {sc.count} ")"
                                            </li>
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </ul>
                        </div>
                    },
                )
            }}
        </div>
    }
}
