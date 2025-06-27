use crate::model::Parameters;
use crate::{app::GlobalState, model::LimitStatus};
use leptos::{logging, prelude::*, server::codee::string::JsonSerdeCodec};
use leptos::{
    task::spawn_local,
    wasm_bindgen::{prelude::Closure, JsCast},
    *,
};
use leptos_use::use_cookie;
use thaw::*;
use web_sys::{HtmlElement, MouseEvent, ScrollToOptions};

use crate::api::{
    debug_update_line, generate_path_preview, load_gcode, start_gcode_execution,
    stop_gcode_execution, zmc_init_eth, zmc_init_fake,
};

// Simple G-code syntax highlighting
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
    // Simple regex-like parsing for G-code commands and parameters
    let mut result = Vec::new();
    let mut current_pos = 0;

    for (i, c) in code.char_indices() {
        if i == 0 && (c == 'G' || c == 'M' || c == 'T') {
            // Find the end of the command number
            if let Some(end_pos) = code[1..].find(|c: char| !c.is_digit(10)).map(|p| p + 1) {
                result.push(view! { <span class="command">{&code[0..end_pos]}</span> });
                current_pos = end_pos;
            }
        } else if current_pos <= i && c.is_alphabetic() && i + 1 < code.len() {
            // Parameter (like X100 Y200)
            let param_start = i;
            let mut param_end = i + 1;

            // Find the parameter value end
            while param_end < code.len()
                && (code[param_end..].chars().next().unwrap().is_digit(10)
                    || code[param_end..].chars().next().unwrap() == '.'
                    || code[param_end..].chars().next().unwrap() == '-')
            {
                param_end += 1;
            }

            if param_end > param_start + 1 {
                result
                    .push(view! { <span class="parameter">{&code[param_start..param_end]}</span> });
                current_pos = param_end;
            }
        }
    }

    // Add any remaining text
    if current_pos < code.len() {
        result.push(view! { <span class="remaining-text">{&code[current_pos..]}</span> });
    }

    view! { <div>{result}</div> }
}

#[component]
pub fn AutoModeView() -> impl IntoView {
    let file_content = RwSignal::new(String::new());
    let current_line_ws =
        leptos_ws::ServerSignal::new("current_line".to_string(), 0 as usize).unwrap();

    let (ip_addr, set_ip_addr) = use_cookie::<String, JsonSerdeCodec>("ip_addr_cookie");
    // Ensure global state is initialized
    if ip_addr.read_untracked().is_none() {
        set_ip_addr.set(Some(String::new()));
    }

    let processing_line = RwSignal::new(0usize);
    Effect::watch(
        move || current_line_ws.get(),
        move |l, _, _| {
            logging::log!("Current line: {}", l);
            processing_line.set(*l);
        },
        false,
    );

    let is_preview = RwSignal::new(false);

    Effect::watch(
        move || (is_preview.get(), ip_addr.get_untracked().unwrap()),
        move |(preview, ip), _, _| {
            logging::log!("Preview mode: {}", preview);
            let preview = preview.clone();
            let ip = ip.clone();
            spawn_local(async move {
                if preview {
                    zmc_init_fake()
                        .await
                        .expect("Failed to initialize fake controller");
                } else {
                    zmc_init_eth(ip)
                        .await
                        .expect("Failed to initialize Ethernet controller");
                }
            });
            // Here you can add logic to handle preview mode changes
        },
        false,
    );

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
                            logging::log!("File content: {}", text);
                            file_content.set(text.clone());
                            spawn_local(async move {
                                load_gcode(text).await.expect("Failed to load G-code");
                            });
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

    let on_start_click = move |_: MouseEvent| {
        spawn_local(async move {
            start_gcode_execution()
                .await
                .expect("Failed to start G-code execution");
        });
    };
    let on_stop_click = move |_: MouseEvent| {
        spawn_local(async move {
            stop_gcode_execution()
                .await
                .expect("Failed to stop G-code execution");
        });
    };

    let on_debug_click = move |_: MouseEvent| {
        spawn_local(async move {
            logging::log!("Testing WebSocket connection...");
            debug_update_line()
                .await
                .expect("Failed to send debug update");
        });
    };

    let on_genenrate_preview_click = move |_: MouseEvent| {
        spawn_local(async move {
            generate_path_preview()
                .await
                .expect("Failed to generate path preview");
        });
    };

    let scrollbar_ref = ComponentRef::<ScrollbarRef>::new();
    Effect::new(move |_| {
        // Watch for changes to processing_line
        let current_line = processing_line.get();

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

    view! {
        <Button on_click=on_genenrate_preview_click>"Debug"</Button>
        <div class="status-container">
            <ProgressCircle
                value=Signal::derive(move || {
                    let total_lines = file_content.get().lines().count() as f64;
                    if total_lines == 0.0 {
                        100.0
                    } else {
                        ((processing_line.get() as f64 / total_lines) * 100.0 * 10.0).round() / 10.0
                    }
                })
                color=ProgressCircleColor::Success
            />
        </div>
        <div class="control-container">
            <Upload custom_request>
                <Button>"upload"</Button>
            </Upload>
            <Button on_click=on_start_click>"Start"</Button>
            <Button on_click=on_stop_click>"Stop"</Button>
            <Switch checked=is_preview label="Preview Mode" />
        </div>
        <div class="file-content">
            <p>"G-code Preview:"</p>
            <Scrollbar
                style="max-height: 300px;"
                class="gcode-scrollbar"
                // Add the node reference here
                comp_ref=scrollbar_ref
            >
                <pre style="text-align: left;" class="gcode-display">
                    {move || {
                        let content = file_content.get();
                        let current = processing_line.get();
                        let lines: Vec<_> = content
                            .lines()
                            .enumerate()
                            .map(|(i, line)| {
                                let is_current = i == current;

                                // 将G代码文本转换为带有高亮的HTML内容
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

                        view! { <div>{lines}</div> }
                    }}
                </pre>
            </Scrollbar>
        </div>
    }
}
