use crate::{api::zmc_get_move_status, app::GlobalState};
use leptos::html::Canvas;
use leptos::prelude::*;
use leptos::{logging, prelude::*, reactive::spawn_local, server::codee::string::JsonSerdeCodec};
use leptos_use::storage::{use_storage, use_storage_with_options, UseStorageOptions};
use leptos_use::{use_cookie, watch_debounced};
use std::time::Duration;
use web_sys::wasm_bindgen::closure::Closure;
use web_sys::wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AxisMoveStatus {
    pub is_idle: bool,
    pub speed: f32,
    pub pos: f32,
}

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MoveStatus {
    pub x: AxisMoveStatus,
    pub y: AxisMoveStatus,
    pub z: AxisMoveStatus,
}

fn save_canvas_state(canvas_ref: NodeRef<Canvas>) {
    let (canvas_state, set_canvas_state, _reset) = use_storage::<String, JsonSerdeCodec>(
        leptos_use::storage::StorageType::Local,
        "canvas_storage",
    );
    // Get the canvas element
    let canvas: web_sys::HtmlCanvasElement = canvas_ref.get().unwrap();
    let data_url = canvas.to_data_url().unwrap();
    set_canvas_state.set(data_url);
}

fn restore_canvas_state(canvas_ref: NodeRef<Canvas>) {
    logging::log!("Restoring canvas state...");
    let (canvas_state, _set_canvas_state, _reset) = use_storage::<String, JsonSerdeCodec>(
        leptos_use::storage::StorageType::Local,
        "canvas_storage",
    );

    // Get the canvas element
    if let Some(canvas) = canvas_ref.get() {
        let data_url = canvas_state.get();

        // Check if we have stored data
        if !data_url.is_empty() {
            logging::log!("Found saved canvas data, loading image...");
            let img = web_sys::HtmlImageElement::new().unwrap();

            // Create a context
            let context: CanvasRenderingContext2d = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into()
                .unwrap();

            let img_clone = img.clone();
            // Create a closure for the onload event
            let closure = Closure::wrap(Box::new(move || {
                logging::log!("Image loaded, drawing to canvas");
                context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
                context
                    .draw_image_with_html_image_element(&img_clone, 0.0, 0.0)
                    .unwrap_or_else(|err| {
                        logging::log!("Error drawing image: {:?}", err);
                    });
            }) as Box<dyn FnMut()>);

            // Set the onload handler
            img.set_onload(Some(closure.as_ref().unchecked_ref()));

            // Set the src to start loading the image
            img.set_src(data_url.as_str());

            // We need to forget the closure to keep it alive
            closure.forget();
        } else {
            logging::log!("No saved canvas data found");
        }
    } else {
        logging::log!("Canvas reference not available");
    }
}

#[component]
fn PointVisual() -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let (smile_value, set_smile_value) = signal(0.0); // 1.0 means full smile
    let (canvas_ready, set_canvas_ready) = signal(false);

    let move_status = expect_context::<RwSignal<MoveStatus>>();

    Effect::new(move || {
        if canvas_ref.get().is_some() && !canvas_ready.get() {
            logging::log!("Canvas is ready, initializing drawing...");
            restore_canvas_state(canvas_ref);
            // Ensure the canvas is ready before drawing
            set_canvas_ready.set(true);
        }
    });

    Effect::new(move |_| {
        if canvas_ready.get_untracked() {
            // Get the canvas element
            let canvas: web_sys::HtmlCanvasElement = canvas_ref.get().unwrap();
            let uncanvas = canvas.get_context("2d").unwrap().unwrap();
            let context: CanvasRenderingContext2d = uncanvas.dyn_into().unwrap();
            context.begin_path();
            context
                .arc(
                    (move_status.read().x.pos / 100.0) as f64,
                    (move_status.read().y.pos / 100.0) as f64,
                    1.0,
                    0.0,
                    2.0 * std::f64::consts::PI,
                )
                .unwrap();
            context.fill();
        }
    });

    let _ = watch_debounced(
        move || move_status.get(),
        move |_, _, _| {
            logging::log!("save canvas state...");
            save_canvas_state(canvas_ref);
        },
        500.0,
    );

    view! {
        <input
            type="range"
            min="0.1"
            max="2.0"
            step="0.1"
            value="1.0"
            on:input=move |ev| {
                let value = event_target_value(&ev).parse::<f64>().unwrap();
                set_smile_value.set(value);
            }
        />
        <button on:click=move |_| {
            save_canvas_state(canvas_ref);
        }>"Save Canvas"</button>
        <button on:click=move |_| {
            restore_canvas_state(canvas_ref);
        }>"Restore Canvas"</button>
        <canvas width="400" height="400" style="border: 1px solid black" node_ref=canvas_ref />
    }
}

#[component]
fn AxisVisual() -> impl IntoView {
    let (global_state, set_global_state) =
        use_cookie::<GlobalState, JsonSerdeCodec>("global_state_cookie");
    // Ensure global state is initialized
    if global_state.read_untracked().is_none() {
        set_global_state.set(Some(GlobalState::default()));
    }

    let connected = move || global_state.get().unwrap().connected;

    let move_status = expect_context::<RwSignal<MoveStatus>>();
    let mut interval_handle = None;
    Effect::new(move || {
        if connected() {
            interval_handle = Some(
                set_interval_with_handle(
                    move || {
                        spawn_local(async move {
                            match zmc_get_move_status(0, 1, 2).await {
                                Ok(status) => {
                                    move_status.set(status);
                                }
                                Err(e) => {
                                    logging::error!("Failed to fetch move status: {}", e);
                                }
                            }
                        });
                    },
                    Duration::from_millis(100),
                )
                .unwrap(),
            );
        } else {
            if let Some(handle) = interval_handle {
                handle.clear();
            }
        }
    });

    view! {
        <Transition fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            <div class="axis-status">
                {move || {
                    if !connected() {
                        view! { <div class="error-message">"Not connected"</div> }
                    } else {
                        let status = move_status.get();
                        view! {
                            <div class="axis-status-container">
                                <div class="axis">
                                    <h3>"X Axis"</h3>
                                    <p>"Idle: " {if status.x.is_idle { "Yes" } else { "No" }}</p>
                                    <p>"Speed: " {status.x.speed} " mm/s"</p>
                                    <p>"Position: " {status.x.pos} " mm"</p>
                                </div>
                                <div class="axis">
                                    <h3>"Y Axis"</h3>
                                    <p>"Idle: " {if status.y.is_idle { "Yes" } else { "No" }}</p>
                                    <p>"Speed: " {status.y.speed} " mm/s"</p>
                                    <p>"Position: " {status.y.pos} " mm"</p>
                                </div>
                                <div class="axis">
                                    <h3>"Z Axis"</h3>
                                    <p>"Idle: " {if status.z.is_idle { "Yes" } else { "No" }}</p>
                                    <p>"Speed: " {status.z.speed} " mm/s"</p>
                                    <p>"Position: " {status.z.pos} " mm"</p>
                                </div>
                            </div>
                        }
                    }
                }}
            </div>
        </Transition>
    }
}

#[component]
pub fn VisualView() -> impl IntoView {
    let move_status = RwSignal::new(MoveStatus::default());
    provide_context(move_status);

    view! {
        <div class="status">
            <AxisVisual />
            <PointVisual />
        </div>
    }
}
