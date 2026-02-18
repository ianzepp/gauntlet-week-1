//! Dashboard page listing boards with create and open actions.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::components::board_card::BoardCard;
use crate::net::api::BoardListItem;
use crate::state::auth::AuthState;

/// Dashboard page — shows a board list and a create-board button.
/// Redirects to `/login` if the user is not authenticated.
#[component]
pub fn DashboardPage() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let navigate = use_navigate();

    // Redirect to login if not authenticated.
    Effect::new(move || {
        let state = auth.get();
        if !state.loading && state.user.is_none() {
            navigate("/login", NavigateOptions::default());
        }
    });

    // Board list resource — fetches on mount.
    let boards = LocalResource::new(|| crate::net::api::fetch_boards());

    // Create-board dialog state.
    let show_create = RwSignal::new(false);
    let new_board_name = RwSignal::new(String::new());

    let on_create = move |_| {
        show_create.set(true);
        new_board_name.set(String::new());
    };

    let on_cancel = Callback::new(move |_| show_create.set(false));

    view! {
        <div class="dashboard-page">
            <header class="dashboard-page__header">
                <h1>"Boards"</h1>
                <button class="btn btn--primary" on:click=on_create>
                    "+ New Board"
                </button>
            </header>

            <div class="dashboard-page__grid">
                <Suspense fallback=move || view! { <p>"Loading boards..."</p> }>
                    {move || {
                        boards
                            .get()
                            .map(|list| {
                                if list.is_empty() {
                                    view! {
                                        <div class="dashboard-page__cards">
                                            <button class="dashboard-page__new-card" on:click=on_create title="Create board">
                                                <svg class="dashboard-page__new-icon" viewBox="0 0 20 20" aria-hidden="true">
                                                    <line x1="10" y1="4" x2="10" y2="16"></line>
                                                    <line x1="4" y1="10" x2="16" y2="10"></line>
                                                </svg>
                                            </button>
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <div class="dashboard-page__cards">
                                            <button class="dashboard-page__new-card" on:click=on_create title="Create board">
                                                <svg class="dashboard-page__new-icon" viewBox="0 0 20 20" aria-hidden="true">
                                                    <line x1="10" y1="4" x2="10" y2="16"></line>
                                                    <line x1="4" y1="10" x2="16" y2="10"></line>
                                                </svg>
                                            </button>
                                            {list
                                                .into_iter()
                                                .map(|b| {
                                                    view! { <BoardCard id=b.id name=b.name/> }
                                                })
                                                .collect::<Vec<_>>()}
                                        </div>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>
            </div>

            <Show when=move || show_create.get()>
                <CreateBoardDialog
                    name=new_board_name
                    on_cancel=on_cancel
                    boards=boards
                />
            </Show>
        </div>
    }
}

/// Modal dialog for creating a new board.
#[component]
fn CreateBoardDialog(
    name: RwSignal<String>,
    on_cancel: Callback<()>,
    boards: LocalResource<Vec<BoardListItem>>,
) -> impl IntoView {
    #[cfg(feature = "hydrate")]
    let navigate = use_navigate();

    let submit = Callback::new(move |_| {
        let board_name = name.get();
        if board_name.trim().is_empty() {
            return;
        }

        #[cfg(feature = "hydrate")]
        {
            let board_name = board_name.trim().to_owned();
            let navigate = navigate.clone();
            let boards = boards.clone();
                    leptos::task::spawn_local(async move {
                        if let Some(board) = crate::net::api::create_board(&board_name).await {
                            boards.refetch();
                            navigate(&format!("/board/{}", board.id), NavigateOptions::default());
                        }
                    });
        }

        #[cfg(not(feature = "hydrate"))]
        {
            let _ = board_name;
            let _ = &boards;
        }
    });

    view! {
        <div class="dialog-backdrop" on:click=move |_| on_cancel.run(())>
            <div class="dialog" on:click=move |ev| ev.stop_propagation()>
                <h2>"Create Board"</h2>
                <label class="dialog__label">
                    "Board Name"
                    <input
                        class="dialog__input"
                        type="text"
                        prop:value=move || name.get()
                        on:input=move |ev| {
                            name.set(event_target_value(&ev));
                        }
                        on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                ev.prevent_default();
                                submit.run(());
                            }
                        }
                    />
                </label>
                <div class="dialog__actions">
                    <button class="btn" on:click=move |_| on_cancel.run(())>
                        "Cancel"
                    </button>
                    <button class="btn btn--primary" on:click=move |_| submit.run(())>
                        "Create"
                    </button>
                </div>
            </div>
        </div>
    }
}
