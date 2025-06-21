use leptos::prelude::*;

use crate::components::AxisMoveStatus;
use crate::components::LimitStatus;
use crate::components::MoveStatus;
use crate::components::Parameters;
#[cfg(feature = "ssr")]
pub use once_cell::sync::OnceCell;
#[cfg(feature = "ssr")]
use std::sync::Mutex;
use std::sync::MutexGuard;
#[cfg(feature = "ssr")]
use zmc_lib::ZmcController;
#[cfg(feature = "ssr")]
use zmc_lib::ZmcError;

#[cfg(feature = "ssr")]
pub static CONTROLLER: OnceCell<Mutex<ZmcController>> = OnceCell::new();

#[cfg(feature = "ssr")]
fn get_controller<'a>() -> Result<MutexGuard<'a, ZmcController>, ServerFnError> {
    CONTROLLER
        .get()
        .ok_or_else(|| ServerFnError::<ZmcError>::WrappedServerError(ZmcError::NotOpen))?
        .lock()
        .map_err(|e| e.into())
}

#[server]
pub async fn zmc_open_eth(ip: String) -> Result<(), ServerFnError> {
    if CONTROLLER.get().is_some() {
        if CONTROLLER
            .get()
            .expect("Controller should be initialized")
            .lock()?
            .is_open()
        {
            return Err(ServerFnError::ServerError(
                "Controller already opened".to_string(),
            ));
        } else {
            // If the controller is already initialized but not open, we can just close it
            // and reinitialize it with the new IP address.
            let mut controller = get_controller()?;
            controller.open_eth(ip.as_str())?
        }
    } else {
        // Create a new ZmcController instance
        let mut controller = ZmcController::new();
        controller
            .open_eth(ip.as_str())
            .map_err(|e| ServerFnError::<ZmcError>::WrappedServerError(e))?;
        CONTROLLER
            .set(Mutex::new(controller))
            .expect("Controller should not be initialized yet");
    }
    Ok(())
}

#[server]
pub async fn zmc_close() -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    Ok(controller.close()?)
}

// 获取限位信息
#[server]
pub async fn zmc_get_limit_status(
    emer_io: u16,
    door_switch_io: u16,
    x_plus_io: u16,
    x_minus_io: u16,
    y_plus_io: u16,
    y_minus_io: u16,
    z_plus_io: u16,
    z_minus_io: u16,
) -> Result<LimitStatus, ServerFnError> {
    let mut controller = get_controller()?;
    let emer = controller.direct_get_in(emer_io)?;
    let door_switch = controller.direct_get_in(door_switch_io)?;
    let x_plus = controller.direct_get_in(x_plus_io)?;
    let x_minus = controller.direct_get_in(x_minus_io)?;
    let y_plus = controller.direct_get_in(y_plus_io)?;
    let y_minus = controller.direct_get_in(y_minus_io)?;
    let z_plus = controller.direct_get_in(z_plus_io)?;
    let z_minus = controller.direct_get_in(z_minus_io)?;
    Ok(LimitStatus::new(
        emer,
        door_switch,
        x_plus,
        x_minus,
        y_plus,
        y_minus,
        z_plus,
        z_minus,
    ))
}

#[server]
pub async fn zmc_get_move_status(
    x_axis: u8,
    y_axis: u8,
    z_axis: u8,
) -> Result<MoveStatus, ServerFnError> {
    let mut controller = get_controller()?;
    let x_speed = controller.direct_get_m_speed(x_axis)?;
    let y_speed = controller.direct_get_m_speed(y_axis)?;
    let z_speed = controller.direct_get_m_speed(z_axis)?;
    let x_pos = controller.direct_get_d_pos(x_axis)?;
    let y_pos = controller.direct_get_d_pos(y_axis)?;
    let z_pos = controller.direct_get_d_pos(z_axis)?;
    let x_is_idle = controller.direct_get_if_idle(x_axis)?;
    let y_is_idle = controller.direct_get_if_idle(y_axis)?;
    let z_is_idle = controller.direct_get_if_idle(z_axis)?;
    Ok(MoveStatus {
        x: AxisMoveStatus {
            is_idle: x_is_idle,
            speed: x_speed,
            pos: x_pos,
        },
        y: AxisMoveStatus {
            is_idle: y_is_idle,
            speed: y_speed,
            pos: y_pos,
        },
        z: AxisMoveStatus {
            is_idle: z_is_idle,
            speed: z_speed,
            pos: z_pos,
        },
    })
}

// 设定参数
#[server]
pub async fn zmc_set_parameters(params: Parameters) -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    // 设置输入IO的电平反转
    controller.direct_set_invert_in(
        params.emergency_stop_io,
        params.inverted_status.emergency_stop_level_inverted,
    )?;
    controller.direct_set_invert_in(
        params.door_switch_io,
        params.inverted_status.door_switch_level_inverted,
    )?;
    let io_limit_list = [
        params.x.positive_limit_io,
        params.y.positive_limit_io,
        params.z.positive_limit_io,
        params.x.negative_limit_io,
        params.y.negative_limit_io,
        params.z.negative_limit_io,
    ];
    for io in io_limit_list {
        controller.direct_set_invert_in(io, params.inverted_status.limit_io_level_inverted)?;
    }

    let axis_params = [params.x, params.y, params.z];

    for i in 0..3 {
        // TODO: Change to 65 after simulation
        controller.direct_set_a_type(i, 65)?;
        controller.direct_set_speed(i, params.speed.processing_speed)?;
        // 设置初始速度为0
        controller.direct_set_l_speed(i, 0.0)?;
        // 设置加速度和减速度
        controller.direct_set_accel(i, params.speed.acceleration)?;
        controller.direct_set_decel(i, params.speed.deceleration)?;
        // 设置梯形速度
        controller.direct_set_sramp(i, 20.0)?;
        controller.direct_set_units(i, axis_params[i as usize].pulse_equivalent)?;
        // 设置软件正限位
        controller.direct_set_fs_limit(i, axis_params[i as usize].software_positive_limit)?;
        // 设置软件负限位
        controller.direct_set_rs_limit(i, axis_params[i as usize].software_negative_limit)?;
        // 设置硬件正限位IO
        controller.direct_set_fwd_in(i, axis_params[i as usize].positive_limit_io)?;
        // 设置硬件负限位IO
        controller.direct_set_Rev_in(i, axis_params[i as usize].negative_limit_io)?;
        // 设置回零开关IO
        controller.direct_set_datum_in(i, axis_params[i as usize].zero_point_io)?;
        controller.direct_set_alm_in(i, params.emergency_stop_io)?;
        // TODO: 设置PID参数
    }
    Ok(())
}

// 变频器运行
#[server]
pub async fn zmc_converter_run(freq: u32, inverted: bool) -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    controller.modbus_set4x_long(3, 1, &[freq as i32])?;
    controller.direct_command("MODBUSM_REGSET(100,1,3)")?;
    if inverted {
        controller.direct_command("MODBUSM_REGSET(99,1,0)")?;
    } else {
        controller.direct_command("MODBUSM_REGSET(99,1,2)")?;
    }
    Ok(())
}

// 变频器停止
#[server]
pub async fn zmc_converter_stop() -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    controller.execute_noack("MODBUSM_REGSET(99,1,1)")?;
    Ok(())
}

// 设置输入轴的电平反转
#[server]
pub async fn zmc_set_in_inverted(in_num: u16, inverted: bool) -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    controller.direct_set_invert_in(in_num, inverted)?;
    Ok(())
}

// 手动移动轴,输入轴和运动的正负，
#[server]
pub async fn zmc_manual_move(axis: u8, direction: i8) -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    controller.direct_single_v_move(axis, direction)?;
    Ok(())
}

// 手动停止轴
#[server]
pub async fn zmc_manual_stop(axis: u8) -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    controller.direct_single_cancel(axis, 2)?;
    Ok(())
}

// 获取当前轴位置
#[server]
pub async fn zmc_get_axis_position(axis: u8) -> Result<f32, ServerFnError> {
    let mut controller = get_controller()?;
    let position = controller.direct_get_d_pos(axis)?;
    Ok(position)
}

//  寻找零点
#[server]
pub async fn zmc_datum(axis: u8) -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    // 获取当前轴的正负
    let pos = controller.direct_get_d_pos(axis)?;
    if pos > 0.0 {
        controller.direct_single_v_move(axis, 19)?;
    } else {
        controller.direct_single_v_move(axis, 18)?;
    }
    Ok(())
}

// 所有轴坐标清零
#[server]
pub async fn zmc_set_zero() -> Result<(), ServerFnError> {
    let mut controller = get_controller()?;
    controller.execute_noack("MPOS=0,0,0")?;
    controller.execute_noack("DPOS=0,0,0")?;
    Ok(())
}
