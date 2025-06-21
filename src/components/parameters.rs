use leptos::logging::{self, log};
use leptos::prelude::*;
use leptos::server::codee::string::JsonSerdeCodec;
use leptos::{ev::MouseEvent, reactive::spawn_local};
use leptos_use::use_cookie;
use thaw::ssr::SSRMountStyleProvider;
use thaw::*;

use crate::api::{zmc_close, zmc_set_in_inverted, zmc_set_parameters};
use crate::{api::zmc_open_eth, app::GlobalState};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InvertedStatus {
    pub emergency_stop_level_inverted: bool,
    pub door_switch_level_inverted: bool,
    pub limit_io_level_inverted: bool,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AxisParameters {
    // 脉冲当量
    pub pulse_equivalent: f32,
    // 软件正限位
    pub software_positive_limit: f32,
    // 软件负限位
    pub software_negative_limit: f32,
    // 限位IO设置参数
    // 正限位IO
    pub positive_limit_io: u16,
    // 负限位IO
    pub negative_limit_io: u16,
    // 零点IO
    pub zero_point_io: u16,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PidParameters {
    pub p: f32,
    pub i: f32,
    pub d: f32,
}
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SpeedParameters {
    // 加工速度
    pub processing_speed: f32,
    // 最大速度
    pub max_speed: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    // 过渡时间
    pub transition_time: f32,
    // 爬行速度
    pub crawling_speed: f32,
}
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Parameters {
    pub ip: String,
    pub pid: PidParameters,
    pub x: AxisParameters,
    pub y: AxisParameters,
    pub z: AxisParameters,
    // 急停IO
    pub emergency_stop_io: u16,
    pub speed: SpeedParameters,
    // 门限位IO
    pub door_switch_io: u16,
    pub inverted_status: InvertedStatus,
}

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
            let p = p.clone();
            spawn_local(async move {
                zmc_set_parameters(p)
                    .await
                    .expect("Failed to set parameters");
            });
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
    let (parameters, set_parameters) =
        use_cookie::<Parameters, JsonSerdeCodec>("parameters_cookie");
    // Ensure parameters are initialized
    if parameters.read_untracked().is_none() {
        set_parameters.set(Some(Parameters::default()));
    }

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

    // Shit code :(
    // signals to bind to input fields
    let parameters = parameters.get_untracked().unwrap();
    let v_p = RwSignal::new(parameters.pid.p.to_string());
    let v_i = RwSignal::new(parameters.pid.i.to_string());
    let v_d = RwSignal::new(parameters.pid.d.to_string());

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
            params.pid.p = v_p.get().parse().unwrap_or(0.0);
            params.pid.i = v_i.get().parse().unwrap_or(0.0);
            params.pid.d = v_d.get().parse().unwrap_or(0.0);
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
            params.emergency_stop_io = v_emergency_stop_io.get().parse().unwrap_or(0);
            params.door_switch_io = v_door_switch_io.get().parse().unwrap_or(0);
            params.inverted_status.emergency_stop_level_inverted =
                v_emergency_stop_level_inverted.get();
            params.inverted_status.door_switch_level_inverted = v_door_switch_level_inverted.get();
            params.inverted_status.limit_io_level_inverted = v_limit_io_level_inverted.get();
        });
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
                        <TableCell>"脉冲当量"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_pulse_equivalent_x
                                placeholder="float"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_pulse_equivalent_y
                                placeholder="float"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_pulse_equivalent_z
                                placeholder="float"
                                input_type=InputType::Number
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
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_positive_limit_io_y
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_positive_limit_io_z
                                placeholder="int"
                                input_type=InputType::Number
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
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_negative_limit_io_y
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_negative_limit_io_z
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"零点IO"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_zero_point_io_x
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_zero_point_io_y
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_zero_point_io_z
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>"软件正限位"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_positive_limit_x
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_positive_limit_y
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_positive_limit_z
                                placeholder="int"
                                input_type=InputType::Number
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
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_negative_limit_y
                                placeholder="int"
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_software_negative_limit_z
                                placeholder="int"
                                input_type=InputType::Number
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
                                input_type=InputType::Number
                            />
                        </TableCell>
                        <TableCell>"门限位IO"</TableCell>
                        <TableCell>
                            <Input
                                class="limit-input"
                                value=v_door_switch_io
                                placeholder="int"
                                input_type=InputType::Number
                            />
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

    let connected = move || global_state.get().unwrap().connected;

    let (parameters, set_parameters) =
        use_cookie::<Parameters, JsonSerdeCodec>("parameters_cookie");
    // Ensure parameters are initialized
    if parameters.read_untracked().is_none() {
        set_parameters.set(Some(Parameters::default()));
    }

    let toaster = ToasterInjection::expect_context();

    let v_ip = RwSignal::new(parameters.get_untracked().unwrap().ip);

    Effect::watch(
        move || v_ip.get(),
        move |ip, _, _| {
            set_parameters.update(|params| {
                params.as_mut().unwrap().ip = ip.trim().to_string();
            });
            log!("IP updated: {}", ip);
        },
        false,
    );

    let on_connect_click = move |e: MouseEvent| {
        if !connected() {
            let ip = v_ip.get().trim().to_string();
            log!("Connecting to IP: {}", ip);
            spawn_local(async move {
                match zmc_open_eth(ip).await {
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
