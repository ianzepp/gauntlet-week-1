//! Root application component with routing and context providers.

use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, StaticSegment,
    components::{Route, Router, Routes},
};

use crate::pages::{board::BoardPage, dashboard::DashboardPage, login::LoginPage};
use crate::state::{ai::AiState, auth::AuthState, board::BoardState, chat::ChatState, ui::UiState};

/// Wrapper around the frame client sender, provided as Leptos context.
///
/// Components use this to send frames to the server. On the server (SSR),
/// this is `None`.
#[derive(Clone, Debug, Default)]
pub struct FrameSender {
    #[cfg(feature = "hydrate")]
    pub tx: Option<futures::channel::mpsc::UnboundedSender<String>>,
}

impl FrameSender {
    /// Send a frame to the server. Returns `false` if not connected.
    pub fn send(&self, frame: &crate::net::types::Frame) -> bool {
        #[cfg(feature = "hydrate")]
        {
            if let Some(ref tx) = self.tx {
                return crate::net::frame_client::send_frame(tx, frame);
            }
        }
        let _ = frame;
        false
    }
}

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
    let frame_sender = RwSignal::new(FrameSender::default());

    provide_context(auth);
    provide_context(board);
    provide_context(ui);
    provide_context(chat);
    provide_context(ai);
    provide_context(frame_sender);

    // Client-side initialization: fetch user and spawn frame client.
    Effect::new(move || {
        #[cfg(feature = "hydrate")]
        {
            // Fetch current user.
            auth.update(|a| a.loading = true);
            leptos::task::spawn_local(async move {
                let user = crate::net::api::fetch_current_user().await;
                auth.update(|a| {
                    a.user = user;
                    a.loading = false;
                });
            });

            // Spawn WebSocket frame client.
            let tx = crate::net::frame_client::spawn_frame_client(auth, ai, board, chat);
            frame_sender.update(|fs| fs.tx = Some(tx));

            // Initialize dark mode from stored preference.
            let dark = crate::util::dark_mode::read_preference();
            ui.update(|u| u.dark_mode = dark);
            crate::util::dark_mode::apply(dark);
        }
    });

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
