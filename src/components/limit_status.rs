use leptos::{logging, prelude::*, reactive::spawn_local, server::codee::string::JsonSerdeCodec};
use leptos_use::
    use_cookie
;
use std::time::Duration;
use thaw::*;

use crate::{api::zmc_get_limit_status, app::GlobalState, components::Parameters};

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LimitStatus {
    emergency_stop: bool,
    door_switch: bool,
    x_plus: bool,
    x_minus: bool,
    y_plus: bool,
    y_minus: bool,
    z_plus: bool,
    z_minus: bool,
}
impl LimitStatus {
    pub fn new(
        emergency_stop: bool,
        door_switch: bool,
        x_plus: bool,
        x_minus: bool,
        y_plus: bool,
        y_minus: bool,
        z_plus: bool,
        z_minus: bool,
    ) -> Self {
        Self {
            emergency_stop,
            door_switch,
            x_plus,
            x_minus,
            y_plus,
            y_minus,
            z_plus,
            z_minus,
        }
    }
}

fn status_to_badge(status: bool) -> BadgeColor {
    if status {
        BadgeColor::Severe
    } else {
        BadgeColor::Success
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

    let (parameters, set_parameters) =
        use_cookie::<Parameters, JsonSerdeCodec>("parameters_cookie");
    // Ensure parameters are initialized
    if parameters.read_untracked().is_none() {
        set_parameters.set(Some(Parameters::default()));
    }

    let limit_status = RwSignal::new(LimitStatus::default());
    let mut interval_handle: Option<IntervalHandle> = None;
    Effect::watch(
        move || {
            (
                global_state.get().unwrap().connected,
                parameters.get().unwrap(),
            )
        },
        move |(c, p), _, _| {
            if *c {
                let p = p.clone();
                if interval_handle.is_some() {
                    logging::log!("Clearing previous interval handle.");
                    if let Some(handle) = interval_handle {
                        handle.clear();
                    }
                }
                interval_handle = Some(
                    set_interval_with_handle(
                        move || {
                            spawn_local(async move {
                                let status = zmc_get_limit_status(
                                    p.emergency_stop_io,
                                    p.door_switch_io,
                                    p.x.positive_limit_io,
                                    p.x.negative_limit_io,
                                    p.y.positive_limit_io,
                                    p.y.negative_limit_io,
                                    p.z.positive_limit_io,
                                    p.z.negative_limit_io,
                                )
                                .await;
                                if let Ok(status) = status {
                                    logging::log!("Limit status: {:?}", status);
                                    limit_status.set(status);
                                } else {
                                    logging::log!("Failed to fetch limit status.");
                                }
                            });
                        },
                        Duration::from_millis(1000),
                    )
                    .unwrap(),
                );
            } else {
                logging::log!("Not connected, clearing interval handle.");
                if let Some(handle) = interval_handle {
                    handle.clear();
                }
            }
        },
        true,
    );

    view! {
        <Transition fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            <div class="limit-status-container">
                {move || {
                    if !connected() {
                        view! {
                            "warning"
                            >
                            "Not connected to the machine."
                        }
                    } else {
                        let status = limit_status.get();
                        view! {
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
                        }
                    }
                }}
            </div>
        </Transition>
    }
}
