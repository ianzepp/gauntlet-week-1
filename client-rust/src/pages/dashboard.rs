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

    let on_cancel = move |_| {
        show_create.set(false);
    };

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
                                        <p class="dashboard-page__empty">"No boards yet. Create one to get started."</p>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <div class="dashboard-page__cards">
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
    on_cancel: impl Fn(leptos::ev::MouseEvent) + 'static,
    boards: LocalResource<Vec<BoardListItem>>,
) -> impl IntoView {
    let on_submit = move |_| {
        let board_name = name.get();
        if board_name.trim().is_empty() {
            return;
        }
        // TODO: send board:create via frame client, then refetch boards
        leptos::logging::log!("create board: {}", board_name);
        boards.refetch();
    };

    view! {
        <div class="dialog-backdrop">
            <div class="dialog">
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
                    />
                </label>
                <div class="dialog__actions">
                    <button class="btn" on:click=on_cancel>
                        "Cancel"
                    </button>
                    <button class="btn btn--primary" on:click=on_submit>
                        "Create"
                    </button>
                </div>
            </div>
        </div>
    }
}
