use crate::api::{
    zmc_converter_run, zmc_converter_set_freq, zmc_converter_stop, zmc_move_abs, zmc_set_speed,
};
use leptos::prelude::*;
use leptos_ws::ServerSignal;
#[cfg(feature = "ssr")]
use std::sync::Arc;
#[cfg(feature = "ssr")]
use std::sync::LazyLock;
#[cfg(feature = "ssr")]
use tokio::sync::Mutex;
#[cfg(feature = "ssr")]
use crate::utils::Bitmap;

#[cfg(feature = "ssr")]
#[derive(Debug, serde::Serialize, serde::Deserialize, thiserror::Error)]
enum GCodeError {
    #[error("Invalid G-code command: {0}")]
    ParseError(String),
    #[error("Failed to execute G-code command: {0}")]
    ExecutionError(String),
}

#[cfg(feature = "ssr")]
struct GCodeManager {
    // G-code file content lines
    lines: Arc<Mutex<Vec<String>>>,
    // Current line being processed
    current_line: ServerSignal<usize>,
    thread_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    bitmap: Arc<Mutex<Bitmap>>,
}

#[cfg(feature = "ssr")]
impl GCodeManager {
    pub fn new() -> Self {
        GCodeManager {
            lines: Arc::new(Mutex::new(Vec::new())),
            current_line: ServerSignal::new("current_line".to_string(), 0).unwrap(),
            thread_handle: Arc::new(Mutex::new(None)),
            bitmap: Arc::new(Mutex::new(Bitmap::new(800, 800, 4.0))),
        }
    }

    pub async fn load_gcode(&self, content: String) {
        let mut lines = self.lines.lock().await;
        *lines = content.lines().map(|line| line.to_string()).collect();
        self.current_line.update(|v| *v = 0);
    }

    // Add a new method to generate the bitmap path preview
    pub async fn generate_path_preview(&self) -> Result<(), String> {
        let lines = self.lines.lock().await;
        let mut bitmap = self.bitmap.lock().await;

        // Clear the bitmap before drawing new path
        bitmap.clear();

        // Initialize tool position
        let mut current_x: f32 = 0.0;
        let mut current_y: f32 = 0.0;
        let mut current_z: f32 = 0.0;

        // Process all G-code lines to generate the path
        for line in lines.iter() {
            if let Some(command) = parse_gcode_line(line) {
                preview_gcode_movement(
                    &command,
                    &mut bitmap,
                    &mut current_x,
                    &mut current_y,
                    &mut current_z,
                );
            }
        }
        // Generate data URL and update the path_img signal
        let data_url = bitmap.to_data_url();
        let path_img = ServerSignal::new("path_img".to_string(), String::new()).unwrap();
        path_img.update(|v| *v = data_url);

        Ok(())
    }

    pub async fn start(&self) -> Result<(), String> {
        let lines = self.lines.clone();
        let current_line = self.current_line.clone();
        // Check if already running
        if self.thread_handle.lock().await.is_some() {
            return Err("G-code execution already in progress".to_string());
        }
        // Spawn a new task to execute G-code lines
        let handle = tokio::spawn(async move {
            loop {
                // Check if execution is completed
                // TODO: Not necessary to check this flag
                let lines = lines.lock().await;
                let current_line_index = current_line.get();
                if current_line_index >= lines.len() {
                    // All lines executed, exit the loop
                    println!("All G-code lines executed.");
                    break;
                }
                // Execute one line of G-code
                if let Err(e) = execute_one_line(&lines[current_line_index as usize]).await {
                    eprintln!("Error executing G-code line: {}", e);
                    break;
                }
                zmc_wait_idle(&[0, 1, 2]).await; // Wait for axis to be idle
                                                 // Update the current line index
                current_line.update(|v| *v += 1);
            }
        });
        self.thread_handle.lock().await.replace(handle);
        Ok(())
    }

    pub async fn stop(&self) {
        // Stop the G-code execution thread if it exists
        if let Some(handle) = self.thread_handle.lock().await.take() {
            handle.abort();
        }
    }

    pub async fn reset(&self) {
        self.current_line.update(|v| *v = 0);
    }
}

#[cfg(feature = "ssr")]
async fn execute_one_line(line: &str) -> Result<(), String> {
    let g_code_command = parse_gcode_line(line);
    if let Some(command) = g_code_command {
        interpret_gcode_movement(&command).await;
    } else {
        eprintln!("Failed to parse G-code line: {}", line);
    }

    Ok(())
}

/// Represents a parsed G-code command
#[cfg(feature = "ssr")]
#[derive(Debug, Clone)]
pub struct GCodeCommand {
    pub command_type: String,         // G, M, T, etc
    pub command_number: i32,          // The number after the command type (G1, M104, etc)
    pub parameters: Vec<(char, f64)>, // Parameters like X10.5, Y20, etc.
    pub comment: Option<String>,
}

/// Parse a single line of G-code
#[cfg(feature = "ssr")]
pub fn parse_gcode_line(line: &str) -> Option<GCodeCommand> {
    // Skip empty lines and pure comment lines
    let line = line.trim();
    if line.is_empty() || line.starts_with(';') {
        return None;
    }

    // Extract comment if present
    let (code_part, comment) = match line.split_once(';') {
        Some((code, comment)) => (code.trim(), Some(comment.trim().to_string())),
        None => (line, None),
    };

    // Find the command (G, M, T, etc)
    let re_command = regex::Regex::new(r"^([A-Za-z])(\d+)").unwrap();
    let command_cap = if let Some(caps) = re_command.captures(code_part) {
        (
            caps.get(1).unwrap().as_str().to_uppercase(),
            caps.get(2).unwrap().as_str().parse::<i32>().unwrap_or(0),
        )
    } else {
        return None; // No valid command found
    };

    // Extract parameters (X, Y, Z, E, F, etc)
    let re_params = regex::Regex::new(r"([A-Za-z])(-?\d*\.?\d+)").unwrap();
    let mut parameters = Vec::new();

    for cap in re_params.captures_iter(code_part) {
        if cap.get(0).unwrap().start() == 0 {
            // Skip the initial command we already processed
            continue;
        }

        let param_letter = cap.get(1).unwrap().as_str().chars().next().unwrap();
        let param_value = cap.get(2).unwrap().as_str().parse::<f64>().unwrap_or(0.0);
        parameters.push((param_letter, param_value));
    }

    Some(GCodeCommand {
        command_type: command_cap.0,
        command_number: command_cap.1,
        parameters,
        comment,
    })
}

#[cfg(feature = "ssr")]
async fn zmc_wait_idle(axis_list: &[u8]) {
    // Wait for the ZMC to be idle before executing the next command
    let mut idle_axis_num;
    use super::zmc_get_idle;
    loop {
        idle_axis_num = 0; // Reset idle count for each iteration
        for axis in axis_list {
            // Try to get idle status up to 10 times
            if zmc_get_idle(*axis)
                .await
                .expect("Failed to get idle status")
            {
                idle_axis_num += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }
        if idle_axis_num == axis_list.len() {
            // All axes are idle
            return;
        }
    }
}

#[cfg(feature = "ssr")]
async fn interpret_gcode_movement(command: &GCodeCommand) {
    let mut movement = String::new();
    // Handle G commands (movement related)
    if command.command_type == "G" {
        match command.command_number {
            0 | 1 => {
                // G0: Rapid positioning, G1: Linear interpolation
                if command.command_number == 0 {
                    movement = String::from("Rapid move to");
                } else {
                    movement = String::from("Linear move to");
                }

                // Extract coordinates
                for (param, value) in &command.parameters {
                    let value = value.clone() as f32;
                    match param {
                        'X' => {
                            zmc_move_abs(vec![0], vec![value])
                                .await
                                .expect("Failed to move in X direction");
                            movement.push_str(format!(" {} in X direction,", value).as_str())
                        }
                        'Y' => {
                            zmc_move_abs(vec![1], vec![value])
                                .await
                                .expect("Failed to move in Y direction");
                            movement.push_str(format!(" {} in Y direction,", value).as_str())
                        }
                        'Z' => {
                            zmc_move_abs(vec![2], vec![value])
                                .await
                                .expect("Failed to move in Z direction");
                            movement.push_str(format!(" {} in Z direction,", value).as_str())
                        }
                        'F' => {
                            for i in 0..3 {
                                zmc_set_speed(i, value as f32)
                                    .await
                                    .expect("Failed to set speed");
                            }
                            movement.push_str(&format!(" at speed {:.0}", value));
                        }
                        _ => {
                            // Ignore other parameters
                            eprintln!("Ignoring unsupported parameter: {}", param);
                        }
                    }
                }
            }
            2 | 3 => {
                // G2/G3: Arc movement (clockwise/counterclockwise)
                let direction = if command.command_number == 2 {
                    "clockwise"
                } else {
                    "counterclockwise"
                };
                movement = format!("Arc move {} to", direction);

                // Extract end coordinates and arc parameters
                let mut has_ij = false;

                for (param, value) in &command.parameters {
                    match param {
                        'X' | 'Y' | 'Z' => {
                            movement.push_str(&format!(" {}:{:.3}", param, value));
                        }
                        'I' | 'J' => {
                            has_ij = true;
                        }
                        'R' => {
                            movement.push_str(&format!(" with radius {:.3}", value));
                        }
                        'F' => {
                            movement.push_str(&format!(" at speed {:.0}", value));
                        }
                        _ => {} // Ignore other parameters
                    }
                }

                if has_ij {
                    movement.push_str(" using IJK arc definition");
                }
            }
            4 => {
                // G4: Dwell/pause
                let mut time = 0.0;
                for (param, value) in &command.parameters {
                    if *param == 'P' {
                        time = *value;
                        break;
                    }
                }
                movement.push_str(format!("Pause/dwell for {:.3} milliseconds", time).as_str());
            }
            28 => {
                // G28: Home axes
                let mut axes = Vec::new();

                // Check which axes are being homed
                if command.parameters.is_empty() {
                    // No parameters means home all axes
                    axes.push("all axes".to_string());
                } else {
                    for (param, _) in &command.parameters {
                        if *param == 'X' || *param == 'Y' || *param == 'Z' {
                            axes.push(param.to_string());
                        }
                    }
                }

                if axes.is_empty() {
                    movement.push_str("Home axes (no axis specified)");
                } else {
                    movement.push_str(format!("Home {}", axes.join(", ")).as_str());
                }
            }
            90 => movement.push_str("Set absolute positioning mode"),
            91 => movement.push_str("Set relative positioning mode"),
            92 => movement.push_str("Set position (reset origin point)"),
            _ => movement.push_str(format!("Unknown G{} command", command.command_number).as_str()),
        }
    }
    // Handle M commands (machine state related)
    else if command.command_type == "M" {
        match command.command_number {
            0 => {
                zmc_converter_stop()
                    .await
                    .expect("Failed to stop converter");
                movement.push_str("Emergency stop");
            }
            1 => {
                zmc_converter_stop()
                    .await
                    .expect("Failed to stop converter");
                movement.push_str("Sleep/pause operation");
            }
            3 | 4 => {
                let direction = if command.command_number == 3 {
                    zmc_converter_run(false)
                        .await
                        .expect("Failed to stop converter");
                    "clockwise"
                } else {
                    zmc_converter_run(true)
                        .await
                        .expect("Failed to stop converter");
                    "counterclockwise"
                };
                let mut speed = String::new();

                for (param, value) in &command.parameters {
                    if *param == 'S' {
                        speed = format!(" at speed {:.0}", value);
                        let value = value.clone();
                        zmc_converter_set_freq(value as u32)
                            .await
                            .expect("Failed to set converter frequency");
                        break;
                    }
                }
                movement.push_str(format!("Spindle on {}{}", direction, speed).as_str());
            }
            5 => {
                zmc_converter_stop()
                    .await
                    .expect("Failed to stop converter");
                movement.push_str("Spindle stop");
            }
            84 => movement.push_str("Stop idle hold"),
            104 | 109 => {
                let wait = if command.command_number == 109 {
                    " and wait"
                } else {
                    ""
                };
                let mut temp = 0.0;

                for (param, value) in &command.parameters {
                    if *param == 'S' {
                        temp = *value;
                        break;
                    }
                }

                movement.push_str(
                    format!("Set extruder temperature to {:.0}°C{}", temp, wait).as_str(),
                );
            }
            140 | 190 => {
                let wait = if command.command_number == 190 {
                    " and wait"
                } else {
                    ""
                };
                let mut temp = 0.0;

                for (param, value) in &command.parameters {
                    if *param == 'S' {
                        temp = *value;
                        break;
                    }
                }

                movement.push_str(format!("Set bed temperature to {:.0}°C{}", temp, wait).as_str());
            }
            _ => movement
                .push_str(format!("Other state change: M{}", command.command_number).as_str()),
        }
    }
    // Handle other command types
    else {
        println!(
            "Non-movement command: {}{}",
            command.command_type, command.command_number
        );
    }
    println!("command >>> {:?}", movement);
}

// Helper function to draw a line on the bitmap
#[cfg(feature = "ssr")]
fn draw_line(bitmap: &mut Bitmap, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) {
    // Use Bresenham's line algorithm for drawing
    let dx = (x2 - x1).abs();
    let dy = (y2 - y1).abs();
    let steps = dx.max(dy).max(1.0) * 4.0; // Increase resolution for smoother lines

    // Interpolate points along the line
    for i in 0..=steps as usize {
        let t = i as f32 / steps;
        let x = x1 + (x2 - x1) * t;
        let y = y1 + (y2 - y1) * t;
        let z = z1 + (z2 - z1) * t;

        // Set the pixel in the bitmap - z value determines color
        bitmap.set_pixel(x, y, z);
    }
}

// Helper function to draw an arc on the bitmap
#[cfg(feature = "ssr")]
fn draw_arc(
    bitmap: &mut Bitmap,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
    i: f32,
    j: f32,
    is_clockwise: bool,
) {
    // Calculate center point
    let center_x = x1 + i;
    let center_y = y1 + j;

    // Calculate angles
    let start_angle = (y1 - center_y).atan2(x1 - center_x);
    let end_angle = (y2 - center_y).atan2(x2 - center_x);

    // Calculate radius
    let radius = ((x1 - center_x).powi(2) + (y1 - center_y).powi(2)).sqrt();

    // Determine angle direction and step
    let mut angle = start_angle;
    let mut angle_step = 0.05; // Small step for smooth arcs

    // Adjust direction based on clockwise flag
    if is_clockwise {
        if end_angle > start_angle {
            angle_step = -((2.0 * std::f32::consts::PI) - (end_angle - start_angle)) / 100.0;
        } else {
            angle_step = -(start_angle - end_angle) / 100.0;
        }
    } else {
        if end_angle < start_angle {
            angle_step = ((2.0 * std::f32::consts::PI) - (start_angle - end_angle)) / 100.0;
        } else {
            angle_step = (end_angle - start_angle) / 100.0;
        }
    }

    // Make sure we have enough steps
    let steps = ((end_angle - start_angle).abs() / angle_step.abs()).max(50.0) as usize;
    angle_step = (end_angle - start_angle) / steps as f32;
    if is_clockwise {
        angle_step = -angle_step;
    }

    // Interpolate z-value
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let z = z1 + (z2 - z1) * t;

        // Calculate point on arc
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();

        // Set the pixel
        bitmap.set_pixel(x, y, z);

        // Advance angle
        angle += angle_step;
    }
}

#[cfg(feature = "ssr")]
fn preview_gcode_movement(
    command: &GCodeCommand,
    bitmap: &mut Bitmap,
    current_x: &mut f32,
    current_y: &mut f32,
    current_z: &mut f32,
) {
    if command.command_type == "G" {
        match command.command_number {
            0 | 1 => {
                // G0/G1: linear movement
                let mut target_x = *current_x;
                let mut target_y = *current_y;
                let mut target_z = *current_z;
                let mut has_movement = false;

                // Extract target coordinates
                for (param, value) in &command.parameters {
                    match param {
                        'X' => {
                            target_x = *value as f32;
                            has_movement = true;
                        }
                        'Y' => {
                            target_y = *value as f32;
                            has_movement = true;
                        }
                        'Z' => {
                            target_z = *value as f32;
                            has_movement = true;
                        }
                        _ => {} // Ignore other parameters for preview
                    }
                }

                if has_movement {
                    // Draw line from current position to target position
                    draw_line(
                        bitmap, *current_x, *current_y, *current_z, target_x, target_y, target_z,
                    );

                    // Update current position
                    *current_x = target_x;
                    *current_y = target_y;
                    *current_z = target_z;
                }
            }
            2 | 3 => {
                // G2/G3: arc movement
                let is_clockwise = command.command_number == 2;
                let mut target_x = *current_x;
                let mut target_y = *current_y;
                let mut target_z = *current_z;
                let mut center_x_offset = 0.0; // I: X offset from current position to arc center
                let mut center_y_offset = 0.0; // J: Y offset from current position to arc center
                let mut has_movement = false;

                for (param, value) in &command.parameters {
                    match param {
                        'X' => {
                            target_x = *value as f32;
                            has_movement = true;
                        }
                        'Y' => {
                            target_y = *value as f32;
                            has_movement = true;
                        }
                        'Z' => {
                            target_z = *value as f32;
                        }
                        'I' => center_x_offset = *value as f32,
                        'J' => center_y_offset = *value as f32,
                        _ => {} // Ignore other parameters for preview
                    }
                }

                if has_movement {
                    // Draw arc from current position to target position
                    draw_arc(
                        bitmap,
                        *current_x,
                        *current_y,
                        *current_z,
                        target_x,
                        target_y,
                        target_z,
                        center_x_offset,
                        center_y_offset,
                        is_clockwise,
                    );

                    // Update current position
                    *current_x = target_x;
                    *current_y = target_y;
                    *current_z = target_z;
                }
            }
            _ => {} // Ignore other G commands for preview
        }
    }
    // We don't need to handle M commands for path preview
}

#[server]
pub async fn debug_update_line() -> Result<(), ServerFnError> {
    // Force an update to the current line to test WebSocket connection
    let current_line = G_CODE_MANAGER.current_line.clone();
    let value = current_line.get();

    println!(
        "Debug: Updating current line from {} to {}",
        value,
        value + 1
    );
    current_line.update(|v| *v += 1);

    Ok(())
}

#[cfg(feature = "ssr")]
static G_CODE_MANAGER: LazyLock<GCodeManager> = LazyLock::new(GCodeManager::new);

#[server]
pub async fn load_gcode(content: String) -> Result<(), ServerFnError> {
    G_CODE_MANAGER.load_gcode(content).await;
    Ok(())
}
#[server]
pub async fn start_gcode_execution() -> Result<(), ServerFnError> {
    Ok(G_CODE_MANAGER
        .start()
        .await
        .map_err(|e| ServerFnError::new(e))?)
}
#[server]
pub async fn stop_gcode_execution() -> Result<(), ServerFnError> {
    G_CODE_MANAGER.stop().await;
    Ok(())
}
#[server]
pub async fn reset_gcode_execution() -> Result<(), ServerFnError> {
    G_CODE_MANAGER.reset().await;
    Ok(())
}
#[server]
pub async fn generate_path_preview() -> Result<(), ServerFnError> {
    G_CODE_MANAGER.generate_path_preview().await.expect("Failed to generate path preview");
    Ok(())
}
