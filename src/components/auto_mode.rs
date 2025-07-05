use std::cell::RefCell;
use std::rc::Rc;

use crate::model::Parameters;
use crate::{app::GlobalState, model::LimitStatus};
use leptos::{logging, prelude::*, server::codee::string::JsonSerdeCodec};
use leptos::{
    task::spawn_local,
    wasm_bindgen::{prelude::Closure, JsCast},
    *,
};
use leptos_use::{use_cookie, use_interval, use_interval_fn, utils::Pausable, UseIntervalReturn};
use leptos_ws::ServerSignal;
use thaw::*;
use web_sys::{HtmlElement, MouseEvent, ScrollToOptions};

use crate::api::{
    debug_update_line, generate_path_preview, load_gcode, start_gcode_execution,
    stop_gcode_execution, zmc_init_eth, zmc_init_fake,
};

fn highlight_gcode(line: &str) -> impl IntoView {
    // Skip empty lines
    if line.trim().is_empty() {
        return view! {
            <div>
                <span>{line}</span>
            </div>
        };
    }

    // Check for comments
    if let Some(comment_pos) = line.find(';') {
        let (code_part, comment_part) = line.split_at(comment_pos);
        return view! {
            <div>
                <span>{highlight_gcode_command(code_part)}</span>
                <span class="comment">{comment_part}</span>
            </div>
        };
    }

    // Highlight regular G-code
    view! {
        <div>
            <span>{highlight_gcode_command(line)}</span>
        </div>
    }
}

fn highlight_gcode_command(code: &str) -> impl IntoView {
    // Preallocate vec with estimated capacity to avoid reallocations
    let mut result = Vec::with_capacity(code.len() / 3);
    let mut current_pos = 0;
    let code_bytes = code.as_bytes();
    let code_len = code_bytes.len();

    // Optimization: process in chunks instead of char by char
    let mut i = 0;
    while i < code_len {
        let c = code_bytes[i] as char;

        // Command detection (G1, M3, T2, etc)
        if i == 0 && (c == 'G' || c == 'M' || c == 'T') {
            // Find command number end (first non-digit)
            let mut end_pos = i + 1;
            while end_pos < code_len && code_bytes[end_pos].is_ascii_digit() {
                end_pos += 1;
            }

            if end_pos > i + 1 {
                result.push(view! { <span class="command">{&code[i..end_pos]}</span> });
                current_pos = end_pos;
                i = end_pos;
                continue;
            }
        }
        // Parameter detection (X100, Y-20.5, etc)
        else if current_pos <= i && c.is_ascii_alphabetic() && i + 1 < code_len {
            let param_start = i;

            // Find parameter value end efficiently
            let mut param_end = i + 1;
            while param_end < code_len {
                let byte = code_bytes[param_end];
                if byte.is_ascii_digit() || byte == b'.' || byte == b'-' {
                    param_end += 1;
                } else {
                    break;
                }
            }

            if param_end > param_start + 1 {
                result
                    .push(view! { <span class="parameter">{&code[param_start..param_end]}</span> });
                current_pos = param_end;
                i = param_end;
                continue;
            }
        }

        // Move to next character
        i += 1;
    }

    // Add any remaining text (if any)
    if current_pos < code_len {
        result.push(view! { <span class="remaining-text">{&code[current_pos..]}</span> });
    }

    view! { <div>{result}</div> }
}

#[component]
pub fn AutoModeView() -> impl IntoView {
    let (global_state, set_global_state) =
        use_cookie::<GlobalState, JsonSerdeCodec>("global_state_cookie");
    // Ensure global state is initialized
    if global_state.read_untracked().is_none() {
        set_global_state.set(Some(GlobalState::default()));
    }
    let connected = move || global_state.get().unwrap().connected;

    let file_content = RwSignal::new(String::new());
    let current_line = ServerSignal::new("current_line".to_string(), 0usize).unwrap();
    // let current_line = use_context::<ServerSignal<Cu>>();

    let (ip_addr, set_ip_addr) = use_cookie::<String, JsonSerdeCodec>("ip_addr_cookie");
    let preview_processed_line = ServerSignal::new("preview_processed_line".to_string(), 0usize)
        .expect("Failed to create client signal");
    // Ensure global state is initialized
    if ip_addr.read_untracked().is_none() {
        set_ip_addr.set(Some(String::new()));
    }

    let custom_request = move |file_list: web_sys::FileList| {
        if file_list.length() > 0 {
            let file = file_list.get(0).expect("Failed to get file");
            // Create a closure to handle the file content
            let file_loaded = Closure::wrap(Box::new(move |event: web_sys::ProgressEvent| {
                let target = event.target().expect("Event should have a target");
                let reader: web_sys::FileReader =
                    target.dyn_into().expect("Target should be a FileReader");

                match reader.result() {
                    Ok(content) => {
                        // For text files
                        if let Some(text) = content.as_string() {
                            let text_clone = text.clone();
                            spawn_local(async move {
                                load_gcode(text_clone).await.expect("Failed to load G-code");
                            });
                            file_content.set(text);
                        }
                        // For binary files (as ArrayBuffer)
                        else {
                            logging::log!("Binary file loaded");
                            // Handle binary content
                        }
                    }
                    Err(e) => logging::error!("Error reading file: {:?}", e),
                }
            })
                as Box<dyn FnMut(web_sys::ProgressEvent)>);
            // Create FileReader and set up the onload handler
            let reader = web_sys::FileReader::new().expect("Failed to create FileReader");
            reader.set_onload(Some(file_loaded.as_ref().unchecked_ref()));
            // Start reading the file as text
            if let Err(e) = reader.read_as_text(&file) {
                logging::error!("Error initiating file read: {:?}", e);
            }
            // Keep the closure alive
            file_loaded.forget();
        }
    };

    let lines_per_second = RwSignal::new(0f32);
    let current_line_clone = current_line.clone();
    let time_used = RwSignal::new(0);

    let Pausable {
        pause: interval_pause,
        resume: interval_resume,
        ..
    } = use_interval_fn(
        move || {
            if time_used.get_untracked() == 0 {
                // Reset lines per second at the start
                lines_per_second.set(0.0);
            } else {
                lines_per_second.set(
                    current_line_clone.get_untracked() as f32 / time_used.get_untracked() as f32,
                );
            }
            time_used.update(|t| *t += 1);
        },
        1000,
    );
    interval_pause();

    let on_start_click = move |_: MouseEvent| {
        interval_resume();
        spawn_local(async move {
            start_gcode_execution()
                .await
                .expect("Failed to start G-code execution");
        });
    };
    let on_stop_click = move |_: MouseEvent| {
        interval_pause();
        spawn_local(async move {
            stop_gcode_execution()
                .await
                .expect("Failed to stop G-code execution");
        });
    };

    let on_debug_click = move |_: MouseEvent| {
        spawn_local(async move {
            generate_path_preview()
                .await
                .expect("Failed to generate path preview");
        });
    };

    let on_genenrate_preview_click = move |_: MouseEvent| {
        spawn_local(async move {
            generate_path_preview()
                .await
                .expect("Failed to generate path preview");
        });
    };

    let current_line_clone = current_line.clone();
    let scrollbar_ref = ComponentRef::<ScrollbarRef>::new();
    Effect::new(move |_| {
        // Watch for changes to processing_line
        let current_line = current_line_clone.get();

        // Skip if we're at the beginning
        if current_line == 0 || !scrollbar_ref.get().is_some() {
            return;
        }

        // Schedule scroll after render is complete
        request_animation_frame(move || {
            // Find the current line element directly from the document
            if let Some(document) = web_sys::window().and_then(|win| win.document()) {
                if let Some(current_elem) = document.query_selector(".current-line").ok().flatten()
                {
                    // Smooth scroll to the element
                    let _ = current_elem.scroll_into_view_with_bool(true); // true = smooth scrolling
                }
            }
        });
    });

    let preview_processed_line_clone = preview_processed_line.clone();
    let current_line_clone = current_line.clone();
    view! {
        <Flex>
            <Flex vertical=true>
                <Label class="auto-mode-label">
                    {move || format!("Time used: {}s", time_used.get())}
                </Label>
                <Label class="auto-mode-label">
                    {move || format!("{:.2} lines/s", lines_per_second.get())}
                </Label>
                <div class="auto-mode-label">
                    {move || {
                        let total_lines = file_content.read().lines().count();
                        let lines_per_second = lines_per_second.get();
                        if lines_per_second == 0.0 {
                            "infinity".to_string()
                        } else {
                            let seconds = total_lines as f32 / lines_per_second;
                            logging::log!("Estimated time: {:.0} seconds", seconds);
                            format!(
                                "Estimated time: {:.0}h:{:.0}m:{:.0}s",
                                (seconds / 3600.0).floor(),
                                ((seconds % 3600.0) / 60.0).floor(),
                                seconds % 60.0,
                            )
                        }
                    }}
                </div>
            </Flex>
            <Flex vertical=true>
                <Button on_click=on_genenrate_preview_click>"Generate"</Button>
                {move || {
                    let preview_processed_line = *preview_processed_line_clone.read();
                    let total_lines = file_content.read().lines().count();
                    if preview_processed_line < total_lines {
                        view! {
                            <div>
                                <Spinner size=SpinnerSize::Medium>
                                    {format!(
                                        "processed line: {}/{}",
                                        preview_processed_line,
                                        total_lines,
                                    )}
                                </Spinner>
                            </div>
                        }
                    } else {
                        view! { <div>""</div> }
                    }
                }}
            </Flex>
        </Flex>
        <Flex vertical=true>
            <div class="status-container">
                <ProgressCircle
                    value=Signal::derive(move || {
                        let total_lines = file_content.get().lines().count() as f64;
                        if total_lines == 0.0 {
                            100.0
                        } else {
                            ((current_line.get() as f64 / total_lines) * 100.0 * 100.0).round()
                                / 100.0
                        }
                    })
                    color=ProgressCircleColor::Success
                />
            </div>
            <div class="control-container">
                <Upload custom_request>
                    <Button>"upload"</Button>
                </Upload>
                <Button on_click=on_start_click disabled=Signal::derive(move || !connected())>
                    "Start"
                </Button>
                <Button on_click=on_stop_click disabled=Signal::derive(move || !connected())>
                    "Stop"
                </Button>
            </div>
        </Flex>
        <div class="file-content">
            <p>"G-code Content:"</p>
            <Scrollbar
                style="max-height: 300px;"
                class="gcode-scrollbar"
                // Add the node reference here
                comp_ref=scrollbar_ref
            >
                <pre style="text-align: left;" class="gcode-display">
                    {move || {
                        let content = file_content.get();
                        let current = current_line_clone.get();
                        const VISIBLE_WINDOW: usize = 100;
                        const BUFFER_ZONE: usize = 10;
                        let total_lines = content.lines().count();
                        let (start_line, end_line) = Memo::new(move |
                                prev_bounds: Option<&(usize, usize)>|
                            {
                                let ideal_start = current.saturating_sub(VISIBLE_WINDOW / 2);
                                let ideal_end = (ideal_start + VISIBLE_WINDOW).min(total_lines);
                                if let Some(&(prev_start, prev_end)) = prev_bounds {
                                    let distance_from_start = if current >= prev_start {
                                        current - prev_start
                                    } else {
                                        0
                                    };
                                    let distance_from_end = if current < prev_end {
                                        prev_end - current
                                    } else {
                                        0
                                    };
                                    if distance_from_start >= BUFFER_ZONE
                                        && distance_from_end >= BUFFER_ZONE
                                    {
                                        return (prev_start, prev_end);
                                    }
                                }
                                (ideal_start, ideal_end)
                            })
                            .get();
                        let before_placeholder = if start_line > 0 {
                            Some(
                                // Use saturating arithmetic for all distance calculations
                                view! {
                                    <div class="gcode-line-placeholder">
                                        <span>{format!("... {} more lines ...", start_line)}</span>
                                    </div>
                                },
                            )
                        } else {
                            None
                        };
                        let visible_lines: Vec<_> = content
                            .lines()
                            .skip(start_line)
                            .take(end_line.saturating_sub(start_line))
                            .enumerate()
                            .map(|(rel_i, line)| {
                                let i = rel_i + start_line;
                                let is_current = i == current;
                                view! {
                                    <div class=if is_current {
                                        "gcode-line current-line"
                                    } else {
                                        "gcode-line"
                                    }>
                                        <span class="line-number">{format!("{:4}: ", i + 1)}</span>
                                        <span class="line-content">{highlight_gcode(line)}</span>
                                    </div>
                                }
                            })
                            .collect();
                        let after_placeholder = if end_line < total_lines {
                            Some(
                                view! {
                                    <div class="gcode-line-placeholder">
                                        <span>
                                            {format!("... {} more lines ...", total_lines - end_line)}
                                        </span>
                                    </div>
                                },
                            )
                        } else {
                            None
                        };
                        view! {
                            <div>{before_placeholder} {visible_lines} {after_placeholder}</div>
                        }
                    }}
                </pre>
            </Scrollbar>
        </div>
    }
}
