use std::ops::RangeInclusive;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_tnua::{TnuaFreeFallBehavior, TnuaPlatformerConfig};

use super::ui_plotting::PlotSource;
use super::FallingThroughControlScheme;

pub struct ExampleUi;

impl Plugin for ExampleUi {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        app.insert_resource(ExampleUiTnuaActive(true));
        app.add_system(ui_system);
        app.add_system(super::ui_plotting::plot_source_rolling_update);
    }
}

// NOTE: The examples are responsible for taking this into account
#[derive(Resource)]
pub struct ExampleUiTnuaActive(pub bool);

#[derive(Component)]
pub struct TrackedEntity(pub String);

#[derive(Component)]
pub struct CommandAlteringSelectors(Vec<CommandAlteringSelector>);

impl Default for CommandAlteringSelectors {
    fn default() -> Self {
        Self(Default::default())
    }
}

enum CommandAlteringSelector {
    Combo {
        chosen: usize,
        caption: String,
        options: Vec<(String, fn(EntityCommands))>,
        set_to: Option<usize>,
    },
    Checkbox {
        checked: bool,
        caption: String,
        applier: fn(EntityCommands, bool),
        set_to: Option<bool>,
    },
}

impl CommandAlteringSelectors {
    pub fn with_combo(
        mut self,
        caption: &str,
        initial: usize,
        options: &[(&str, fn(EntityCommands))],
    ) -> Self {
        self.0.push(CommandAlteringSelector::Combo {
            chosen: 0,
            caption: caption.to_owned(),
            options: options
                .into_iter()
                .map(|(name, applier)| (name.to_string(), *applier))
                .collect(),
            set_to: Some(initial),
        });
        self
    }

    pub fn with_checkbox(
        mut self,
        caption: &str,
        initial: bool,
        applier: fn(EntityCommands, bool),
    ) -> Self {
        self.0.push(CommandAlteringSelector::Checkbox {
            checked: false,
            caption: caption.to_owned(),
            applier,
            set_to: Some(initial),
        });
        self
    }
}

fn slider_or_infinity(
    ui: &mut egui::Ui,
    caption: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
) {
    #[derive(Clone)]
    struct CachedValue(f32);

    ui.horizontal(|ui| {
        let mut infinite = !value.is_finite();
        let resp = ui.toggle_value(&mut infinite, "\u{221e}");
        if resp.clicked() {
            if infinite {
                ui.memory_mut(|memory| memory.data.insert_temp(resp.id, CachedValue(*value)));
                *value = f32::INFINITY
            } else {
                if let Some(CachedValue(saved_value)) =
                    ui.memory_mut(|memory| memory.data.get_temp(resp.id))
                {
                    *value = saved_value;
                } else {
                    *value = *range.end();
                }
            }
        }
        if infinite {
            let mut copied_saved_value = ui.memory_mut(|memory| {
                let CachedValue(saved_value) = memory
                    .data
                    .get_temp_mut_or(resp.id, CachedValue(*range.end()));
                *saved_value
            });
            ui.add_enabled(
                false,
                egui::Slider::new(&mut copied_saved_value, range).text(caption),
            );
        } else {
            ui.add(egui::Slider::new(value, range).text(caption));
        }
    });
}

fn slider_or_none(
    ui: &mut egui::Ui,
    caption: &str,
    value: &mut Option<f32>,
    range: RangeInclusive<f32>,
) {
    #[derive(Clone)]
    struct CachedValue(f32);

    ui.horizontal(|ui| {
        let mut is_none = value.is_none();
        let resp = ui.toggle_value(&mut is_none, "\u{d8}");
        if resp.clicked() {
            if is_none {
                ui.memory_mut(|memory| memory.data.insert_temp(resp.id, CachedValue(value.expect("checkbox was clicked, and is_none is now true, so previously it was false, which means value should not be None"))));
                *value = None;
            } else {
                if let Some(CachedValue(saved_value)) =
                    ui.memory_mut(|memory| memory.data.get_temp(resp.id))
                {
                    *value = Some(saved_value);
                } else {
                    *value = Some(*range.start());
                }
            }
        }
        if let Some(value) = value.as_mut() {
            ui.add(egui::Slider::new(value, range).text(caption));
        } else {
            let mut copied_saved_value = ui.memory_mut(|memory| {
                let CachedValue(saved_value) = memory
                    .data
                    .get_temp_mut_or(resp.id, CachedValue(*range.start()));
                *saved_value
            });
            ui.add_enabled(
                false,
                egui::Slider::new(&mut copied_saved_value, range).text(caption),
            );
        }
    });
}

fn ui_system(
    mut egui_context: EguiContexts,
    mut tnua_active: ResMut<ExampleUiTnuaActive>,
    mut query: Query<(
        Entity,
        &TrackedEntity,
        &PlotSource,
        &mut TnuaPlatformerConfig,
        &mut FallingThroughControlScheme,
        Option<&mut CommandAlteringSelectors>,
    )>,
    mut commands: Commands,
) {
    for (entity, .., command_altering_selectors) in query.iter_mut() {
        if let Some(mut command_altering_selectors) = command_altering_selectors {
            for selector in command_altering_selectors.0.iter_mut() {
                match selector {
                    CommandAlteringSelector::Combo {
                        chosen,
                        caption: _,
                        options,
                        set_to,
                    } => {
                        if let Some(set_to) = set_to.take() {
                            *chosen = set_to;
                            options[set_to].1(commands.entity(entity));
                        }
                    }
                    CommandAlteringSelector::Checkbox {
                        checked,
                        caption: _,
                        applier,
                        set_to,
                    } => {
                        if let Some(set_to) = set_to.take() {
                            *checked = set_to;
                            applier(commands.entity(entity), set_to);
                        }
                    }
                }
            }
        }
    }
    egui::Window::new("Tnua").show(egui_context.ctx_mut(), |ui| {
        egui::CollapsingHeader::new("Controls:")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Move with the arrow keys");
                ui.label("Jump with Spacebar (Also with the up arrow also works in 2D)");
                ui.label("Crouch or fall through pink platforms with Ctrl (Also with the down arrow key in 2D)");
                ui.label("Turn in place with Alt");
            });
        ui.checkbox(&mut tnua_active.0, "Tnua Enabled (does not affect the physics backend itself)");
        for (
            entity,
            TrackedEntity(name),
            plot_source,
            mut platformer_config,
            mut falling_through_control_scheme,
            command_altering_selectors,
        ) in query.iter_mut()
        {
            egui::CollapsingHeader::new(name)
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Slider::new(&mut platformer_config.full_speed, 0.0..=60.0)
                                    .text("Speed"),
                            );
                            ui.add(
                                egui::Slider::new(&mut platformer_config.full_jump_height, 0.0..=10.0)
                                    .text("Jump Height"),
                            );
                            platformer_config.full_jump_height = platformer_config.full_jump_height.max(0.1);

                            if let Some(mut command_altering_selectors) = command_altering_selectors
                            {
                                for selector in command_altering_selectors.0.iter_mut() {
                                    match selector {
                                        CommandAlteringSelector::Combo { chosen, caption, options, set_to: _ } => {
                                            let mut selected_idx: usize = *chosen;
                                            egui::ComboBox::from_label(caption.as_str())
                                                .selected_text(&options[*chosen].0)
                                                .show_ui(ui, |ui| {
                                                    for (idx, (name, _)) in options.iter().enumerate() {
                                                        ui.selectable_value(&mut selected_idx, idx, name);
                                                    }
                                                });
                                            if selected_idx != *chosen {
                                                options[selected_idx].1(commands.entity(entity));
                                                *chosen = selected_idx;
                                            }
                                        }
                                        CommandAlteringSelector::Checkbox { checked, caption, applier, set_to: _ } => {
                                            if ui.checkbox(checked, caption.as_str()).clicked() {
                                                applier(commands.entity(entity), *checked);
                                            }
                                        }
                                    }
                                }
                            }

                            ui.add(
                                egui::Slider::new(&mut platformer_config.float_height, 0.0..=10.0)
                                    .text("Float At"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.cling_distance,
                                    0.0..=10.0,
                                )
                                .text("Cling Distance"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.spring_strengh,
                                    0.0..=4000.0,
                                )
                                .text("Spring Strengh"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.spring_dampening,
                                    0.0..=1.9,
                                )
                                .text("Spring Dampening"),
                            );
                            slider_or_infinity(ui, "Acceleration", &mut platformer_config.acceleration, 0.0..=200.0);
                            slider_or_infinity(ui, "Air Acceleration", &mut platformer_config.air_acceleration, 0.0..=200.0);
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.coyote_time,
                                    0.0..=1.0,
                                )
                                .text("Coyote Time"),
                            );
                            ui.add(egui::Slider::new(&mut platformer_config.jump_input_buffer_time, 0.0..=1.0).text("Jump Input Buffer Time"));
                            slider_or_none(ui, "Held Jump Cooldown", &mut platformer_config.held_jump_cooldown, 0.0..=2.0);
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.upslope_jump_extra_gravity,
                                    0.0..=100.0,
                                )
                                .text("Upslope Jump Extra Gravity"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_takeoff_extra_gravity,
                                    0.0..=100.0,
                                )
                                .text("Jump Takeoff Extra Gravity"),
                            );
                            slider_or_infinity(ui, "Jump Takeoff Above Velocity", &mut platformer_config.jump_takeoff_above_velocity, 0.0..=20.0);
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_fall_extra_gravity,
                                    0.0..=50.0,
                                )
                                .text("Jump Fall Extra Gravity"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_shorten_extra_gravity,
                                    0.0..=100.0,
                                )
                                .text("Jump Shorten Extra Gravity"),
                            );

                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_peak_prevention_at_upward_velocity,
                                    0.0..=20.0,
                                )
                                .text("Jump Peak Prevention At Upward Velocity"),
                            );

                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_peak_prevention_extra_gravity,
                                    0.0..=100.0,
                                )
                                .text("Jump Peak Prevention Extra Gravity"),
                            );

                            let free_fall_options: [(bool, &str, fn() -> TnuaFreeFallBehavior); 3] = [
                                (
                                    matches!(platformer_config.free_fall_behavior, TnuaFreeFallBehavior::ExtraGravity(_)),
                                    "Extra Gravity",
                                    || TnuaFreeFallBehavior::ExtraGravity(0.0),
                                ),
                                (
                                    matches!(platformer_config.free_fall_behavior, TnuaFreeFallBehavior::LikeJumpShorten),
                                    "Like Jump Shorten",
                                    || TnuaFreeFallBehavior::LikeJumpShorten,
                                ),
                                (
                                    matches!(platformer_config.free_fall_behavior, TnuaFreeFallBehavior::LikeJumpFall),
                                    "Like Jump Fall",
                                    || TnuaFreeFallBehavior::LikeJumpFall,
                                ),
                            ];
                            egui::ComboBox::from_label("Free Fall Behavior")
                                .selected_text(free_fall_options.iter().find_map(|(chosen, name, _)| chosen.then_some(*name)).unwrap_or("???"))
                                .show_ui(ui, |ui| {
                                    for (chosen, name, make_variant) in free_fall_options {
                                        if ui.selectable_label(chosen, name).clicked() {
                                             platformer_config.free_fall_behavior = make_variant();
                                        }
                                    }
                                });
                            if let TnuaFreeFallBehavior::ExtraGravity(extra_gravity) = &mut platformer_config.free_fall_behavior {
                                ui.add(
                                    egui::Slider::new(extra_gravity, 0.0..=100.0).text("Extra Gravity"),
                                );
                            }

                            slider_or_infinity(ui, "Staying Upward Max Angular Velocity", &mut platformer_config.tilt_offset_angvel, 0.0..=20.0);
                            slider_or_infinity(ui, "Staying Upward Max Angular Acceleration", &mut platformer_config.tilt_offset_angacl, 0.0..=2000.0);

                            slider_or_infinity(ui, "Turning Angular Velocity", &mut platformer_config.turning_angvel, 0.0..=70.0);

                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.height_change_impulse_for_duration,
                                    0.001..=0.2,
                                ).text("Height Change Impulse for Duration"),
                            );

                            slider_or_infinity(ui, "Height Change Impulse", &mut platformer_config.height_change_impulse_limit, 0.0..=40.0);

                            falling_through_control_scheme.edit_with_ui(ui);
                        });
                        ui.vertical(|ui| {
                            plot_source.show(entity, ui);
                        });
                    });
                });
        }
    });
}
