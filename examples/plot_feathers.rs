//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    ecs::template::TemplateContext,
    feathers::{
        FeathersPlugins,
        constants::icons,
        containers::*,
        controls::*,
        dark_theme::create_dark_theme,
        display::{caption, icon, label, label_dim},
        theme::UiTheme,
    },
    prelude::*,
    ui_widgets::ValueChange,
};
use bevy_waveform::{
    ViewportRange, WaveformDecodeF32, WaveformMaterial, WaveformParams, WaveformPlugin,
    WaveformSource, WaveformStyle, on_waveform_drag_end, on_waveform_drag_start,
    on_waveform_scroll,
};

/// A struct to hold the state of various widgets shown in the demo.
#[derive(Resource)]
struct DemoWidgetStates {
    scalar_prop: f32,
}

#[derive(Component, Clone, Copy, Default)]
struct DemoScalarField;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .add_plugins(WaveformPlugin::<AudioData>::new())
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(DemoWidgetStates { scalar_prop: 7.0 })
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list! {
        Camera2d
        --
        @demo_column_2()
        --
    }
}

fn demo_column_2() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(30),
            min_width: px(200),
        }
        Children [
            @pane() Children [
                @pane_header() Children [
                    @FeathersToolButton {
                        @variant: ButtonVariant::Primary,
                        @caption: bsn! { @caption("\u{0398}") }
                    }
                    --
                    @pane_header_divider()
                    --
                    @label("Audio Waveform")
                    --
                    @flex_spacer()
                    --
                    @FeathersToolButton {
                        @variant: ButtonVariant::Plain,
                        @caption: bsn! { @icon(icons::X) }
                    }
                ]
                --
                @pane_body() Children [
                    @label_dim("A standard editor pane")
                    --
                    @subpane() Children [
                        Node
                        Children [
                            @audio(px(400),px(200))
                        ]
                        --
                        @FeathersNumberInput
                        DemoScalarField
                        NumberInputPrecision(2)
                        HardLimit::f32(0.0..=100.0)
                        Node {
                            flex_grow: 1.0,
                            max_width: px(100),
                        }
                        on(|value_change: On<ValueChange<f32>>, mut states: ResMut<DemoWidgetStates>| {
                            states.scalar_prop = value_change.value;
                        })
                    ]
                ]
            ]
            --
        ]
    }
}

#[derive(Component)]
pub struct AudioData(WaveformDecodeF32);

impl WaveformSource for AudioData {
    fn channel_samples(&self) -> &[f32] {
        self.0.channel(0)
    }
}

pub const AUDIO: &str = r".\assets\audio.wav";

fn audio(width: Val, height: Val) -> impl Scene {
    let test = template(|context: &mut TemplateContext| {
        let audio = WaveformDecodeF32::from_path(AUDIO, None).unwrap();
        let total = audio.channel(0).len();

        let range = ViewportRange::full(total);
        let style = WaveformStyle::default();

        let image = audio.waveform_texture(0, &range, style.texture_width);
        let mut images = context.resource_mut::<Assets<Image>>();
        let tex = images.add(image);

        let mut ui_materials = context.resource_mut::<Assets<WaveformMaterial>>();
        let mat = ui_materials.add(WaveformMaterial {
            waveform: tex.clone(),
            params: WaveformParams::build_params(&style),
        });

        context
            .entity
            .insert((MaterialNode(mat), style, range))
            .observe(on_waveform_drag_start)
            .observe(on_waveform_drag_end)
            .observe(on_waveform_scroll);

        Ok(AudioData(audio))
    });

    bsn! {
        Node {
            display: Display::Flex,
            width,
            height,
            justify_self: JustifySelf::Center,
            align_self: AlignSelf::Center,
        }
        test
    }
}
