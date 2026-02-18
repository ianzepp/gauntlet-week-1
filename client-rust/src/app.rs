//! Root application component with routing and context providers.

use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, StaticSegment,
    components::{Route, Router, Routes},
};

use crate::pages::{board::BoardPage, dashboard::DashboardPage, login::LoginPage};
use crate::state::{ai::AiState, auth::AuthState, board::BoardState, chat::ChatState, ui::UiState};

/// HTML shell rendered on the server for SSR + hydration.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Root application component.
///
/// Provides all shared state contexts and sets up client-side routing.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // Provide reactive state contexts for all child components.
    let auth = RwSignal::new(AuthState::default());
    let board = RwSignal::new(BoardState::default());
    let ui = RwSignal::new(UiState::default());
    let chat = RwSignal::new(ChatState::default());
    let ai = RwSignal::new(AiState::default());

    provide_context(auth);
    provide_context(board);
    provide_context(ui);
    provide_context(chat);
    provide_context(ai);

    view! {
        <Stylesheet id="leptos" href="/pkg/gauntlet-ui.css"/>
        <Title text="Gauntlet"/>

        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("login") view=LoginPage/>
                <Route path=StaticSegment("") view=DashboardPage/>
                <Route path=(StaticSegment("board"), ParamSegment("id")) view=BoardPage/>
            </Routes>
        </Router>
    }
}
