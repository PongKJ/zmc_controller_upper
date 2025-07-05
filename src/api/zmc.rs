use leptos::prelude::*;
use leptos_ws::ServerSignal;

use crate::model::AxisMoveStatus;
use crate::model::LimitStatus;
use crate::model::MoveStatus;
use crate::model::Parameters;

#[cfg(feature = "ssr")]
use crate::utils::Bitmap;
#[cfg(feature = "ssr")]
use std::sync::Arc;
#[cfg(feature = "ssr")]
use std::sync::LazyLock;
use std::time::Duration;
#[cfg(feature = "ssr")]
use tokio::sync::Mutex;
#[cfg(feature = "ssr")]
use tokio::task::JoinSet;
#[cfg(feature = "ssr")]
use zmc_lib::{Controller, ControllerError, FakeController, ZmcController};

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ControllerType {
    Zmc(String), // String is the IP address for ZMC controller
    Fake,
}

#[cfg(feature = "ssr")]
pub struct ZmcManager {
    controller: Arc<Mutex<Option<Box<dyn Controller + Send>>>>,
    parameters: Arc<Mutex<Parameters>>,

    polling_interval: Arc<Mutex<Duration>>,
    polling_tasks: Arc<Mutex<JoinSet<Result<(), ServerFnError>>>>,
    limit_status: ServerSignal<LimitStatus>,
    move_status: Arc<Mutex<MoveStatus>>,
    move_status_signal: ServerSignal<MoveStatus>,
    // For drawing the movement path
    path_img_update_counter: Arc<Mutex<u32>>,
    bitmap: Arc<Mutex<Bitmap>>, // 500x500 bitmap with scale 10.0
    path_img: ServerSignal<String>,
}

#[cfg(feature = "ssr")]
async fn update_limit_status(
    controller: &mut Box<dyn Controller + Send>,
    params: &Parameters,
    limit_status: &mut ServerSignal<LimitStatus>,
) -> Result<(), ControllerError> {
    let emer = controller.direct_get_in(params.emergency_stop_io)?;
    let door_switch = controller.direct_get_in(params.door_switch_io)?;
    let x_plus = controller.direct_get_in(params.x.positive_limit_io)?;
    let x_minus = controller.direct_get_in(params.x.negative_limit_io)?;
    let y_plus = controller.direct_get_in(params.y.positive_limit_io)?;
    let y_minus = controller.direct_get_in(params.y.negative_limit_io)?;
    let z_plus = controller.direct_get_in(params.z.positive_limit_io)?;
    let z_minus = controller.direct_get_in(params.z.negative_limit_io)?;
    // HACK: Should not use set() to update here, or it will cause the signal not to track changes
    // Maybe it is a bug in leptos_ws ?
    limit_status.update(|status| {
        *status = LimitStatus::new(
            emer,
            door_switch,
            x_plus,
            x_minus,
            y_plus,
            y_minus,
            z_plus,
            z_minus,
        );
    });
    Ok(())
}

#[cfg(feature = "ssr")]
async fn update_move_status(
    controller: &mut Box<dyn Controller + Send>,
    params: &Parameters,
    move_status: &mut MoveStatus,
    bitmap: &mut Bitmap,
) -> Result<(), ControllerError> {
    let x_axis = params.x.axis_num;
    let y_axis = params.y.axis_num;
    let z_axis = params.z.axis_num;

    let x_pos = controller.direct_get_m_pos(x_axis)?;
    let y_pos = controller.direct_get_m_pos(y_axis)?;
    let z_pos = controller.direct_get_m_pos(z_axis)?;
    move_status.x.speed = controller.direct_get_m_speed(x_axis)?;
    move_status.y.speed = controller.direct_get_m_speed(y_axis)?;
    move_status.z.speed = controller.direct_get_m_speed(z_axis)?;
    move_status.x.pos = x_pos;
    move_status.y.pos = y_pos;
    move_status.z.pos = z_pos;
    move_status.x.is_idle = controller.direct_get_if_idle(x_axis)?;
    move_status.y.is_idle = controller.direct_get_if_idle(y_axis)?;
    move_status.z.is_idle = controller.direct_get_if_idle(z_axis)?;
    // Update the SVG path for visualization
    // 80x80 to 500x500 bitmap with scale 10.0
    bitmap.set_pixel(x_pos, y_pos, (-z_pos) * 75.0);
    Ok(())
}

const MOVE_STATUS_UPDATE_INTERVAL: u32 = 5; // Update every 50ms
const UPDATE_COUNT: u32 = 100 / MOVE_STATUS_UPDATE_INTERVAL; // Update every 100ms
#[cfg(feature = "ssr")]
impl ZmcManager {
    pub async fn start_polling(&self) -> Result<(), ServerFnError> {
        let controller = self.controller.clone();
        let parameters = self.parameters.clone();
        let mut limit_status = self.limit_status.clone();
        let move_status = self.move_status.clone();
        let move_status_signal = self.move_status_signal.clone();
        let path_img = self.path_img.clone();
        let bitmap = self.bitmap.clone();
        let counter = self.path_img_update_counter.clone();

        self.polling_tasks.lock().await.spawn(async move {
            loop {
                {
                    let mut controller = controller.lock().await;
                    if controller.is_none() {
                        return Err(ServerFnError::ServerError(
                            "Controller is not initialized".to_string(),
                        ));
                    }
                    let mut controller = controller.as_mut().unwrap();
                    let mut parameters = parameters.lock().await;
                    let mut bitmap = bitmap.lock().await;
                    let mut counter = counter.lock().await;
                    let mut move_status = move_status.lock().await;
                    // Update the move status
                    // Don't update the limit status and path img too frequently
                    if *counter > UPDATE_COUNT {
                        path_img.update(move |path| {
                            *path = bitmap.to_data_url();
                        });
                        *counter = 0;
                        update_limit_status(&mut controller, &mut parameters, &mut limit_status)
                            .await
                            .expect("Failed to update limit status");
                        move_status_signal.update(|status| {
                            *status = move_status.clone();
                        });
                    } else {
                        // println!("Skipping limit status update, counter: {}", *counter);
                        *counter += 1;
                        update_move_status(
                            &mut controller,
                            &mut parameters,
                            &mut move_status,
                            &mut bitmap,
                        )
                        .await
                        .expect("Failed to update move status");
                    }
                }
                tokio::time::sleep(Duration::from_millis(MOVE_STATUS_UPDATE_INTERVAL as u64)).await;
            }
        });
        Ok(())
    }
    pub async fn stop_polling(&self) -> Result<(), ServerFnError> {
        self.polling_tasks.lock().await.shutdown().await;
        Ok(())
    }
    pub async fn clear_path(&self) -> Result<(), ServerFnError> {
        let mut bitmap = self.bitmap.lock().await;
        bitmap.clear();
        self.path_img.set(String::new());
        Ok(())
    }

    pub async fn init(&self, controller_type: ControllerType) -> Result<(), ServerFnError> {
        let mut controller = self.controller.lock().await;
        if controller.is_some() {
            return Err(ServerFnError::ServerError(
                "Controller is already initialized".to_string(),
            ));
        }
        match controller_type {
            ControllerType::Zmc(ip) => {
                let mut zmc_controller = ZmcController::new();
                zmc_controller.open_eth(&ip)?;
                *controller = Some(Box::new(zmc_controller));
            }
            ControllerType::Fake => {
                *controller = Some(Box::new(FakeController::new()));
            }
        }
        Ok(())
    }

    pub async fn deinit(&self) -> Result<(), ServerFnError> {
        let mut controller = self.controller.lock().await;
        if controller.is_none() {
            return Ok(());
        }
        let controller_unwrapped = controller.as_mut().unwrap();
        if controller_unwrapped.is_open() {
            controller_unwrapped.close()?;
        }
        controller.take(); // Clear the controller
        Ok(())
    }

    /// Helper function to execute operations that require controller
    /// return error if the controller is not open
    pub async fn with_controller<F, R>(&self, op: F) -> Result<R, ServerFnError>
    where
        F: FnOnce(&mut Box<dyn Controller + Send>) -> Result<R, ControllerError>,
    {
        let mut controller = self.controller.lock().await;
        if controller.is_none() {
            return Err(ServerFnError::ServerError(
                "Controller is not initialized".to_string(),
            ));
        }
        let controller = controller.as_mut().unwrap();
        if !controller.is_open() {
            return Err(ServerFnError::ServerError(
                "Controller is not open".to_string(),
            ));
        }
        Ok(op(controller)?)
    }
}

#[cfg(feature = "ssr")]
static ZMC_MANAGER: LazyLock<ZmcManager> = LazyLock::new(|| ZmcManager {
    controller: Arc::new(Mutex::new(None)),
    parameters: Arc::new(Mutex::new(Parameters::default())),
    polling_interval: Arc::new(Mutex::new(Duration::from_millis(100))),
    polling_tasks: Arc::new(Mutex::new(JoinSet::new())),
    limit_status: ServerSignal::new("limit_status".to_string(), LimitStatus::default()).unwrap(),
    move_status: Arc::new(Mutex::new(MoveStatus::default())),
    move_status_signal: ServerSignal::new("move_status".to_string(), MoveStatus::default())
        .unwrap(),
    path_img_update_counter: Arc::new(Mutex::new(0)),
    path_img: ServerSignal::new("path_img".to_string(), String::new()).unwrap(),
    bitmap: Arc::new(Mutex::new(Bitmap::new(500, 500, 4.0))), // 500x500 bitmap with scale 10.0
});

#[server]
pub async fn zmc_init_eth(ip: String) -> Result<(), ServerFnError> {
    ZMC_MANAGER.deinit().await?;
    ZMC_MANAGER.init(ControllerType::Zmc(ip)).await?;
    ZMC_MANAGER.start_polling().await
}

#[server]
pub async fn zmc_init_fake() -> Result<(), ServerFnError> {
    ZMC_MANAGER.deinit().await?;
    ZMC_MANAGER.init(ControllerType::Fake).await?;
    ZMC_MANAGER.start_polling().await
}

#[server]
pub async fn zmc_close() -> Result<(), ServerFnError> {
    ZMC_MANAGER.stop_polling().await?;
    ZMC_MANAGER.with_controller(|c| Ok(c.close()?)).await
}

// 设定参数
#[server]
pub async fn zmc_set_parameters(params: Parameters) -> Result<(), ServerFnError> {
    println!("Setting parameters: {:?}", params);
    *ZMC_MANAGER.parameters.lock().await = params.clone();
    ZMC_MANAGER
        .with_controller(|controller| {
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
                controller
                    .direct_set_invert_in(io, params.inverted_status.limit_io_level_inverted)?;
            }

            let axis_num_list = [params.x.axis_num, params.y.axis_num, params.z.axis_num];
            let axis_params = [params.x, params.y, params.z];

            for i in axis_num_list {
                // TODO: Change to 65 after simulation
                // controller.direct_set_a_type(i, 0)?;
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
                controller
                    .direct_set_fs_limit(i, axis_params[i as usize].software_positive_limit)?;
                // 设置软件负限位
                controller
                    .direct_set_rs_limit(i, axis_params[i as usize].software_negative_limit)?;
                // 设置硬件正限位IO
                controller.direct_set_fwd_in(i, axis_params[i as usize].positive_limit_io)?;
                // 设置硬件负限位IO
                controller.direct_set_rev_in(i, axis_params[i as usize].negative_limit_io)?;
                // 设置回零开关IO
                // controller.direct_set_datum_in(i, axis_params[i as usize].zero_point_io)?;
                controller.direct_set_alm_in(i, params.emergency_stop_io)?;
                // TODO: 设置PID参数
            }
            Ok(())
        })
        .await
}

#[server]
pub async fn zmc_get_idle(axis: u8) -> Result<bool, ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            let is_idle = controller.direct_get_if_idle(axis)?;
            Ok(is_idle)
        })
        .await
}

// 绝对移动
#[server]
pub async fn zmc_move_abs(axis_list: Vec<u8>, pos_list: Vec<f32>) -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            controller.direct_move_abs(
                axis_list.len() as u8,
                axis_list.as_ref(),
                pos_list.as_ref(),
            )?;
            Ok(())
        })
        .await
}
// 相对移动
#[server]
pub async fn zmc_move(axis_list: Vec<u8>, pos_list: Vec<f32>) -> Result<(), ServerFnError> {
    if axis_list.len() != pos_list.len() {
        return Err(ServerFnError::ServerError(
            "Axis list and position list must have the same length".to_string(),
        ));
    }
    if axis_list.is_empty() {
        return Err(ServerFnError::ServerError(
            "Axis list cannot be empty".to_string(),
        ));
    }
    ZMC_MANAGER
        .with_controller(|controller| {
            controller.direct_move(axis_list.len() as u8, axis_list.as_ref(), pos_list.as_ref())?;
            Ok(())
        })
        .await
}
// 设置速度
#[server]
pub async fn zmc_set_speed(axis: u8, speed: f32) -> Result<(), ServerFnError> {
    if speed < 0.0 {
        return Err(ServerFnError::ServerError(
            "Speed cannot be negative".to_string(),
        ));
    }
    ZMC_MANAGER
        .with_controller(|controller| {
            controller.direct_set_speed(axis, speed)?;
            Ok(())
        })
        .await
}
// 变频器运行
#[server]
pub async fn zmc_converter_set_freq(freq: u32) -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            controller.modbus_set4x_long(3, 1, &[freq as i32])?;
            controller.execute("MODBUSM_REGSET(100,1,3)")?;
            Ok(())
        })
        .await
}

#[server]
pub async fn zmc_converter_run(inverted: bool) -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            if inverted {
                controller.execute("MODBUSM_REGSET(99,1,0)")?;
            } else {
                controller.execute("MODBUSM_REGSET(99,1,2)")?;
            }
            Ok(())
        })
        .await
}

// 变频器停止
#[server]
pub async fn zmc_converter_stop() -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            controller.execute("MODBUSM_REGSET(99,1,1)")?;
            Ok(())
        })
        .await
}

// 设置输入轴的电平反转
#[server]
pub async fn zmc_set_in_inverted(in_num: u16, inverted: bool) -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            controller.direct_set_invert_in(in_num, inverted)?;
            Ok(())
        })
        .await
}

// 手动移动轴,输入轴和运动的正负，
#[server]
pub async fn zmc_manual_move(axis: u8, direction: i8) -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            controller.direct_single_v_move(axis, direction)?;
            Ok(())
        })
        .await
}

// 手动停止轴
#[server]
pub async fn zmc_manual_stop(axis: u8) -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            controller.direct_single_cancel(axis, 2)?;
            Ok(())
        })
        .await
}

// 获取当前轴位置
#[server]
pub async fn zmc_get_axis_position(axis: u8) -> Result<f32, ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            let position = controller.direct_get_d_pos(axis)?;
            Ok(position)
        })
        .await
}

//  寻找零点
#[server]
pub async fn zmc_datum(axis: u8) -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            // 获取当前轴的正负
            let pos = controller.direct_get_d_pos(axis)?;
            if pos > 0.0 {
                controller.direct_single_v_move(axis, 19)?;
            } else {
                controller.direct_single_v_move(axis, 18)?;
            }
            Ok(())
        })
        .await
}

// 轴坐标清零
#[server]
pub async fn zmc_set_zero(axis_list: Vec<u8>) -> Result<(), ServerFnError> {
    ZMC_MANAGER
        .with_controller(|controller| {
            for axis in axis_list {
                controller.direct_set_d_pos(axis, 0.0)?;
                controller.direct_set_m_pos(axis, 0.0)?;
            }
            Ok(())
        })
        .await
}

// 清除路径图像
#[server]
pub async fn zmc_clear_path() -> Result<(), ServerFnError> {
    ZMC_MANAGER.clear_path().await
}
