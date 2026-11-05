use std::marker::PhantomData;

use bevy::app::{App, Plugin, PostUpdate, Update};
use bevy::asset::{Assets, load_internal_asset};
use bevy::camera::primitives::Aabb;
use bevy::camera::{Camera, NormalizedRenderTarget, RenderTarget};
use bevy::ecs::component::Component;
use bevy::ecs::entity::ContainsEntity;
use bevy::ecs::message::MessageReader;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Changed, Or, With};
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::image::Image;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit};
use bevy::math::Vec2;
use bevy::picking::events::{PointerDragEnd, PointerDragStart, PointerScroll, PointerState};
use bevy::picking::pointer::{PointerButton, PointerId};
use bevy::shader::Shader;
use bevy::sprite_render::{Material2dPlugin, MeshMaterial2d};
use bevy::transform::components::GlobalTransform;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::ui_render::UiMaterialPlugin;
use bevy::ui_render::ui_material::MaterialNode;

use crate::{
    ViewportRange, WAVEFORM_SHADER_HANDLE, WAVEFORM_UI_SHADER_HANDLE, WaveformImage,
    WaveformMaterial, WaveformParams, WaveformRangeDrag, WaveformStyle,
};

/// 拖拽开始：给实体打上 WaveformRangeDrag 标记
pub fn on_waveform_drag_start(drag_start: On<PointerDragStart>, mut commands: Commands) {
    commands.entity(drag_start.entity).insert(WaveformRangeDrag);
}

/// 拖拽结束：移除标记
pub fn on_waveform_drag_end(drag_end: On<PointerDragEnd>, mut commands: Commands) {
    commands
        .entity(drag_end.entity)
        .remove::<WaveformRangeDrag>();
}

pub fn on_waveform_scroll(
    scroll: On<PointerScroll>,
    mut query: Query<(
        &mut ViewportRange,
        Option<(&ComputedNode, &UiGlobalTransform)>,
        Option<(&GlobalTransform, &Aabb)>,
    )>,
    camera_query: Query<(&Camera, &GlobalTransform, &RenderTarget)>,
) {
    let Ok((mut range, compute, trans)) = query.get_mut(scroll.entity) else {
        return;
    };
    // 根据滚动单位归一化，使不同设备的缩放体验一致
    let scroll_amount = match scroll.unit {
        MouseScrollUnit::Line => scroll.y * 0.15, // 行模式，一格约15%
        MouseScrollUnit::Pixel => scroll.y * 0.002, // 像素模式，更精细
    };
    // 使用 exp 保证缩放的正负对称性，避免来回缩放产生漂移
    let zoom_factor = scroll_amount.exp();

    if let Some((compute, trans)) = compute {
        let logical_size = compute.size.x * compute.inverse_scale_factor();
        let logical_trans = trans.translation.x * compute.inverse_scale_factor();

        let min = logical_trans - logical_size / 2.0;
        let max = logical_trans + logical_size / 2.0;

        let ratio = ((scroll.pointer.position.x - min) / (max - min)).clamp(0.0, 1.0);

        // ── ratio → 焦点采样索引 ──
        let focus_index = range.min + (ratio * range.span() as f32).round() as usize;
        let focus_index = focus_index.min(range.size.saturating_sub(1));

        // ── ✅ 用焦点缩放，不是 center ──
        range.zoom_at(zoom_factor, focus_index);
    } else if let Some((transform, aabb)) = trans {
        // ✅ 通过 NormalizedRenderTarget 匹配相机
        let Some((camera, camera_transform)) = camera_query
            .iter()
            .find(|(_, _, target)| {
                let NormalizedRenderTarget::Window(window) = scroll.pointer.target else {
                    return false;
                };
                // 将相机的 RenderTarget 归一化，与指针的 target 比较
                target
                    .normalize(Some(window.entity())) // 这里可能需要传入 PrimaryWindow 实体
                    .as_ref()
                    == Some(&scroll.pointer.target)
            })
            .map(|(c, t, _)| (c, t))
        else {
            // 没有匹配的相机，退化为中心缩放
            let center = range.min + range.span() / 2;
            range.zoom_at(zoom_factor, center);
            return;
        };

        // 鼠标窗口坐标 → 世界坐标
        let window_pos = scroll.pointer.position;
        let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, window_pos) else {
            let center = range.min + range.span() / 2;
            range.zoom_at(zoom_factor, center);
            return;
        };

        // 世界坐标 → 实体本地坐标
        let inv = transform.to_matrix().inverse();
        let local = inv.transform_point3(world_pos.extend(0.0));

        let local_width = aabb.half_extents.x * 2.0;
        let world_width = local_width * transform.scale().x.abs();

        let ratio = (local.x / world_width + 0.5).clamp(0.0, 1.0);

        // ratio → 焦点采样索引（绝对索引）
        let focus_index = range.min + (ratio * range.span() as f32).round() as usize;
        let focus_index = focus_index.min(range.size.saturating_sub(1));

        range.zoom_at(zoom_factor, focus_index);
    } else {
        // 以当前视口中心为焦点进行缩放
        let center = range.min + range.span() / 2;
        range.zoom_at(zoom_factor, center);
    }
}

pub struct WaveformBasePlugin;

impl Plugin for WaveformBasePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WAVEFORM_SHADER_HANDLE,
            "shaders/waveform.wesl",
            Shader::from_wesl
        );
        load_internal_asset!(
            app,
            WAVEFORM_UI_SHADER_HANDLE,
            "shaders/waveform_ui.wesl",
            Shader::from_wesl
        );

        if !app.is_plugin_added::<Material2dPlugin<WaveformMaterial>>() {
            app.add_plugins(Material2dPlugin::<WaveformMaterial>::default());
        }

        if !app.is_plugin_added::<UiMaterialPlugin<WaveformMaterial>>() {
            app.add_plugins(UiMaterialPlugin::<WaveformMaterial>::default());
        }

        app.add_systems(Update, (update_uniform_on_view_or_style_change,))
            .add_systems(PostUpdate, on_waveform_drag_system);
    }
}

pub fn on_waveform_drag_system(
    pointer_state: Res<PointerState>,
    mut query: Query<
        (
            &mut ViewportRange,
            Option<(&ComputedNode, &UiGlobalTransform)>,
            Option<(&GlobalTransform, &Aabb)>,
        ),
        With<WaveformRangeDrag>,
    >,
    mut mouse_motion: MessageReader<MouseMotion>,
    camera_query: Query<(&Camera, &GlobalTransform)>, // ← 新增：Mesh 分支需要投影
) {
    // 1. 找出被拖拽的实体
    let Some(dragged_entity) = pointer_state
        .pointer_buttons
        .get(&(PointerId::Mouse, PointerButton::Primary))
        .and_then(|state| state.dragging.keys().next().copied())
    else {
        return;
    };

    // 2. 拿组件
    let Ok((mut range, compute, trans)) = query.get_mut(dragged_entity) else {
        return;
    };

    // 4. 统一读一次 dx（避免两个分支重复消费 MouseMotion）
    let dx: f32 = mouse_motion.read().map(|e| e.delta.x).sum();

    // 5. 统一求「节点在屏幕上的逻辑像素宽度」
    //    —— 这是统一两种拖拽速度的关键
    let node_screen_width: f32 = if let Some((compute, _)) = compute {
        // UI：物理像素 × inverse_scale_factor = 逻辑像素
        (compute.size.x * compute.inverse_scale_factor()).max(1.0)
    } else if let Some((transform, aabb)) = trans {
        let local_width = aabb.half_extents.x * 2.0;
        let world_width = local_width * transform.scale().x.abs();

        let half = world_width * 0.5;
        let center_w = transform.translation().truncate();
        let left_w = center_w - Vec2::X * half;
        let right_w = center_w + Vec2::X * half;

        // 取第一个可用相机做投影。
        // 多相机场景请按 RenderTarget / Camera.is_active 筛选出正确的那一个。
        let Some((camera, cam_transform)) = camera_query.iter().next() else {
            return;
        };
        let Ok(left_s) = camera.world_to_viewport(cam_transform, left_w.extend(0.0)) else {
            return;
        };
        let Ok(right_s) = camera.world_to_viewport(cam_transform, right_w.extend(0.0)) else {
            return;
        };
        (right_s.x - left_s.x).abs().max(1.0)
    } else {
        return;
    };

    // 6. 统一拖拽位移公式
    //    dx（屏幕逻辑像素） / node_screen_width（屏幕逻辑像素） = 拖拽比例
    //    拖拽比例 × span = 采样位移
    let samples_per_px = range.span() as f32 / node_screen_width;
    let drag_delta = -(dx * samples_per_px) as isize;
    if drag_delta != 0 {
        range.pan(drag_delta);
    }
}

pub fn update_uniform_on_view_or_style_change(
    query: Query<
        (
            &WaveformStyle,
            Option<&MaterialNode<WaveformMaterial>>,
            Option<&MeshMaterial2d<WaveformMaterial>>,
        ),
        Or<(
            Changed<WaveformStyle>,
            Changed<ViewportRange>,
            Changed<MaterialNode<WaveformMaterial>>,
            Changed<MeshMaterial2d<WaveformMaterial>>,
            With<MaterialNode<WaveformMaterial>>,
            With<MeshMaterial2d<WaveformMaterial>>,
        )>,
    >,
    mut ui_materials: ResMut<Assets<WaveformMaterial>>,
) {
    for (style, mat_node, mat_mesh) in query.iter() {
        if let Some(handle) = mat_node.map(|mat| &mat.0).or(mat_mesh.map(|mat| &mat.0))
            && let Some(mut material) = ui_materials.get_mut(handle)
        {
            material.params = WaveformParams::build_params(style);
        };
    }
}

pub struct WaveformPlugin<T: Send + Sync + 'static> {
    _mark: PhantomData<T>,
}

impl<T: Send + Sync + 'static + WaveformImage + Component> WaveformPlugin<T> {
    pub fn new() -> Self {
        Self {
            _mark: PhantomData::<T>,
        }
    }
}

impl<T: Send + Sync + 'static + WaveformImage + Component> Plugin for WaveformPlugin<T> {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WaveformBasePlugin>() {
            app.add_plugins(WaveformBasePlugin);
        }

        app.add_systems(Update, (update_texture_on_range_change::<T>,));
    }
}

pub fn update_texture_on_range_change<T: WaveformImage + Component>(
    query: Query<
        (
            &T,
            &ViewportRange,
            &WaveformStyle,
            Option<&MaterialNode<WaveformMaterial>>,
            Option<&MeshMaterial2d<WaveformMaterial>>,
        ),
        Or<(
            Changed<T>,
            Changed<WaveformStyle>,
            Changed<ViewportRange>,
            Changed<MaterialNode<WaveformMaterial>>,
            Changed<MeshMaterial2d<WaveformMaterial>>,
            With<MaterialNode<WaveformMaterial>>,
            With<MeshMaterial2d<WaveformMaterial>>,
        )>,
    >,
    mut images: ResMut<Assets<Image>>,
    ui_materials: Res<Assets<WaveformMaterial>>,
) {
    for (audio, range, style, mat_node, mat_mesh) in query.iter() {
        let Some(material) = mat_node
            .and_then(|mat| ui_materials.get(&mat.0))
            .or(mat_mesh.and_then(|mat| ui_materials.get(&mat.0)))
        else {
            return;
        };

        let Some(mut image) = images.get_mut(&material.waveform) else {
            return;
        };

        image.data = Some(audio.channel_samples(range, style.texture_width));
    }
}
