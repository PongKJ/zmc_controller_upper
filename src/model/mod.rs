// From client to post to the server
#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct ManualControl {
    pub converter_frequency: u16,
    pub converter_inverted: bool,
    pub converter_enabled: bool,
    // 对刀恢复坐标存储
    pub pos_store_x: f32,
    pub pos_store_y: f32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InvertedStatus {
    pub emergency_stop_level_inverted: bool,
    pub door_switch_level_inverted: bool,
    pub limit_io_level_inverted: bool,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AxisParameters {
    // 轴号
    pub axis_num: u8,
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

// From server to send to client by websocket
#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LimitStatus {
    pub emergency_stop: bool,
    pub door_switch: bool,
    pub x_plus: bool,
    pub x_minus: bool,
    pub y_plus: bool,
    pub y_minus: bool,
    pub z_plus: bool,
    pub z_minus: bool,
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
