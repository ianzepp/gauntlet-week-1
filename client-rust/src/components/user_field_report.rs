//! User profile popover with statistics, shown on presence chip click.

use leptos::prelude::*;

use crate::net::types::UserProfile;

/// Popover that displays a user's profile and statistics.
#[component]
pub fn UserFieldReport(
    user_id: String,
    anchor_x: i32,
    #[prop(optional)] direction: Option<&'static str>,
    on_close: Callback<()>,
) -> impl IntoView {
    let direction = direction.unwrap_or("up");

    let uid = user_id.clone();
    let profile = LocalResource::new(move || {
        let uid = uid.clone();
        async move { crate::net::api::fetch_user_profile(&uid).await }
    });

    let popover_style = move || {
        let raw_left = anchor_x - 120;
        #[cfg(feature = "hydrate")]
        {
            let max_left = web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|v| v.as_f64())
                .map_or(1000.0, |w| w - 248.0)
                .round() as i32;
            let left = raw_left.max(8).min(max_left.max(8));
            if direction == "down" {
                return format!("left: {left}px; top: 36px; bottom: auto;");
            }
            return format!("left: {left}px; bottom: 32px; top: auto;");
        }

        #[cfg(not(feature = "hydrate"))]
        {
            let left = raw_left.max(8);
            if direction == "down" {
                return format!("left: {left}px; top: 36px; bottom: auto;");
            }
            format!("left: {left}px; bottom: 32px; top: auto;")
        }
    };

    view! {
        <div class="user-field-report">
            <div class="user-field-report__backdrop" on:click=move |_| on_close.run(())></div>

            <div class="user-field-report__popover" style=popover_style>
                <Suspense fallback=move || view! { <div class="user-field-report__loading">"Loading field report..."</div> }>
                    {move || {
                        profile
                            .get()
                            .map(|p| {
                                if let Some(p) = p {
                                    render_profile(p).into_any()
                                } else {
                                    view! { <div class="user-field-report__loading">"Agent not found"</div> }.into_any()
                                }
                            })
                    }}
                </Suspense>
            </div>
        </div>
    }
}

fn render_profile(p: UserProfile) -> impl IntoView {
    let top_syscalls = p.stats.top_syscalls.clone();
    let has_top_syscalls = !top_syscalls.is_empty();

    view! {
        <div class="user-field-report__card">
            <div class="user-field-report__header">
                {p
                    .avatar_url
                    .as_ref()
                    .map(|url| {
                        let src = url.clone();
                        let alt = p.name.clone();
                        view! {
                            <img class="user-field-report__avatar" src=src alt=alt/>
                        }
                    })}

                <div class="user-field-report__header-info">
                    <span class="user-field-report__name">{p.name}</span>
                    <span class="user-field-report__badge">
                        {if let Some(member_since) = p.member_since {
                            format!("Field Agent // Since {member_since}")
                        } else {
                            "Field Agent".to_owned()
                        }}
                    </span>
                </div>
            </div>

            <div class="user-field-report__section">
                <div class="user-field-report__section-title">"Activity Log"</div>

                <ReportRow label="Transmissions" value=p.stats.total_frames.to_string()/>
                <ReportRow label="Objects Created" value=p.stats.objects_created.to_string()/>
                <ReportRow label="Boards Active" value=p.stats.boards_active.to_string()/>

                {p
                    .stats
                    .last_active
                    .map(|last| {
                        view! { <ReportRow label="Last Signal" value=last/> }
                    })}
            </div>

            <Show when=move || has_top_syscalls>
                <div class="user-field-report__section">
                    <div class="user-field-report__section-title">"Top Operations"</div>
                    {top_syscalls
                        .iter()
                        .map(|sc| {
                            let name = sc.syscall.clone();
                            view! {
                                <div class="user-field-report__syscall-bar">
                                    <span class="user-field-report__syscall-name">{name}</span>
                                    <span class="user-field-report__syscall-count">{sc.count}</span>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>
            </Show>
        </div>
    }
}

#[component]
fn ReportRow(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="user-field-report__row">
            <span class="user-field-report__row-label">{label}</span>
            <span class="user-field-report__row-value">{value}</span>
        </div>
    }
}
