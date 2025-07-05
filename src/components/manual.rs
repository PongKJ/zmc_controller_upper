use crate::model::{ManualControl, Parameters};
use crate::{
    api::{
        zmc_converter_run, zmc_converter_set_freq, zmc_converter_stop, zmc_manual_move,
        zmc_manual_stop, zmc_set_zero,
    },
    app::GlobalState,
};
use leptos::{
    ev::MouseEvent, logging, prelude::*, reactive::spawn_local,
    server::codee::string::JsonSerdeCodec,
};
use leptos_use::use_cookie;
use thaw::*;

fn manual_move(axis: u8, direction: i8) {
    spawn_local(async move {
        logging::log!("Moving axis {} in direction {}", axis, direction);
        zmc_manual_move(axis, direction).await.unwrap();
    });
}
fn manual_stop(axis: u8) {
    spawn_local(async move {
        logging::log!("Stopping axis {}", axis);
        zmc_manual_stop(axis).await.unwrap();
    });
}

#[component]
fn ControlView() -> impl IntoView {
    let (global_state, set_global_state) =
        use_cookie::<GlobalState, JsonSerdeCodec>("global_state_cookie");
    // Ensure global state is initialized
    if global_state.read_untracked().is_none() {
        set_global_state.set(Some(GlobalState::default()));
    }
    let (parameters, set_parameters) =
        use_cookie::<Parameters, JsonSerdeCodec>("parameters_cookie");
    // Ensure parameters are initialized
    if parameters.read_untracked().is_none() {
        set_parameters.set(Some(Parameters::default()));
    }

    let connected = move || global_state.get().unwrap().connected;

    view! {
        <div class="manual-view-container">
            <div class="axis-control-container">
                <Button
                    disabled=Signal::derive(move || !connected())
                    on_click=move |_ev: MouseEvent| {
                        let params = parameters.get_untracked().expect("parameters should exist");
                        spawn_local(async move {
                            zmc_set_zero(
                                    vec![params.x.axis_num, params.y.axis_num, params.z.axis_num],
                                )
                                .await
                                .expect("Failed to set zero position");
                        });
                    }
                >
                    "坐标置零"
                </Button>
            </div>
            <div class="joystick-container">
                <Flex>
                    <Flex vertical=true>
                        <Flex justify=FlexJustify::Center>
                            <Button
                                disabled=Signal::derive(move || !connected())
                                icon=icondata::AiUpOutlined
                                on:mousedown=move |_| {
                                    manual_move(1, 1);
                                }
                                on:mouseup=move |_| {
                                    manual_stop(1);
                                }
                            />
                        </Flex>
                        <Flex justify=FlexJustify::Center>
                            <Button
                                disabled=Signal::derive(move || !connected())
                                icon=icondata::AiLeftOutlined
                                on:mousedown=move |_| {
                                    manual_move(0, -1);
                                }
                                on:mouseup=move |_| {
                                    manual_stop(0);
                                }
                            />
                            <div style="width: 30px;" />
                            <Button
                                disabled=Signal::derive(move || !connected())
                                icon=icondata::AiRightOutlined
                                on:mousedown=move |_| {
                                    manual_move(0, 1);
                                }
                                on:mouseup=move |_| {
                                    manual_stop(0);
                                }
                            />
                        </Flex>
                        <Flex justify=FlexJustify::Center>
                            <Button
                                disabled=Signal::derive(move || !connected())
                                icon=icondata::AiDownOutlined
                                on:mousedown=move |_| {
                                    manual_move(1, -1);
                                }
                                on:mouseup=move |_| {
                                    manual_stop(1);
                                }
                            />
                        </Flex>
                    </Flex>
                    <div style="width: 20px;" />
                    <Flex vertical=true justify=FlexJustify::Center>
                        <Button
                            disabled=Signal::derive(move || !connected())
                            icon=icondata::AiArrowUpOutlined
                            on:mousedown=move |_| {
                                manual_move(2, 1);
                            }
                            on:mouseup=move |_| {
                                manual_stop(2);
                            }
                        />
                        <div style="height: 10px;" />
                        <Button
                            disabled=Signal::derive(move || !connected())
                            icon=icondata::AiArrowDownOutlined
                            on:mousedown=move |_| {
                                manual_move(2, -1);
                            }
                            on:mouseup=move |_| {
                                manual_stop(2);
                            }
                        />
                    </Flex>
                </Flex>
            </div>
        </div>
    }
}

#[component]
fn ConverterControlView() -> impl IntoView {
    let (global_state, set_global_state) =
        use_cookie::<GlobalState, JsonSerdeCodec>("global_state_cookie");
    // Ensure global state is initialized
    if global_state.read_untracked().is_none() {
        set_global_state.set(Some(GlobalState::default()));
    }
    let connected = move || global_state.get().unwrap().connected;

    let (manual_control, set_manual_control) =
        use_cookie::<ManualControl, JsonSerdeCodec>("manual_control_cookie");
    // Ensure manual control is initialized
    if manual_control.read_untracked().is_none() {
        set_manual_control.set(Some(ManualControl::default()));
    }

    let frequency = RwSignal::new(
        manual_control
            .get_untracked()
            .unwrap_or_default()
            .converter_frequency
            .to_string(),
    );
    let inverted = RwSignal::new(
        manual_control
            .get_untracked()
            .unwrap_or_default()
            .converter_inverted,
    );
    let enabled = RwSignal::new(
        manual_control
            .get_untracked()
            .unwrap_or_default()
            .converter_enabled,
    );

    Effect::watch(
        move || (frequency.get().clone(), *inverted.read(), *enabled.read()),
        move |(f, i, e), _, _| {
            set_manual_control.update(|manual_control| {
                if manual_control.is_none() {
                    *manual_control = Some(ManualControl {
                        converter_frequency: f.parse().unwrap_or(0),
                        converter_inverted: *i,
                        converter_enabled: *e,
                        pos_store_x: 0.0,
                        pos_store_y: 0.0,
                    });
                } else {
                    let manual_control = manual_control
                        .as_mut()
                        .expect("ManualControl should not be None");
                    manual_control.converter_frequency = f.parse().unwrap_or(0);
                    manual_control.converter_inverted = *i;
                    manual_control.converter_enabled = *e;
                }
            });
        },
        false,
    );
    let on_control_click = move |_ev: MouseEvent| {
        let frequency_value = frequency.get().parse::<u32>().unwrap_or(0);
        let inverted_value = *inverted.read();
        let en = *enabled.read();
        spawn_local(async move {
            if en {
                logging::log!("Converter is already enabled, stopping it first.");
                match zmc_converter_stop().await {
                    Ok(_) => {
                        logging::log!("Converter stopped successfully.");
                        *enabled.write() = false;
                    }
                    Err(e) => {
                        logging::error!("Failed to stop converter: {}", e);
                        return;
                    }
                };
            } else {
                logging::log!(
                    "Starting converter with frequency: {}, inverted: {}",
                    frequency.read_untracked(),
                    inverted.read_untracked()
                );
                match zmc_converter_set_freq(frequency_value).await {
                    Ok(_) => {
                        logging::log!("Converter started successfully.");
                    }
                    Err(e) => {
                        logging::error!("Failed to start converter: {}", e);
                    }
                }
                match zmc_converter_run(inverted_value).await {
                    Ok(_) => {
                        logging::log!("Converter run command sent successfully.");
                        *enabled.write() = true;
                    }
                    Err(e) => {
                        logging::error!("Failed to run converter: {}", e);
                    }
                };
            }
        });
    };

    let enabled = move || manual_control.get().unwrap_or_default().converter_enabled;

    view! {
        <Input value=frequency input_type=InputType::Number placeholder="输入频率" />
        <Switch checked=inverted value="inverted" label="反转" />
        <Button
            disabled=Signal::derive(move || !connected())
            on_click=on_control_click
            appearance=Signal::derive(move || {
                if enabled() { ButtonAppearance::Primary } else { ButtonAppearance::Secondary }
            })
        >
            {move || { if enabled() { "停止" } else { "启动" } }}
        </Button>
    }
}

#[component]
pub fn ManualView() -> impl IntoView {
    view! {
        <Flex vertical=true>
            <ControlView />
            <ConverterControlView />
        </Flex>
    }
}
