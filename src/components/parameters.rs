use crate::model::Parameters;
use leptos::logging::{self, log};
use leptos::prelude::*;
use leptos::server::codee::string::JsonSerdeCodec;
use leptos::{ev::MouseEvent, reactive::spawn_local};
use leptos_use::use_cookie;
use thaw::ssr::SSRMountStyleProvider;
use thaw::*;

use crate::api::{zmc_close, zmc_set_parameters};
use crate::{api::zmc_init_eth, app::GlobalState};

#[component]
pub fn ParametersView() -> impl IntoView {
    let (parameters, set_parameters) =
        use_cookie::<Parameters, JsonSerdeCodec>("parameters_cookie");
    // Ensure parameters are initialized
    if parameters.read_untracked().is_none() {
        set_parameters.set(Some(Parameters::default()));
    }

    Effect::watch(
        move || parameters.get().unwrap(),
        move |p, _, _| {
            log!("Parameters updated: {:?}", p);
        },
        false,
    );

    view! {
        <SSRMountStyleProvider>
            <div class="parameters">
                // Ip input field
                <div class="input-container">
                    <div class="connection-container">
                        <ConnectionInput />
                    </div>
                    <div class="parameter-container">
                        <ParametersInput />
                    </div>
                </div>
            </div>
        </SSRMountStyleProvider>
    }
}

#[component]
fn ParametersInput() -> impl IntoView {
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

    let allow_float = |value: String| {
        // Allow only digits and a single decimal point
        value
            .chars()
            .all(|c| c.is_digit(10) || c == '.' || c == 'e' || c == '-')
    };
    let allow_integer = |value: String| {
        // Allow only digits and an optional leading minus sign
        value.chars().all(|c| c.is_digit(10) || c == '-')
    };
    let allow_io_integer = |value: String| {
        // Allow only digits and an optional leading minus sign
        value.chars().all(|c| c.is_digit(10))
    };

    let parameters_tracked = move || parameters.get().unwrap();

    let parameters = parameters.get_untracked().unwrap();
    // Shit code :(
    // signals to bind to input fields
    let v_p = RwSignal::new(parameters.pid.p.to_string());
    let v_i = RwSignal::new(parameters.pid.i.to_string());
    let v_d = RwSignal::new(parameters.pid.d.to_string());

    let v_x_axis_num = RwSignal::new(parameters.x.axis_num.to_string());
    let v_y_axis_num = RwSignal::new(parameters.y.axis_num.to_string());
    let v_z_axis_num = RwSignal::new(parameters.z.axis_num.to_string());

    let v_pulse_equivalent_x = RwSignal::new(parameters.x.pulse_equivalent.to_string());
    let v_pulse_equivalent_y = RwSignal::new(parameters.y.pulse_equivalent.to_string());
    let v_pulse_equivalent_z = RwSignal::new(parameters.z.pulse_equivalent.to_string());

    let v_positive_limit_io_x = RwSignal::new(parameters.x.positive_limit_io.to_string());
    let v_negative_limit_io_x = RwSignal::new(parameters.x.negative_limit_io.to_string());
    let v_zero_point_io_x = RwSignal::new(parameters.x.zero_point_io.to_string());
    let v_software_positive_limit_x =
        RwSignal::new(parameters.x.software_positive_limit.to_string());
    let v_software_negative_limit_x =
        RwSignal::new(parameters.x.software_negative_limit.to_string());
    let v_positive_limit_io_y = RwSignal::new(parameters.y.positive_limit_io.to_string());
    let v_negative_limit_io_y = RwSignal::new(parameters.y.negative_limit_io.to_string());
    let v_zero_point_io_y = RwSignal::new(parameters.y.zero_point_io.to_string());
    let v_software_positive_limit_y =
        RwSignal::new(parameters.y.software_positive_limit.to_string());
    let v_software_negative_limit_y =
        RwSignal::new(parameters.y.software_negative_limit.to_string());
    let v_positive_limit_io_z = RwSignal::new(parameters.z.positive_limit_io.to_string());
    let v_negative_limit_io_z = RwSignal::new(parameters.z.negative_limit_io.to_string());
    let v_zero_point_io_z = RwSignal::new(parameters.z.zero_point_io.to_string());
    let v_software_positive_limit_z =
        RwSignal::new(parameters.z.software_positive_limit.to_string());
    let v_software_negative_limit_z =
        RwSignal::new(parameters.z.software_negative_limit.to_string());

    let v_processing_speed = RwSignal::new(parameters.speed.processing_speed.to_string());
    let v_max_speed = RwSignal::new(parameters.speed.max_speed.to_string());
    let v_acceleration = RwSignal::new(parameters.speed.acceleration.to_string());
    let v_deceleration = RwSignal::new(parameters.speed.deceleration.to_string());
    let v_transition_time = RwSignal::new(parameters.speed.transition_time.to_string());
    let v_crawling_speed = RwSignal::new(parameters.speed.crawling_speed.to_string());

    let v_emergency_stop_io = RwSignal::new(parameters.emergency_stop_io.to_string());
    let v_door_switch_io = RwSignal::new(parameters.door_switch_io.to_string());

    let v_emergency_stop_level_inverted =
        RwSignal::new(parameters.inverted_status.emergency_stop_level_inverted);
    let v_door_switch_level_inverted =
        RwSignal::new(parameters.inverted_status.door_switch_level_inverted);
    let v_limit_io_level_inverted =
        RwSignal::new(parameters.inverted_status.limit_io_level_inverted);

    let on_save_click = move |_| {
        // Validate and parse the PID parameters
        set_parameters.update(|params| {
            let params = params.as_mut().expect("Parameters should not be None");
            params.pid.p = v_p.get().parse().unwrap_or(0.5);
            params.pid.i = v_i.get().parse().unwrap_or(0.5);
            params.pid.d = v_d.get().parse().unwrap_or(0.5);
            params.x.axis_num = v_x_axis_num.get().parse().unwrap_or(0);
            params.y.axis_num = v_y_axis_num.get().parse().unwrap_or(1);
            params.z.axis_num = v_z_axis_num.get().parse().unwrap_or(2);
            params.x.pulse_equivalent = v_pulse_equivalent_x.get().parse().unwrap_or(0.0);
            params.y.pulse_equivalent = v_pulse_equivalent_y.get().parse().unwrap_or(0.0);
            params.z.pulse_equivalent = v_pulse_equivalent_z.get().parse().unwrap_or(0.0);
            params.x.positive_limit_io = v_positive_limit_io_x.get().parse().unwrap_or(0);
            params.x.negative_limit_io = v_negative_limit_io_x.get().parse().unwrap_or(0);
            params.x.zero_point_io = v_zero_point_io_x.get().parse().unwrap_or(0);
            params.x.software_positive_limit =
                v_software_positive_limit_x.get().parse().unwrap_or(0.0);
            params.x.software_negative_limit =
                v_software_negative_limit_x.get().parse().unwrap_or(0.0);
            params.y.positive_limit_io = v_positive_limit_io_y.get().parse().unwrap_or(0);
            params.y.negative_limit_io = v_negative_limit_io_y.get().parse().unwrap_or(0);
            params.y.zero_point_io = v_zero_point_io_y.get().parse().unwrap_or(0);
            params.y.software_positive_limit =
                v_software_positive_limit_y.get().parse().unwrap_or(0.0);
            params.y.software_negative_limit =
                v_software_negative_limit_y.get().parse().unwrap_or(0.0);
            params.z.positive_limit_io = v_positive_limit_io_z.get().parse().unwrap_or(0);
            params.z.negative_limit_io = v_negative_limit_io_z.get().parse().unwrap_or(0);
            params.z.zero_point_io = v_zero_point_io_z.get().parse().unwrap_or(0);
            params.z.software_positive_limit =
                v_software_positive_limit_z.get().parse().unwrap_or(0.0);
            params.z.software_negative_limit =
                v_software_negative_limit_z.get().parse().unwrap_or(0.0);
            params.speed.processing_speed = v_processing_speed.get().parse().unwrap_or(0.0);
            params.speed.max_speed = v_max_speed.get().parse().unwrap_or(0.0);
            params.speed.acceleration = v_acceleration.get().parse().unwrap_or(0.0);
            params.speed.deceleration = v_deceleration.get().parse().unwrap_or(0.0);
            params.speed.transition_time = v_transition_time.get().parse().unwrap_or(0.0);
            params.speed.crawling_speed = v_crawling_speed.get().parse().unwrap_or(0.0);
            params.emergency_stop_io = v_emergency_stop_io.get().parse().unwrap_or(0);
            params.door_switch_io = v_door_switch_io.get().parse().unwrap_or(0);
            params.inverted_status.emergency_stop_level_inverted =
                v_emergency_stop_level_inverted.get();
            params.inverted_status.door_switch_level_inverted = v_door_switch_level_inverted.get();
            params.inverted_status.limit_io_level_inverted = v_limit_io_level_inverted.get();
        });
        if connected() {
            let p = parameters_tracked();
            spawn_local(async move {
                zmc_set_parameters(p)
                    .await
                    .expect("Failed to set parameters");
            });
        }
        log!("Parameters saved");
    };

    view! {
        // <div class="pid-inputs">
        // <Table>
        // <TableHeader>
        // <TableRow>
        // <TableCell>P</TableCell>
        // <TableCell>I</TableCell>
        // <TableCell>D</TableCell>
        // </TableRow>
        // </TableHeader>
        // <TableRow>
        // <TableCell>
        // <Input
        // class="pid-input"
        // value=v_p
        // placeholder="P"
        // input_type=InputType::Number
        // />
        // </TableCell>
        // <TableCell>
        // <Input
        // class="pid-input"
        // value=v_i
        // placeholder="I"
        // input_type=InputType::Number
        // />
        // </TableCell>
        // <TableCell>
        // <Input
        // class="pid-input"
        // value=v_d
        // placeholder="D"
        // input_type=InputType::Number
        // />
        // </TableCell>
        // </TableRow>
        // </Table>
        // </div>
        <div class="axis-parametets">
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableCell>value</TableCell>
                        <TableCell>X</TableCell>
                        <TableCell>Y</TableCell>
                        <TableCell>Z</TableCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    <TableRow>
                        <TableCell>"轴号"</TableCell>
                        <TableCell>
                            <Input class="axis-input" value=v_x_axis_num placeholder="float" />
                        </TableCell>
                        <TableCell>
                            <Input class="axis-input" value=v_y_axis_num placeholder="float" />
                        </TableCell>
                        <TableCell>
                            <Input class="axis-input" value=v_z_axis_num placeholder="float" />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"脉冲当量"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_pulse_equivalent_x
                                placeholder="float"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_pulse_equivalent_y
                                placeholder="float"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_pulse_equivalent_z
                                placeholder="float"
                            />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"正限位IO"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_positive_limit_io_x
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_positive_limit_io_y
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_positive_limit_io_z
                                placeholder="int"
                            />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"负限位IO"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_negative_limit_io_x
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_negative_limit_io_y
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_negative_limit_io_z
                                placeholder="int"
                            />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"零点IO"</TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_zero_point_io_x placeholder="int" />
                        </TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_zero_point_io_y placeholder="int" />
                        </TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_zero_point_io_z placeholder="int" />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"软件正限位"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_positive_limit_x
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_positive_limit_y
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_positive_limit_z
                                placeholder="int"
                            />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"软件负限位"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_negative_limit_x
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_negative_limit_y
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_negative_limit_z
                                placeholder="int"
                            />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"急停IO"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_emergency_stop_io
                                placeholder="int"
                            />
                        </TableCell>
                        <TableCell>"门限位IO"</TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_door_switch_io placeholder="int" />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"加工速度"</TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_processing_speed placeholder="int" />
                        </TableCell>
                        <TableCell>"最大速度"</TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_max_speed placeholder="int" />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"加速度"</TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_acceleration placeholder="int" />
                        </TableCell>
                        <TableCell>"减速度"</TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_deceleration placeholder="int" />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"过渡时间"</TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_transition_time placeholder="int" />
                        </TableCell>
                        <TableCell>"爬行速度"</TableCell>
                        <TableCell>
                            <Input class="limit-input" value=v_crawling_speed placeholder="int" />
                        </TableCell>
                    </TableRow>
                </TableBody>
            </Table>
        </div>
        <div class="inverted-status">
            <Switch
                checked=v_emergency_stop_level_inverted
                value="emergency_stop_level_inverted"
                label="急停反向"
            />
            <Switch
                checked=v_door_switch_level_inverted
                value="emergency_stop_level_inverted"
                label="门限位反向"
            />
            <Switch
                checked=v_limit_io_level_inverted
                value="emergency_stop_level_inverted"
                label="限位IO反向"
            />
        </div>
        <Button class="save-button" on_click=on_save_click>
            "Save"
        </Button>
    }
}

#[component]
fn ConnectionInput() -> impl IntoView {
    let (global_state, set_global_state) =
        use_cookie::<GlobalState, JsonSerdeCodec>("global_state_cookie");
    // Ensure global state is initialized
    if global_state.read_untracked().is_none() {
        set_global_state.set(Some(GlobalState::default()));
    }

    let (ip_addr, set_ip_addr) = use_cookie::<String, JsonSerdeCodec>("ip_addr_cookie");
    // Ensure global state is initialized
    if ip_addr.read_untracked().is_none() {
        set_ip_addr.set(Some(String::new()));
    }

    let connected = move || global_state.get().unwrap().connected;

    let (parameters, set_parameters) =
        use_cookie::<Parameters, JsonSerdeCodec>("parameters_cookie");
    // Ensure parameters are initialized
    if parameters.read_untracked().is_none() {
        set_parameters.set(Some(Parameters::default()));
    }

    let toaster = ToasterInjection::expect_context();

    let v_ip = RwSignal::new(ip_addr.get_untracked().unwrap());

    Effect::watch(
        move || v_ip.get(),
        move |ip, _, _| {
            set_ip_addr.set(Some(ip.to_string()));
            log!("IP updated: {}", ip);
        },
        false,
    );

    let on_connect_click = move |e: MouseEvent| {
        if !connected() {
            let ip = v_ip.get().trim().to_string();
            log!("Connecting to IP: {}", ip);
            spawn_local(async move {
                match zmc_init_eth(ip).await {
                    Ok(_) => {
                        log!("Connected successfully");
                        set_global_state.update(|state| {
                            state.as_mut().unwrap().connected = true;
                        });
                    }
                    Err(e) => {
                        log!("Failed to connect: {:?}", e);
                        // Handle the error, e.g., show a notification or alert
                        // For example, you could use a toast notification here
                        toaster.dispatch_toast(
                            move || {
                                view! {
                                    <Toast>
                                        <ToastTitle>"Connection"</ToastTitle>
                                        <ToastBody>
                                            "Connecting failed"
                                            <ToastBodySubtitle slot>"Subtitle"</ToastBodySubtitle>
                                        </ToastBody>
                                        <ToastFooter>"Footer"</ToastFooter>
                                    </Toast>
                                }
                            },
                            Default::default(),
                        );
                    }
                }
            });
        } else {
            spawn_local(async move {
                log!("Disconnecting...");
                match zmc_close().await {
                    Ok(_) => {
                        log!("Disconnected successfully");
                    }
                    Err(e) => {
                        log!("Failed to disconnect: {:?}", e);
                        // Handle the error, e.g., show a notification or alert
                        toaster.dispatch_toast(
                            move || {
                                view! {
                                    <Toast>
                                        <ToastTitle>"Disconnection"</ToastTitle>
                                        <ToastBody>
                                            "Disconnecting failed"
                                            <ToastBodySubtitle slot>"Subtitle"</ToastBodySubtitle>
                                        </ToastBody>
                                        <ToastFooter>"Footer"</ToastFooter>
                                    </Toast>
                                }
                            },
                            Default::default(),
                        );
                    }
                }
                set_global_state.update(|state| {
                    state.as_mut().unwrap().connected = false;
                });
            });
        }
    };
    view! {
        <Input value=v_ip name="ip" class="ip-input" placeholder="Enter IP address" />
        <Button
            on_click=on_connect_click
            appearance=Signal::derive(move || {
                if connected() { ButtonAppearance::Primary } else { ButtonAppearance::Secondary }
            })
        >
            {move || { if connected() { "Disconnect" } else { "Connect" } }}
        </Button>
    }
}
