//! Property inspector for selected board objects.
//!
//! ARCHITECTURE
//! ============
//! Inspector edits are emitted as partial object updates so multiple clients
//! can converge through server-side versioned object state.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::{BoardObject, Frame, FrameStatus};
use crate::state::board::BoardState;

#[cfg(test)]
#[path = "inspector_panel_test.rs"]
mod inspector_panel_test;

/// Inspector panel with editable fields for the selected object.
#[component]
pub fn InspectorPanel() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();

    let selected_objects = move || {
        let state = board.get();
        state
            .selection
            .iter()
            .filter_map(|id| state.objects.get(id).cloned())
            .collect::<Vec<_>>()
    };

    let selected_object = move || {
        let objs = selected_objects();
        if objs.len() == 1 { objs.into_iter().next() } else { None }
    };

    let draft_width = RwSignal::new(String::new());
    let draft_height = RwSignal::new(String::new());
    let draft_title = RwSignal::new(String::new());
    let draft_body = RwSignal::new(String::new());
    let draft_font_size = RwSignal::new("13".to_owned());
    let draft_background = RwSignal::new("#d94b4b".to_owned());
    let draft_border = RwSignal::new("#d94b4b".to_owned());
    let draft_border_width = RwSignal::new("0".to_owned());

    Effect::new(move || {
        if let Some(obj) = selected_object() {
            draft_width.set(format_number_input(obj.width));
            draft_height.set(format_number_input(obj.height));
            draft_title.set(read_prop_str(&obj, "title").unwrap_or_default());
            draft_body.set(read_prop_str(&obj, "text").unwrap_or_default());
            draft_font_size.set(read_prop_int(&obj, "fontSize", 13).to_string());

            let bg = normalize_hex_color(
                read_prop_str(&obj, "backgroundColor").or_else(|| read_prop_str(&obj, "color")),
                "#d94b4b",
            );
            let border = normalize_hex_color(read_prop_str(&obj, "borderColor"), &bg);
            draft_background.set(bg.clone());
            draft_border.set(border);
            draft_border_width.set(read_prop_int(&obj, "borderWidth", 0).to_string());
        }
    });

    let send_update = move |board_id: String, data: serde_json::Value| {
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(board_id),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data,
        };
        sender.get().send(&frame);
    };

    let commit_dimension = move |key: &'static str, value: String| {
        let Some(obj) = selected_object() else {
            return;
        };

        let parsed = parse_integer_input(&value).unwrap_or_else(|| {
            if key == "width" {
                obj.width.unwrap_or(1.0) as i64
            } else {
                obj.height.unwrap_or(1.0) as i64
            }
        });
        let next = parsed.max(1) as f64;

        let current = if key == "width" {
            obj.width.unwrap_or(1.0)
        } else {
            obj.height.unwrap_or(1.0)
        };

        if current.round() == next.round() {
            if key == "width" {
                draft_width.set(format_number_input(Some(current)));
            } else {
                draft_height.set(format_number_input(Some(current)));
            }
            return;
        }

        board.update(|b| {
            if let Some(existing) = b.objects.get_mut(&obj.id) {
                if key == "width" {
                    existing.width = Some(next);
                } else {
                    existing.height = Some(next);
                }
            }
        });

        let mut data = serde_json::Map::new();
        data.insert("id".to_owned(), serde_json::Value::String(obj.id.clone()));
        data.insert("version".to_owned(), serde_json::json!(obj.version));
        data.insert(key.to_owned(), serde_json::json!(next));
        send_update(obj.board_id.clone(), serde_json::Value::Object(data));

        if key == "width" {
            draft_width.set((next.round() as i64).to_string());
        } else {
            draft_height.set((next.round() as i64).to_string());
        }
    };

    let commit_props = move |patch: serde_json::Map<String, serde_json::Value>| {
        let Some(obj) = selected_object() else {
            return;
        };

        let mut props = obj.props.as_object().cloned().unwrap_or_default();
        let mut changed = false;

        for (k, v) in patch {
            if props.get(&k) != Some(&v) {
                changed = true;
                props.insert(k, v);
            }
        }

        if !changed {
            return;
        }

        let next_props = serde_json::Value::Object(props.clone());

        board.update(|b| {
            if let Some(existing) = b.objects.get_mut(&obj.id) {
                existing.props = next_props.clone();
            }
        });

        send_update(
            obj.board_id.clone(),
            serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "props": next_props,
            }),
        );
    };

    let commit_title = move || {
        let mut patch = serde_json::Map::new();
        patch.insert("title".to_owned(), serde_json::json!(draft_title.get()));
        commit_props(patch);
    };

    let commit_body = move || {
        let mut patch = serde_json::Map::new();
        patch.insert("text".to_owned(), serde_json::json!(draft_body.get()));
        commit_props(patch);
    };

    let commit_font_size = move |value: String| {
        let Some(obj) = selected_object() else {
            return;
        };

        let current = read_prop_int(&obj, "fontSize", 13);
        let next = parse_integer_input(&value).unwrap_or(current).max(1);
        draft_font_size.set(next.to_string());

        let mut patch = serde_json::Map::new();
        patch.insert("fontSize".to_owned(), serde_json::json!(next));
        commit_props(patch);
    };

    let commit_background = move |value: String| {
        let next = normalize_hex_color(Some(value), "#d94b4b");
        draft_background.set(next.clone());

        let mut patch = serde_json::Map::new();
        patch.insert("color".to_owned(), serde_json::json!(next.clone()));
        patch.insert("backgroundColor".to_owned(), serde_json::json!(next));
        commit_props(patch);
    };

    let commit_border = move |value: String| {
        let next = normalize_hex_color(Some(value), &draft_background.get());
        draft_border.set(next.clone());

        let mut patch = serde_json::Map::new();
        patch.insert("borderColor".to_owned(), serde_json::json!(next));
        commit_props(patch);
    };

    let commit_border_width = move |value: String| {
        let Some(obj) = selected_object() else {
            return;
        };

        let current = read_prop_int(&obj, "borderWidth", 0).max(0);
        let next = parse_integer_input(&value).unwrap_or(current).max(0);
        draft_border_width.set(next.to_string());

        let mut patch = serde_json::Map::new();
        patch.insert("borderWidth".to_owned(), serde_json::json!(next));
        commit_props(patch);
    };

    let on_delete = move |_| {
        let Some(obj) = selected_object() else {
            return;
        };

        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id.clone()),
            from: None,
            syscall: "object:delete".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({ "id": obj.id }),
        };
        sender.get().send(&frame);

        board.update(|b| {
            b.objects.remove(&obj.id);
            b.selection.remove(&obj.id);
        });
    };

    view! {
        <div class="inspector-panel">
            {move || {
                let selected_count = selected_objects().len();

                if selected_count == 0 {
                    return view! {
                        <div class="inspector-panel__empty">
                            <span class="inspector-panel__empty-label">"No selection"</span>
                            <span class="inspector-panel__empty-hint">"Double click an object to inspect it"</span>
                        </div>
                    }
                        .into_any();
                }

                let Some(obj) = selected_object() else {
                    return view! {
                        <div class="inspector-panel__section">
                            <span class="inspector-panel__kind">{format!("{} objects selected", selected_count)}</span>
                        </div>
                    }
                        .into_any();
                };

                let object_kind = obj.kind.replace('_', " ");
                let short_id = obj.id.chars().take(8).collect::<String>();

                view! {
                    <div class="inspector-panel__section">
                        <span class="inspector-panel__kind">{object_kind}</span>
                    </div>

                    <div class="inspector-panel__section">
                        <span class="inspector-panel__section-title">"Object Size"</span>
                        <div class="inspector-panel__field-grid">
                            <label class="inspector-panel__label" for="inspector-width">"W"</label>
                            <input
                                id="inspector-width"
                                class="inspector-panel__input"
                                inputmode="numeric"
                                prop:value=move || draft_width.get()
                                on:input=move |ev| draft_width.set(event_target_value(&ev))
                                on:blur=move |_| commit_dimension("width", draft_width.get())
                                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                    if ev.key() == "Enter" {
                                        ev.prevent_default();
                                        commit_dimension("width", draft_width.get());
                                    }
                                }
                            />

                            <label class="inspector-panel__label" for="inspector-height">"H"</label>
                            <input
                                id="inspector-height"
                                class="inspector-panel__input"
                                inputmode="numeric"
                                prop:value=move || draft_height.get()
                                on:input=move |ev| draft_height.set(event_target_value(&ev))
                                on:blur=move |_| commit_dimension("height", draft_height.get())
                                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                    if ev.key() == "Enter" {
                                        ev.prevent_default();
                                        commit_dimension("height", draft_height.get());
                                    }
                                }
                            />
                        </div>
                    </div>

                    <div class="inspector-panel__section">
                        <span class="inspector-panel__section-title">"Text Content"</span>

                        <Show when=move || obj.kind == "sticky_note">
                            <div class="inspector-panel__inline">
                                <label class="inspector-panel__label" for="inspector-title">"Title"</label>
                                <input
                                    id="inspector-title"
                                    class="inspector-panel__input"
                                    prop:value=move || draft_title.get()
                                    on:input=move |ev| draft_title.set(event_target_value(&ev))
                                    on:blur=move |_| commit_title()
                                    on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                        if ev.key() == "Enter" {
                                            ev.prevent_default();
                                            commit_title();
                                        }
                                    }
                                />
                            </div>
                        </Show>

                        <textarea
                            class="inspector-panel__text-area"
                            prop:value=move || draft_body.get()
                            on:input=move |ev| draft_body.set(event_target_value(&ev))
                            on:blur=move |_| commit_body()
                            placeholder="Type object text"
                        ></textarea>

                        <div class="inspector-panel__inline">
                            <label class="inspector-panel__label" for="inspector-font-size">"Font Size"</label>
                            <input
                                id="inspector-font-size"
                                class="inspector-panel__input"
                                inputmode="numeric"
                                prop:value=move || draft_font_size.get()
                                on:input=move |ev| draft_font_size.set(event_target_value(&ev))
                                on:blur=move |_| commit_font_size(draft_font_size.get())
                                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                    if ev.key() == "Enter" {
                                        ev.prevent_default();
                                        commit_font_size(draft_font_size.get());
                                    }
                                }
                            />
                        </div>
                    </div>

                    <div class="inspector-panel__section">
                        <span class="inspector-panel__section-title">"Appearance"</span>

                        <div class="inspector-panel__color-row">
                            <label class="inspector-panel__label" for="inspector-background">"Background"</label>
                            <input
                                id="inspector-background"
                                class="inspector-panel__color-input"
                                type="color"
                                prop:value=move || draft_background.get()
                                on:input=move |ev| commit_background(event_target_value(&ev))
                            />
                        </div>

                        <div class="inspector-panel__color-row">
                            <label class="inspector-panel__label" for="inspector-border">"Border"</label>
                            <input
                                id="inspector-border"
                                class="inspector-panel__color-input"
                                type="color"
                                prop:value=move || draft_border.get()
                                on:input=move |ev| commit_border(event_target_value(&ev))
                            />
                        </div>

                        <div class="inspector-panel__inline">
                            <label class="inspector-panel__label" for="inspector-border-width">"Border Width"</label>
                            <input
                                id="inspector-border-width"
                                class="inspector-panel__input"
                                inputmode="numeric"
                                prop:value=move || draft_border_width.get()
                                on:input=move |ev| draft_border_width.set(event_target_value(&ev))
                                on:blur=move |_| commit_border_width(draft_border_width.get())
                                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                    if ev.key() == "Enter" {
                                        ev.prevent_default();
                                        commit_border_width(draft_border_width.get());
                                    }
                                }
                            />
                        </div>
                    </div>

                    <div class="inspector-panel__section inspector-panel__meta">
                        <span class="inspector-panel__section-title">"Position / Meta"</span>
                        <MetaRow label="X" value=format!("{:.0}", obj.x)/>
                        <MetaRow label="Y" value=format!("{:.0}", obj.y)/>
                        <MetaRow label="Rot" value=format!("{:.0}°", obj.rotation)/>
                        <MetaRow label="Z" value=obj.z_index.to_string()/>
                        <MetaRow label="Ver" value=obj.version.to_string()/>
                        <MetaRow label="ID" value=short_id/>

                        <button class="btn btn--danger inspector-panel__delete" on:click=on_delete>
                            "Delete Object"
                        </button>
                    </div>
                }
                    .into_any()
            }}
        </div>
    }
}

#[component]
fn MetaRow(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="inspector-panel__meta-row">
            <span class="inspector-panel__label">{label}</span>
            <span class="inspector-panel__value">{value}</span>
        </div>
    }
}

fn format_number_input(value: Option<f64>) -> String {
    value.map_or_else(String::new, |v| format!("{:.0}", v.round()))
}

fn parse_integer_input(value: &str) -> Option<i64> {
    value.trim().parse::<i64>().ok()
}

fn read_prop_str(obj: &BoardObject, key: &str) -> Option<String> {
    obj.props
        .get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
}

fn read_prop_int(obj: &BoardObject, key: &str, fallback: i64) -> i64 {
    obj.props
        .get(key)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|n| n.round() as i64)))
        .unwrap_or(fallback)
}

fn normalize_hex_color(value: Option<String>, fallback: &str) -> String {
    let Some(raw) = value else {
        return fallback.to_owned();
    };

    let trimmed = raw.trim();
    if trimmed.len() == 4 && trimmed.starts_with('#') {
        let chars: Vec<char> = trimmed[1..].chars().collect();
        if chars.len() == 3 && chars.iter().all(|c| c.is_ascii_hexdigit()) {
            return format!("#{}{}{}{}{}{}", chars[0], chars[0], chars[1], chars[1], chars[2], chars[2]).to_lowercase();
        }
    }

    if trimmed.len() == 7 && trimmed.starts_with('#') && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return trimmed.to_lowercase();
    }

    fallback.to_owned()
}
