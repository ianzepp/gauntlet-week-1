//! Modal listing canvas keyboard/mouse shortcuts.

use leptos::prelude::*;

#[derive(Clone, Copy)]
struct ShortcutRow {
    action: &'static str,
    keys: &'static str,
}

const SHORTCUTS: &[ShortcutRow] = &[
    ShortcutRow { action: "Select object", keys: "Select tool + Click" },
    ShortcutRow { action: "Toggle selection", keys: "Shift + Click object" },
    ShortcutRow { action: "Clear selection", keys: "Click empty canvas / Esc" },
    ShortcutRow { action: "Marquee selection", keys: "Select tool + Drag empty canvas" },
    ShortcutRow { action: "Pan canvas", keys: "Space + Drag / Middle mouse drag / Trackpad pan" },
    ShortcutRow { action: "Zoom", keys: "Cmd/Ctrl + Mouse wheel" },
    ShortcutRow { action: "Move selection", keys: "Drag selected object(s)" },
    ShortcutRow { action: "Duplicate while dragging", keys: "Alt/Option + Drag" },
    ShortcutRow { action: "Axis lock while dragging", keys: "Shift + Drag" },
    ShortcutRow { action: "Nudge", keys: "Arrow keys" },
    ShortcutRow { action: "Large nudge", keys: "Shift + Arrow keys" },
    ShortcutRow { action: "Group", keys: "Cmd/Ctrl + G" },
    ShortcutRow { action: "Ungroup", keys: "Shift + Cmd/Ctrl + G" },
    ShortcutRow { action: "Select all", keys: "Cmd/Ctrl + A" },
];

/// Fullscreen modal with shortcut table.
#[component]
pub fn HelpShortcutsModal(on_close: Callback<()>) -> impl IntoView {
    let on_backdrop = move |_| on_close.run(());
    let on_close_click = move |_| on_close.run(());
    let on_keydown = Callback::new(move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            ev.prevent_default();
            on_close.run(());
        }
    });

    view! {
        <div class="help-shortcuts-modal__backdrop" on:click=on_backdrop>
            <div class="help-shortcuts-modal" on:click=move |ev| ev.stop_propagation() on:keydown=move |ev| on_keydown.run(ev) tabindex="0">
                <div class="help-shortcuts-modal__header">
                    <h2>"Help"</h2>
                    <button class="help-shortcuts-modal__close" on:click=on_close_click title="Close help">
                        "✕"
                    </button>
                </div>
                <div class="help-shortcuts-modal__subtitle">"Keyboard and mouse combinations"</div>
                <div class="help-shortcuts-modal__table-wrap">
                    <table class="help-shortcuts-modal__table">
                        <thead>
                            <tr>
                                <th>"Action"</th>
                                <th>"Shortcut"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {SHORTCUTS
                                .iter()
                                .map(|row| {
                                    view! {
                                        <tr>
                                            <td>{row.action}</td>
                                            <td class="help-shortcuts-modal__keys">{row.keys}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    }
}
