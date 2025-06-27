use crate::api::zmc_clear_path;
use crate::app::GlobalState;
use crate::model::MoveStatus;
use lazy_static::lazy_static;
use leptos::html::Canvas;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::{logging, prelude::*, server::codee::string::JsonSerdeCodec};
use leptos_use::storage::use_storage;
use leptos_use::{use_cookie, watch_debounced};
use leptos_ws::ServerSignal;
use std::cell::RefCell;
use std::rc::Rc;
use thaw::*;
use web_sys::wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

#[component]
pub fn PathVisualizer() -> impl IntoView {
    // Subscribe to the svg_path signal from the server
    let path_img = ServerSignal::new("path_img".to_string(), String::new())
        .expect("Failed to create client signal");

    // Create some states for visualization controls
    let zoom = RwSignal::new(1.0);
    let offset_x = RwSignal::new(200.0);
    let offset_y = RwSignal::new(200.0);

    // Mouse interaction states
    let dragging = RwSignal::new(false);
    let start_x = RwSignal::new(0);
    let start_y = RwSignal::new(0);
    let start_offset_x = RwSignal::new(0.0);
    let start_offset_y = RwSignal::new(0.0);

    // Function to handle wheel events for zooming
    let handle_wheel = move |e: web_sys::WheelEvent| {
        e.prevent_default();
        let delta = -e.delta_y() / 500.0; // Adjust sensitivity
        zoom.update(|z| {
            let new_zoom = *z * (1.0 + delta as f64);
            // Limit zoom range to prevent extreme values
            *z = new_zoom.clamp(0.1, 10.0);
        });
    };

    // Mouse event handlers for panning
    let handle_mouse_down = move |e: web_sys::MouseEvent| {
        dragging.set(true);
        start_x.set(e.client_x());
        start_y.set(e.client_y());
        start_offset_x.set(offset_x.get());
        start_offset_y.set(offset_y.get());
    };

    let handle_mouse_move = move |e: web_sys::MouseEvent| {
        if dragging.get() {
            let dx = e.client_x() - start_x.get();
            let dy = e.client_y() - start_y.get();
            offset_x.set(start_offset_x.get() + dx as f64);
            offset_y.set(start_offset_y.get() + dy as f64);
        }
    };

    let handle_mouse_up = move |_| {
        dragging.set(false);
    };

    // Function to reset the view
    let reset_view = move |_| {
        zoom.set(1.0);
        offset_x.set(200.0);
        offset_y.set(200.0);
    };

    let clear_view = move |_| {
        // Clear the path image
        spawn_local(async move {
            zmc_clear_path().await.expect("Failed to clear path");
        });
    };

    // Create a zooming status message
    let zoom_text = move || format!("Zoom: {}%", (zoom() * 100.0).round());

    // Calculate transform value for svg
    let transform = move || format!("translate({},{}) scale({})", offset_x(), offset_y(), zoom());

    view! {
        <div class="path-visualizer-container">
            <h3>"Machine Path Visualization"</h3>

            // Controls for the visualization
            <div class="control-panel">
                <button on:click=move |_| zoom.update(|z| *z *= 1.2)>"Zoom In"</button>
                <button on:click=move |_| zoom.update(|z| *z /= 1.2)>"Zoom Out"</button>
                <button on:click=reset_view>"Reset View"</button>
                <button on:click=clear_view>"Clear View"</button>
            </div>

            // SVG container
            <div
                class="svg-container"
                style="border: 1px solid #ccc; margin-top: 10px; position: relative;"
            >
                <svg
                    width="400"
                    height="400"
                    viewBox="0 0 400 400"
                    style="background: #f8f8f8;"
                    on:mousedown=handle_mouse_down
                    on:mousemove=handle_mouse_move
                    on:mouseup=handle_mouse_up
                    on:mouseleave=handle_mouse_up
                    on:wheel=handle_wheel
                >
                    <g transform=transform>
                        // Grid for reference
                        <defs>
                            <pattern id="grid" width="10" height="10" patternUnits="userSpaceOnUse">
                                <path
                                    d="M 10 0 L 0 0 0 10"
                                    fill="none"
                                    stroke="#ddd"
                                    stroke-width="0.5"
                                />
                            </pattern>
                        </defs>
                        <rect x="-1000" y="-1000" width="2000" height="2000" fill="url(#grid)" />

                        // Origin marker
                        <circle cx="0" cy="0" r="3" fill="red" />

                        // The machine path
                        {move || {
                            let bitmap_data_url = path_img.get();
                            if bitmap_data_url.is_empty()
                                || !bitmap_data_url.starts_with("data:image/png;base64,")
                            {
                                view! {
                                    <g class="loading-message">
                                        <text
                                            x="0"
                                            y="0"
                                            text-anchor="middle"
                                            font-family="sans-serif"
                                            fill="#666"
                                        >
                                            "Waiting for machine data..."
                                        </text>
                                    </g>
                                }
                            } else {
                                view! {
                                    <g class="bitmap-container">
                                        <image
                                            href=bitmap_data_url
                                            x="-250"
                                            y="-250"
                                            width="500"
                                            height="500"
                                        />
                                    </g>
                                }
                            }
                        }}
                    </g>
                </svg>
                <div class="zoom-info">{move || zoom_text()}</div>
            </div>
        </div>
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
struct Point {
    x: f64,
    y: f64,
    color: u8, //指定当前绘制的画笔的颜色
}

// HSV转RGB颜色转换
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h_prime as u8 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };

    let r = ((r1 + m) * 255.0) as u8;
    let g = ((g1 + m) * 255.0) as u8;
    let b = ((b1 + m) * 255.0) as u8;

    (r, g, b)
}

// 使用lazy_static预计算HSV->RGB转换的查表数组
lazy_static! {
    static ref RGB_TABLE_R: [u8; 256] = {
        let mut table = [0u8; 256];
        for i in 0..256 {
            let hue = (i as f64 / 255.0) * 360.0;
            let (r, _, _) = hsv_to_rgb(hue, 1.0, 0.9);
            table[i] = r;
        }
        table
    };
    static ref RGB_TABLE_G: [u8; 256] = {
        let mut table = [0u8; 256];
        for i in 0..256 {
            let hue = (i as f64 / 255.0) * 360.0;
            let (_, g, _) = hsv_to_rgb(hue, 1.0, 0.9);
            table[i] = g;
        }
        table
    };
    static ref RGB_TABLE_B: [u8; 256] = {
        let mut table = [0u8; 256];
        for i in 0..256 {
            let hue = (i as f64 / 255.0) * 360.0;
            let (_, _, b) = hsv_to_rgb(hue, 1.0, 0.9);
            table[i] = b;
        }
        table
    };
}
// 使用查表法获取RGB值
fn hsv_to_rgb_r(color_val: u8) -> u8 {
    RGB_TABLE_R[color_val as usize]
}

fn hsv_to_rgb_g(color_val: u8) -> u8 {
    RGB_TABLE_G[color_val as usize]
}

fn hsv_to_rgb_b(color_val: u8) -> u8 {
    RGB_TABLE_B[color_val as usize]
}

impl Point {
    fn get_color(&self) -> String {
        // 将u8值映射到HSV色彩空间的色相(0-360度)，然后转为RGB
        let hue = (self.color as f64 / 255.0) * 360.0;
        // 固定饱和度和明度为高值，保证颜色鲜艳
        let (r, g, b) = hsv_to_rgb(hue, 1.0, 0.9);
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }
}
// 使用矢量存储所有已绘制的点，而不是依赖图像
#[derive(Clone, Debug)]
struct PathHistory {
    points: Vec<Point>,   // 存储已绘制的点
    current_index: usize, // 当前绘制到的索引
    // 用于减少存储和绘制的点数
    simplification_tolerance: f64,
    // 存储线段而非每个点以减少内存使用
    segments: Vec<PathSegment>,
    // 当前正在构建的线段
    current_segment: Option<PathSegment>,
    // 分块存储数据，用于快速渲染
    chunks: Vec<PathChunk>,
}

// 存储连续颜色相近的线段
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
struct PathSegment {
    points: Vec<Point>,
    color: u8,
}

// 用于优化渲染的数据块
#[derive(Clone, Debug)]
struct PathChunk {
    bounds: Rect,
    segments: Vec<usize>, // 索引到主segments列表
}

// 简单的矩形结构，用于空间划分
#[derive(Clone, Debug)]
struct Rect {
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
}

impl PathHistory {
    fn new() -> Self {
        Self {
            points: Vec::new(),
            current_index: 0,
            simplification_tolerance: 1.0, // 简化阈值，根据需要调整
            segments: Vec::new(),
            current_segment: None,
            chunks: Vec::new(),
        }
    }

    fn add_point(&mut self, x: f64, y: f64, color: u8) {
        let new_point = Point { x, y, color };

        // 仅当点与上一个点不在简化阈值内时才添加
        let should_add = self.points.last().map_or(true, |last_point| {
            let dx = last_point.x - x;
            let dy = last_point.y - y;
            let distance_squared = dx * dx + dy * dy;

            // 距离超过阈值或颜色变化明显时才添加点
            let color_change = (last_point.color as i16 - color as i16).abs() > 10;

            distance_squared > self.simplification_tolerance * self.simplification_tolerance
                || color_change
        });

        if should_add {
            // 添加到主点列表
            self.points.push(new_point.clone());

            // 处理线段构建
            let color_changed = self.update_segments(new_point);

            // 颜色变化时强制创建桥接点，确保实时绘制中线条不会断开
            if color_changed && self.points.len() >= 2 {
                // 这里已在update_segments中处理
            }

            // 定期重构空间分块数据（每100个点）
            if self.points.len() % 100 == 0 {
                self.rebuild_spatial_chunks();
            }
        }
    }

    // 返回布尔值表示颜色是否变化
    fn update_segments(&mut self, point: Point) -> bool {
        let mut color_changed = false;

        match &mut self.current_segment {
            None => {
                // 创建新线段
                let mut new_segment = PathSegment {
                    points: Vec::with_capacity(100),
                    color: point.color,
                };
                new_segment.points.push(point);
                self.current_segment = Some(new_segment);
            }
            Some(segment) => {
                // 检查颜色是否有明显变化
                color_changed = (segment.color as i16 - point.color as i16).abs() > 10;

                if color_changed {
                    // 颜色变化明显，创建桥接点并开始新线段

                    // 重要：添加桥接点，确保线段连续
                    let bridge_point = Point {
                        x: point.x,
                        y: point.y,
                        color: segment.color, // 使用旧颜色
                    };
                    segment.points.push(bridge_point.clone());

                    // 创建新线段并添加当前点
                    let mut new_segment = PathSegment {
                        points: Vec::with_capacity(100),
                        color: point.color,
                    };
                    // 先添加一个与桥接点坐标相同但颜色不同的点
                    new_segment.points.push(point);

                    // 完成当前线段，移到segments中
                    if let Some(completed_segment) = self.current_segment.take() {
                        self.segments.push(completed_segment);
                    }
                    self.current_segment = Some(new_segment);
                } else {
                    // 颜色相似，添加到当前线段
                    segment.points.push(point);
                }
            }
        }

        color_changed
    }

    fn rebuild_spatial_chunks(&mut self) {
        // 确保当前线段已添加到segments
        if let Some(segment) = self.current_segment.take() {
            self.segments.push(segment.clone());
            self.current_segment = Some(segment);
        }

        // 清除现有chunks
        self.chunks.clear();

        // 如果segments太少，不需要空间分块
        if self.segments.len() < 10 {
            return;
        }

        // 简单的空间分块策略：划分为16个区域
        let mut x_values: Vec<f64> = self.points.iter().map(|p| p.x).collect();
        let mut y_values: Vec<f64> = self.points.iter().map(|p| p.y).collect();
        x_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        y_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // 找到整体边界
        let x_min = *x_values.first().unwrap_or(&0.0);
        let x_max = *x_values.last().unwrap_or(&0.0);
        let y_min = *y_values.first().unwrap_or(&0.0);
        let y_max = *y_values.last().unwrap_or(&0.0);

        // 分区创建chunks
        let div = 4; // 4x4 分块
        let x_step = (x_max - x_min) / div as f64;
        let y_step = (y_max - y_min) / div as f64;

        for i in 0..div {
            for j in 0..div {
                let chunk_x_min = x_min + i as f64 * x_step;
                let chunk_x_max = chunk_x_min + x_step;
                let chunk_y_min = y_min + j as f64 * y_step;
                let chunk_y_max = chunk_y_min + y_step;

                let chunk_bounds = Rect {
                    x_min: chunk_x_min,
                    y_min: chunk_y_min,
                    x_max: chunk_x_max,
                    y_max: chunk_y_max,
                };

                // 找出与此chunk相交的线段
                let mut segment_indices = Vec::new();
                for (idx, segment) in self.segments.iter().enumerate() {
                    if segment.points.iter().any(|p| {
                        p.x >= chunk_x_min
                            && p.x <= chunk_x_max
                            && p.y >= chunk_y_min
                            && p.y <= chunk_y_max
                    }) {
                        segment_indices.push(idx);
                    }
                }

                if !segment_indices.is_empty() {
                    self.chunks.push(PathChunk {
                        bounds: chunk_bounds,
                        segments: segment_indices,
                    });
                }
            }
        }
    }

    fn draw_complete_path(&self, ctx: &CanvasRenderingContext2d, scale: f64, view_rect: &Rect) {
        if self.segments.len() < 1 && self.current_segment.is_none() {
            return;
        }

        ctx.set_line_width(2.0 / scale);

        // 高效绘制: 仅绘制可见区域内的块
        if !self.chunks.is_empty() {
            // 筛选可见的块
            let visible_chunks: Vec<&PathChunk> = self
                .chunks
                .iter()
                .filter(|chunk| {
                    // 检查chunk是否与视图矩形相交
                    !(chunk.bounds.x_max < view_rect.x_min
                        || chunk.bounds.x_min > view_rect.x_max
                        || chunk.bounds.y_max < view_rect.y_min
                        || chunk.bounds.y_min > view_rect.y_max)
                })
                .collect();

            // 绘制可见块中的线段
            for chunk in visible_chunks {
                for &segment_idx in &chunk.segments {
                    if segment_idx < self.segments.len() {
                        let segment = &self.segments[segment_idx];
                        self.draw_segment(ctx, segment, scale);
                    }
                }
            }
        } else {
            // 如果没有块数据，直接绘制所有线段
            for segment in &self.segments {
                self.draw_segment(ctx, segment, scale);
            }
        }

        // 绘制当前正在构建的线段
        if let Some(segment) = &self.current_segment {
            self.draw_segment(ctx, segment, scale);
        }

        // 绘制终点标记
        if let Some(last_point) = self.points.last() {
            ctx.set_fill_style(&last_point.get_color().as_str().into());
            ctx.begin_path();
            ctx.arc(
                last_point.x,
                last_point.y,
                3.0 / scale,
                0.0,
                2.0 * std::f64::consts::PI,
            )
            .unwrap();
            ctx.fill();
        }
    }

    // 绘制单个线段
    fn draw_segment(&self, ctx: &CanvasRenderingContext2d, segment: &PathSegment, scale: f64) {
        if segment.points.len() < 2 {
            return;
        }

        let color = format!(
            "#{:02x}{:02x}{:02x}",
            hsv_to_rgb_r(segment.color),
            hsv_to_rgb_g(segment.color),
            hsv_to_rgb_b(segment.color)
        );
        ctx.set_stroke_style(&color.as_str().into());

        ctx.begin_path();
        ctx.move_to(segment.points[0].x, segment.points[0].y);

        // 使用更高效的路径绘制方法
        if segment.points.len() > 100 {
            // 对于大量点，使用更高效的绘制策略
            let mut i = 0;
            while i < segment.points.len() {
                ctx.line_to(segment.points[i].x, segment.points[i].y);
                // 在大量点的情况下跳过一些点
                i += if segment.points.len() > 1000 { 5 } else { 2 };
            }
            // 确保最后一个点被绘制
            if i > segment.points.len() && !segment.points.is_empty() {
                let last = segment.points.last().unwrap();
                ctx.line_to(last.x, last.y);
            }
        } else {
            // 对于少量点，正常绘制
            for point in segment.points.iter().skip(1) {
                ctx.line_to(point.x, point.y);
            }
        }

        ctx.stroke();
    }

    // 优化的增量绘制函数
    fn draw_last_segment(&self, ctx: &CanvasRenderingContext2d, scale: f64) {
        if let Some(segment) = &self.current_segment {
            if segment.points.len() < 2 {
                return;
            }

            // 计算最后一个颜色
            let color = format!(
                "#{:02x}{:02x}{:02x}",
                hsv_to_rgb_r(segment.color),
                hsv_to_rgb_g(segment.color),
                hsv_to_rgb_b(segment.color)
            );

            ctx.set_stroke_style(&color.as_str().into());
            ctx.set_line_width(2.0 / scale);

            // 绘制最新的点与上一个点之间的线段
            let len = segment.points.len();
            if len >= 2 {
                let last_index = len - 1;
                let prev_index = len - 2;

                ctx.begin_path();
                ctx.move_to(segment.points[prev_index].x, segment.points[prev_index].y);
                ctx.line_to(segment.points[last_index].x, segment.points[last_index].y);
                ctx.stroke();
            }
        }

        // 重要：如果上一个线段存在且这是一个新段，绘制连接线
        if !self.segments.is_empty() && self.current_segment.is_some() {
            let last_segment = &self.segments[self.segments.len() - 1];
            let current_segment = self.current_segment.as_ref().unwrap();

            // 我们只需确保这些点是连接的，无论颜色是否变化
            if let (Some(last_point_prev), Some(first_point_current)) =
                (last_segment.points.last(), current_segment.points.first())
            {
                // 重要：始终绘制连接线，确保视觉连续性
                // 使用两种颜色各绘制一半，创造平滑过渡效果

                // 1. 先用旧颜色绘制一条短线
                let connect_color1 = format!(
                    "#{:02x}{:02x}{:02x}",
                    hsv_to_rgb_r(last_segment.color),
                    hsv_to_rgb_g(last_segment.color),
                    hsv_to_rgb_b(last_segment.color)
                );

                ctx.set_stroke_style(&connect_color1.as_str().into());
                ctx.begin_path();
                ctx.move_to(last_point_prev.x, last_point_prev.y);
                // 绘制到中点
                let mid_x = (last_point_prev.x + first_point_current.x) / 2.0;
                let mid_y = (last_point_prev.y + first_point_current.y) / 2.0;
                ctx.line_to(mid_x, mid_y);
                ctx.stroke();

                // 2. 再用新颜色绘制另一半
                let connect_color2 = format!(
                    "#{:02x}{:02x}{:02x}",
                    hsv_to_rgb_r(current_segment.color),
                    hsv_to_rgb_g(current_segment.color),
                    hsv_to_rgb_b(current_segment.color)
                );

                ctx.set_stroke_style(&connect_color2.as_str().into());
                ctx.begin_path();
                ctx.move_to(mid_x, mid_y);
                ctx.line_to(first_point_current.x, first_point_current.y);
                ctx.stroke();
            }
        }

        // 绘制当前点作为标记
        if let Some(segment) = &self.current_segment {
            if let Some(current_point) = segment.points.last() {
                let color = format!(
                    "#{:02x}{:02x}{:02x}",
                    hsv_to_rgb_r(current_point.color),
                    hsv_to_rgb_g(current_point.color),
                    hsv_to_rgb_b(current_point.color)
                );

                ctx.set_fill_style(&color.as_str().into());
                ctx.begin_path();
                ctx.arc(
                    current_point.x,
                    current_point.y,
                    3.0 / scale,
                    0.0,
                    2.0 * std::f64::consts::PI,
                )
                .unwrap();
                ctx.fill();
            }
        }
    }
}

// 改进的画布渲染函数 - 优化渲染性能
fn draw_canvas(
    ctx: &CanvasRenderingContext2d,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
    canvas: &web_sys::HtmlCanvasElement,
    path_history: &PathHistory,
    redraw_mode: RedrawMode,
) {
    // 计算当前视图的矩形区域（世界坐标）
    let view_rect = Rect {
        x_min: -offset_x / scale,
        y_min: -offset_y / scale,
        x_max: (canvas.width() as f64 - offset_x) / scale,
        y_max: (canvas.height() as f64 - offset_y) / scale,
    };

    match redraw_mode {
        RedrawMode::Full => {
            // 完全重绘 - 清除画布并绘制所有内容
            ctx.save();
            ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap();
            ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
            ctx.restore();

            // 保存当前状态
            ctx.save();

            // 应用变换
            ctx.translate(offset_x, offset_y).unwrap();
            ctx.scale(scale, scale).unwrap();

            // 1. 绘制网格和参考线
            draw_grid(ctx, scale, &view_rect);

            // 2. 绘制完整路径历史（传递可见区域信息以优化渲染）
            path_history.draw_complete_path(ctx, scale, &view_rect);

            // 恢复状态
            ctx.restore();
        }
        RedrawMode::Incremental => {
            // 增量绘制 - 只绘制最新的线段
            ctx.save();
            ctx.translate(offset_x, offset_y).unwrap();
            ctx.scale(scale, scale).unwrap();

            path_history.draw_last_segment(ctx, scale);

            ctx.restore();
        }
        RedrawMode::Navigation => {
            // 导航绘制 - 清除并重绘
            ctx.save();
            ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap();
            ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
            ctx.restore();

            ctx.save();
            ctx.translate(offset_x, offset_y).unwrap();
            ctx.scale(scale, scale).unwrap();

            // 1. 绘制网格和参考线
            draw_grid(ctx, scale, &view_rect);

            // 2. 绘制完整路径历史（传递可见区域信息）
            path_history.draw_complete_path(ctx, scale, &view_rect);

            ctx.restore();
        }
    }
}

// 优化的网格绘制函数
fn draw_grid(ctx: &CanvasRenderingContext2d, scale: f64, view_rect: &Rect) {
    ctx.set_stroke_style(&"#d0d0d0".into());
    ctx.set_line_width(0.5 / scale);

    let grid_size = 10.0;

    // 只绘制可见区域的网格线，而不是固定数量
    let min_x = (view_rect.x_min / grid_size).floor() as i32;
    let max_x = (view_rect.x_max / grid_size).ceil() as i32;
    let min_y = (view_rect.y_min / grid_size).floor() as i32;
    let max_y = (view_rect.y_max / grid_size).ceil() as i32;

    // 限制线条数量以避免性能问题
    let max_lines = 100;
    let x_step = if max_x - min_x > max_lines {
        (max_x - min_x) / max_lines
    } else {
        1
    };
    let y_step = if max_y - min_y > max_lines {
        (max_y - min_y) / max_lines
    } else {
        1
    };

    // 绘制垂直线
    for i in (min_x..=max_x).step_by(x_step as usize) {
        ctx.begin_path();
        ctx.move_to(i as f64 * grid_size, view_rect.y_min);
        ctx.line_to(i as f64 * grid_size, view_rect.y_max);
        ctx.stroke();
    }

    // 绘制水平线
    for i in (min_y..=max_y).step_by(y_step as usize) {
        ctx.begin_path();
        ctx.move_to(view_rect.x_min, i as f64 * grid_size);
        ctx.line_to(view_rect.x_max, i as f64 * grid_size);
        ctx.stroke();
    }
}

// 绘制模式枚举
enum RedrawMode {
    Full,        // 完全重绘（清除并绘制所有内容）
    Incremental, // 增量绘制（只绘制新内容）
    Navigation,  // 导航绘制（用于缩放/平移）
}

// 优化的存储函数，分块保存和加载
fn save_path_history(path_history: &PathHistory) {
    // 确保当前线段已添加到segments中
    let mut segments = path_history.segments.clone();
    if let Some(segment) = &path_history.current_segment {
        segments.push(segment.clone());
    }

    // 对于大量数据，分片存储
    if segments.len() > 50 {
        // 每批最多存储50个线段
        let batch_count = (segments.len() + 49) / 50;

        for batch in 0..batch_count {
            let start = batch * 50;
            let end = std::cmp::min(start + 50, segments.len());
            let batch_segments = &segments[start..end];

            let (_, set_segments, _) = use_storage::<Vec<PathSegment>, JsonSerdeCodec>(
                leptos_use::storage::StorageType::Local,
                format!("path_segments_batch_{}", batch).as_str(),
            );
            set_segments.set(batch_segments.to_vec());
        }

        // 存储批次数量
        let (_, set_batch_count, _) = use_storage::<usize, JsonSerdeCodec>(
            leptos_use::storage::StorageType::Local,
            "path_segments_batch_count",
        );
        set_batch_count.set(batch_count);
    } else {
        // 数据量小，直接存储
        let (_, set_segments, _) = use_storage::<Vec<PathSegment>, JsonSerdeCodec>(
            leptos_use::storage::StorageType::Local,
            "path_segments",
        );
        set_segments.set(segments);

        // 清除批次计数
        let (_, set_batch_count, _) = use_storage::<usize, JsonSerdeCodec>(
            leptos_use::storage::StorageType::Local,
            "path_segments_batch_count",
        );
        set_batch_count.set(0);
    }
}

// 从本地存储加载路径历史
fn load_path_history() -> PathHistory {
    let (batch_count_signal, _, _) = use_storage::<usize, JsonSerdeCodec>(
        leptos_use::storage::StorageType::Local,
        "path_segments_batch_count",
    );
    let batch_count = batch_count_signal.get_untracked();

    let mut history = PathHistory::new();

    if batch_count > 0 {
        // 分批加载
        for batch in 0..batch_count {
            let (segments_signal, _, _) = use_storage::<Vec<PathSegment>, JsonSerdeCodec>(
                leptos_use::storage::StorageType::Local,
                format!("path_segments_batch_{}", batch).as_str(),
            );
            let segments = segments_signal.get_untracked();
            history.segments.extend(segments);

            // 重建点列表
            for segment in &history.segments {
                history.points.extend(segment.points.clone());
            }
        }
    } else {
        // 直接加载
        let (segments_signal, _, _) = use_storage::<Vec<PathSegment>, JsonSerdeCodec>(
            leptos_use::storage::StorageType::Local,
            "path_segments",
        );
        history.segments = segments_signal.get_untracked();

        // 重建点列表
        for segment in &history.segments {
            history.points.extend(segment.points.clone());
        }
    }

    // 重建空间索引
    if !history.segments.is_empty() {
        history.rebuild_spatial_chunks();
    }

    history
}

#[component]
fn PointVisual() -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let context = Rc::new(RefCell::new(None));

    // 使用RefCell保存路径历史，以便在不同闭包中修改
    let path_history = Rc::new(RefCell::new(PathHistory::new()));

    // 存储最新的移动状态
    let current_status = RwSignal::new(MoveStatus::default());

    // 视图变换状态
    let scale = RwSignal::new(1.0);
    let offset_x = RwSignal::new(200.0);
    let offset_y = RwSignal::new(200.0);

    // 鼠标交互状态
    let dragging = RwSignal::new(false);
    let last_mouse_x = RwSignal::new(0);
    let last_mouse_y = RwSignal::new(0);

    // 连接到WebSocket的移动状态信号
    let move_status =
        leptos_ws::ServerSignal::new("move_status".to_string(), MoveStatus::default()).unwrap();

    // 初始化画布和加载历史
    let context_clone = context.clone();
    let path_history_clone = path_history.clone();

    Effect::new(move || {
        if canvas_ref.get().is_some() && context_clone.borrow().is_none() {
            logging::log!("Initializing canvas...");

            // 获取Canvas上下文
            let canvas = canvas_ref.get().unwrap();
            let ctx = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap();

            // 设置线条样式
            ctx.set_line_cap("round");
            ctx.set_line_join("round");

            // 存储上下文
            context_clone.borrow_mut().replace(ctx);

            // 加载历史路径
            *path_history_clone.borrow_mut() = load_path_history();

            // 初始渲染
            if let Some(ctx) = context_clone.borrow().as_ref() {
                draw_canvas(
                    ctx,
                    scale.get(),
                    offset_x.get(),
                    offset_y.get(),
                    &canvas,
                    &path_history_clone.borrow(),
                    RedrawMode::Full,
                );
            }
        }
    });

    // 处理移动状态更新
    let context_clone = context.clone();
    let path_history_clone = path_history.clone();

    Effect::new(move || {
        let status = move_status.get();
        current_status.set(status.clone());

        if let Some(canvas) = canvas_ref.get() {
            if let Some(ctx) = context_clone.borrow().as_ref() {
                let mut path = path_history_clone.borrow_mut();

                // 只有当点不同时才添加新点
                let is_new_point = path.points.is_empty()
                    || path
                        .points
                        .last()
                        .map(|point| {
                            point.x != status.x.pos as f64 || point.y != status.y.pos as f64
                        })
                        .unwrap_or(true);

                if is_new_point {
                    // 计算当前Z轴位置作为颜色
                    let current_color = (5.0 - status.z.pos).clamp(0.0, 5.0) as u8 * (255 / 5);

                    // 添加新点到路径历史
                    path.add_point(status.x.pos as f64, status.y.pos as f64, current_color);

                    // 每次添加点后，确保完整绘制当前线段
                    // 这样可以解决颜色过渡时的断线问题
                    let redraw_mode = if path.points.len() <= 2 {
                        // 第一个或第二个点时完全重绘
                        RedrawMode::Full
                    } else {
                        // 增量绘制包含连接线
                        RedrawMode::Incremental
                    };

                    draw_canvas(
                        ctx,
                        scale.get(),
                        offset_x.get(),
                        offset_y.get(),
                        &canvas,
                        &path,
                        redraw_mode,
                    );

                    // 定期保存历史
                    if path.points.len() % 10 == 0 {
                        save_path_history(&path);
                    }
                }
            }
        }
    });

    // 鼠标滚轮缩放
    let context_clone = context.clone();
    let path_history_clone = path_history.clone();

    let handle_wheel = move |ev: web_sys::WheelEvent| {
        ev.prevent_default();
        let delta = if ev.delta_y() > 0.0 { 0.9 } else { 1.1 };

        if let Some(canvas) = canvas_ref.get() {
            if let Some(ctx) = context_clone.borrow().as_ref() {
                let rect = canvas.get_bounding_client_rect();
                let mouse_x = ev.client_x() as f64 - rect.left();
                let mouse_y = ev.client_y() as f64 - rect.top();

                // 计算缩放前鼠标在世界坐标中的位置
                let world_x = (mouse_x - offset_x.get()) / scale.get();
                let world_y = (mouse_y - offset_y.get()) / scale.get();

                // 更新缩放
                scale.update(|s| *s *= delta);

                // 调整偏移量以保持鼠标位置不变
                offset_x.set(mouse_x - world_x * scale.get());
                offset_y.set(mouse_y - world_y * scale.get());

                // 重绘使用导航模式
                draw_canvas(
                    ctx,
                    scale.get(),
                    offset_x.get(),
                    offset_y.get(),
                    &canvas,
                    &path_history_clone.borrow(),
                    RedrawMode::Navigation,
                );
            }
        }
    };

    // 鼠标拖拽平移
    let context_clone = context.clone();
    let path_history_clone = path_history.clone();

    let handle_mouse_down = move |ev: web_sys::MouseEvent| {
        dragging.set(true);
        last_mouse_x.set(ev.client_x());
        last_mouse_y.set(ev.client_y());
    };

    let handle_mouse_move = move |ev: web_sys::MouseEvent| {
        if dragging.get() {
            if let Some(canvas) = canvas_ref.get() {
                if let Some(ctx) = context_clone.borrow().as_ref() {
                    let dx = ev.client_x() - last_mouse_x.get();
                    let dy = ev.client_y() - last_mouse_y.get();

                    offset_x.update(|x| *x += dx as f64);
                    offset_y.update(|y| *y += dy as f64);

                    last_mouse_x.set(ev.client_x());
                    last_mouse_y.set(ev.client_y());

                    // 使用导航模式重绘
                    draw_canvas(
                        ctx,
                        scale.get(),
                        offset_x.get(),
                        offset_y.get(),
                        &canvas,
                        &path_history_clone.borrow(),
                        RedrawMode::Navigation,
                    );
                }
            }
        }
    };

    let handle_mouse_up = move |_| {
        dragging.set(false);
    };

    // 重置视图
    let context_clone = context.clone();
    let path_history_clone = path_history.clone();

    let reset_view = move |_| {
        if let Some(canvas) = canvas_ref.get() {
            if let Some(ctx) = context_clone.borrow().as_ref() {
                scale.set(1.0);
                offset_x.set(200.0);
                offset_y.set(200.0);

                draw_canvas(
                    ctx,
                    scale.get(),
                    offset_x.get(),
                    offset_y.get(),
                    &canvas,
                    &path_history_clone.borrow(),
                    RedrawMode::Full,
                );
            }
        }
    };

    // 清除路径
    let context_clone = context.clone();
    let path_history_clone = path_history.clone();

    let clear_path = move |_| {
        if let Some(canvas) = canvas_ref.get() {
            if let Some(ctx) = context_clone.borrow().as_ref() {
                // 完全清空路径历史
                let mut history = path_history_clone.borrow_mut();
                history.points.clear();
                history.segments.clear();
                history.current_segment = None;
                history.chunks.clear();

                // 重绘空画布
                draw_canvas(
                    ctx,
                    scale.get(),
                    offset_x.get(),
                    offset_y.get(),
                    &canvas,
                    &history,
                    RedrawMode::Full,
                );

                // 清除本地存储中所有相关数据
                let (batch_count, set_batch_count, _) = use_storage::<usize, JsonSerdeCodec>(
                    leptos_use::storage::StorageType::Local,
                    "path_segments_batch_count",
                );
                let batch_count = batch_count.get_untracked();

                // 清除所有批次
                for batch in 0..batch_count {
                    let (_, set_segments, _) = use_storage::<Vec<PathSegment>, JsonSerdeCodec>(
                        leptos_use::storage::StorageType::Local,
                        format!("path_segments_batch_{}", batch).as_str(),
                    );
                    set_segments.set(Vec::new());
                }

                // 清除主存储
                let (_, set_segments, _) = use_storage::<Vec<PathSegment>, JsonSerdeCodec>(
                    leptos_use::storage::StorageType::Local,
                    "path_segments",
                );
                set_segments.set(Vec::new());

                // 重置批次计数
                set_batch_count.set(0);
            }
        }
    };

    // 保存路径
    let path_history_clone = path_history.clone();

    let save_path = move |_| {
        logging::log!("Manually saving path history");
        save_path_history(&path_history_clone.borrow());
    };

    view! {
        <div class="canvas-controls">
            <input
                type="range"
                min="0.1"
                max="5.0"
                step="0.1"
                value="1.0"
                on:input=move |ev| {
                    if let Ok(value) = event_target_value(&ev).parse::<f64>() {
                        if let Some(canvas) = canvas_ref.get() {
                            if let Some(ctx) = context.borrow().as_ref() {
                                scale.set(value);
                                draw_canvas(
                                    ctx,
                                    scale.get(),
                                    offset_x.get(),
                                    offset_y.get(),
                                    &canvas,
                                    &path_history.borrow(),
                                    RedrawMode::Navigation,
                                );
                            }
                        }
                    }
                }
            />
            <span class="zoom-info">{move || format!("Zoom: {:.1}x", scale.get())}</span>
            <button on:click=save_path>"Save Path"</button>
            <button on:click=clear_path>"Clear Path"</button>
            <button on:click=reset_view>"Reset View"</button>
            <span class="position-info">
                {move || {
                    format!(
                        "X: {:.2}, Y: {:.2}",
                        current_status.get().x.pos,
                        current_status.get().y.pos,
                    )
                }}
            </span>
        </div>
        <canvas
            width="400"
            height="400"
            style="border: 1px solid black; cursor: move; background-color: #fafafa;"
            node_ref=canvas_ref
            on:mousedown=handle_mouse_down
            on:mousemove=handle_mouse_move
            on:mouseup=handle_mouse_up
            on:mouseleave=handle_mouse_up
            on:wheel=handle_wheel
        />
    }
}

#[component]
fn AxisVisual() -> impl IntoView {
    let (global_state, set_global_state) =
        use_cookie::<GlobalState, JsonSerdeCodec>("global_state_cookie");
    // Ensure global state is initialized
    if global_state.read_untracked().is_none() {
        set_global_state.set(Some(GlobalState::default()));
    }

    let connected = move || global_state.get().unwrap().connected;

    let move_status =
        leptos_ws::ServerSignal::new("move_status".to_string(), MoveStatus::default()).unwrap();

    view! {
        <Transition fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            <div class="axis-status">
                {move || {
                    if !connected() {
                        view! { <div class="error-message">"Not connected"</div> }
                    } else {
                        let status = move_status.get();
                        view! {
                            <div class="axis-status-container">
                                <Table class="axis-status-table">
                                    <TableHeader>
                                        <TableRow>
                                            <TableCell>"    "</TableCell>
                                            <TableCell>
                                                <h3>"X Axis"</h3>
                                            </TableCell>
                                            <TableCell>
                                                <h3>"X Axis"</h3>
                                            </TableCell>
                                            <TableCell>
                                                <h3>"X Axis"</h3>
                                            </TableCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        <TableRow>
                                            <TableCell>"Idle"</TableCell>
                                            <TableCell>
                                                {if status.x.is_idle { "Yes" } else { "No" }}
                                            </TableCell>
                                            <TableCell>
                                                {if status.y.is_idle { "Yes" } else { "No" }}
                                            </TableCell>
                                            <TableCell>
                                                {if status.z.is_idle { "Yes" } else { "No" }}
                                            </TableCell>
                                        </TableRow>
                                        <TableRow>
                                            <TableCell>"Speed"</TableCell>
                                            <TableCell>{format!("{:.2}", status.x.speed)}</TableCell>
                                            <TableCell>{format!("{:.2}", status.y.speed)}</TableCell>
                                            <TableCell>{format!("{:.2}", status.z.speed)}</TableCell>
                                        </TableRow>
                                        <TableRow>
                                            <TableCell>"Position"</TableCell>
                                            <TableCell>{format!("{:.3}", status.x.pos)}</TableCell>
                                            <TableCell>{format!("{:.3}", status.y.pos)}</TableCell>
                                            <TableCell>{format!("{:.3}", status.z.pos)}</TableCell>
                                        </TableRow>
                                    </TableBody>
                                </Table>
                            </div>
                        }
                    }
                }}
            </div>
        </Transition>
    }
}

#[component]
pub fn VisualView() -> impl IntoView {
    view! {
        <div class="status">
            <AxisVisual />
            <PathVisualizer />
        // <PointVisual />
        </div>
    }
}
