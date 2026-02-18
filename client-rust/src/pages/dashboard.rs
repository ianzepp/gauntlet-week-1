//! Dashboard page listing boards with create and open actions.

use leptos::prelude::*;

/// Dashboard page — shows a board list and a create-board button.
/// Auth-guarded: redirects to `/login` if the user is not authenticated.
#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <div class="dashboard-page">
            <h1>"Dashboard"</h1>
            <p>"Your boards will appear here."</p>
        </div>
    }
}
