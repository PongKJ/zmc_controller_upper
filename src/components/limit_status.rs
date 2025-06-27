use crate::{app::GlobalState, model::LimitStatus};
use leptos::{logging, prelude::*, server::codee::string::JsonSerdeCodec};
use leptos_use::use_cookie;
use thaw::*;

fn status_to_badge(status: bool) -> BadgeColor {
    if status {
        BadgeColor::Success
    } else {
        BadgeColor::Severe
    }
}

#[component]
pub fn LimitStatusView() -> impl IntoView {
    let (global_state, set_global_state) =
        use_cookie::<GlobalState, JsonSerdeCodec>("global_state_cookie");
    // Ensure global state is initialized
    if global_state.read_untracked().is_none() {
        set_global_state.set(Some(GlobalState::default()));
    }

    let connected = move || global_state.get().unwrap().connected;

    let limit_status =
        leptos_ws::ServerSignal::new("limit_status".to_string(), LimitStatus::default()).unwrap();

    view! {
        <Transition fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            <div class="limit-status-container">
                {move || {
                    if !connected() {
                        view! { <div class="not-connected-text">"Waitting for connected"</div> }
                    } else {
                        let status = limit_status.get();
                        view! {
                            <div class="status-badge">
                                <Badge color=Signal::derive(move || {
                                    status_to_badge(status.emergency_stop)
                                })>"急停"</Badge>
                                <Badge color=Signal::derive(move || {
                                    status_to_badge(status.door_switch)
                                })>"门限位"</Badge>
                                <Badge color=Signal::derive(move || {
                                    status_to_badge(status.x_plus)
                                })>"X+"</Badge>
                                <Badge color=Signal::derive(move || {
                                    status_to_badge(status.x_minus)
                                })>"X-"</Badge>
                                <Badge color=Signal::derive(move || {
                                    status_to_badge(status.y_plus)
                                })>"Y+"</Badge>
                                <Badge color=Signal::derive(move || {
                                    status_to_badge(status.y_minus)
                                })>"Y-"</Badge>
                                <Badge color=Signal::derive(move || {
                                    status_to_badge(status.z_plus)
                                })>"Z+"</Badge>
                                <Badge color=Signal::derive(move || {
                                    status_to_badge(status.z_minus)
                                })>"Z-"</Badge>
                            </div>
                        }
                    }
                }}
            </div>
        </Transition>
    }
}
