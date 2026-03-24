use std::collections::HashMap;

use crate::action::ActionKind;
use crate::condition::ConditionKind;
use crate::model::{Action, Condition, Event, EventSheet};

/// Per-entity runtime state tracked across frames.
#[derive(Debug, Default, Clone)]
pub struct EventSheetState {
    pub created: bool,
    pub timers: HashMap<usize, f64>,
    pub variables: HashMap<String, f64>,
}

/// World context passed to the executor (read-only snapshot).
pub struct EventContext<'a> {
    pub entity_id: u64,
    pub entity_x: f32,
    pub entity_y: f32,
    pub dt: f64,
    pub keys_down: &'a dyn Fn(&str) -> bool,
    pub keys_just_pressed: &'a dyn Fn(&str) -> bool,
    pub keys_just_released: &'a dyn Fn(&str) -> bool,
    pub mouse_just_pressed: &'a dyn Fn(&str) -> bool,
    pub is_colliding_with: &'a dyn Fn(&str) -> bool,
}

/// Commands produced by action execution (applied by the caller).
#[derive(Debug, Clone)]
pub enum EventCommand {
    SetPosition { entity_id: u64, x: f32, y: f32 },
    MoveAtAngle { entity_id: u64, angle_deg: f32, speed: f32 },
    MoveToward { entity_id: u64, target: String, speed: f32 },
    SetVariable { entity_id: u64, name: String, value: f64 },
    Destroy { entity_id: u64 },
    SpawnObject { entity_id: u64, prefab: String, x: f32, y: f32 },
    PlaySound { sound: String },
    PlayAnimation { entity_id: u64, anim: String },
    GoToScene { scene: String },
    Log { message: String },
}

/// Evaluate all events in a sheet for one entity. Returns commands to apply.
pub fn evaluate_event_sheet(
    sheet: &EventSheet,
    state: &mut EventSheetState,
    ctx: &EventContext,
) -> Vec<EventCommand> {
    let mut commands = Vec::new();
    eval_events(&sheet.events, state, ctx, &mut commands, 0);
    if !state.created {
        state.created = true;
    }
    commands
}

fn eval_events(
    events: &[Event],
    state: &mut EventSheetState,
    ctx: &EventContext,
    commands: &mut Vec<EventCommand>,
    base_idx: usize,
) {
    for (i, event) in events.iter().enumerate() {
        if !event.enabled {
            continue;
        }

        let idx = base_idx + i;

        let all_true = if event.conditions.is_empty() {
            true
        } else {
            event.conditions.iter().all(|c| {
                let result = eval_condition(&c.kind, state, ctx, idx);
                if c.negated { !result } else { result }
            })
        };

        if all_true {
            for action in &event.actions {
                if let Some(cmd) = exec_action(&action.kind, state, ctx) {
                    commands.push(cmd);
                }
            }
            if !event.sub_events.is_empty() {
                eval_events(&event.sub_events, state, ctx, commands, idx * 1000);
            }
        }
    }
}

fn eval_condition(
    kind: &ConditionKind,
    state: &mut EventSheetState,
    ctx: &EventContext,
    event_idx: usize,
) -> bool {
    match kind {
        ConditionKind::OnCreate => !state.created,
        ConditionKind::EveryTick => true,
        ConditionKind::OnKeyPressed { key } => (ctx.keys_just_pressed)(key),
        ConditionKind::OnKeyReleased { key } => (ctx.keys_just_released)(key),
        ConditionKind::OnKeyDown { key } => (ctx.keys_down)(key),
        ConditionKind::OnMouseClick { button } => (ctx.mouse_just_pressed)(button),
        ConditionKind::OnCollisionWith { tag } => (ctx.is_colliding_with)(tag),
        ConditionKind::IfVariable { name, op, value } => {
            let current = state.variables.get(name).copied().unwrap_or(0.0);
            op.test(current, *value)
        }
        ConditionKind::EveryNSeconds { interval } => {
            let timer = state.timers.entry(event_idx).or_insert(0.0);
            *timer += ctx.dt;
            if *timer >= *interval {
                *timer -= interval;
                true
            } else {
                false
            }
        }
    }
}

fn exec_action(
    kind: &ActionKind,
    state: &mut EventSheetState,
    ctx: &EventContext,
) -> Option<EventCommand> {
    match kind {
        ActionKind::SetPosition { x, y } => Some(EventCommand::SetPosition {
            entity_id: ctx.entity_id,
            x: *x as f32,
            y: *y as f32,
        }),
        ActionKind::MoveAtAngle { angle, speed } => Some(EventCommand::MoveAtAngle {
            entity_id: ctx.entity_id,
            angle_deg: *angle as f32,
            speed: *speed as f32,
        }),
        ActionKind::MoveToward { target, speed } => Some(EventCommand::MoveToward {
            entity_id: ctx.entity_id,
            target: target.clone(),
            speed: *speed as f32,
        }),
        ActionKind::SetVariable { name, value } => {
            state.variables.insert(name.clone(), *value);
            Some(EventCommand::SetVariable {
                entity_id: ctx.entity_id,
                name: name.clone(),
                value: *value,
            })
        }
        ActionKind::AddToVariable { name, amount } => {
            let current = state.variables.entry(name.clone()).or_insert(0.0);
            *current += amount;
            Some(EventCommand::SetVariable {
                entity_id: ctx.entity_id,
                name: name.clone(),
                value: *current,
            })
        }
        ActionKind::Destroy => Some(EventCommand::Destroy {
            entity_id: ctx.entity_id,
        }),
        ActionKind::SpawnObject { prefab, x, y } => Some(EventCommand::SpawnObject {
            entity_id: ctx.entity_id,
            prefab: prefab.clone(),
            x: *x as f32,
            y: *y as f32,
        }),
        ActionKind::PlaySound { sound } => Some(EventCommand::PlaySound {
            sound: sound.clone(),
        }),
        ActionKind::PlayAnimation { anim } => Some(EventCommand::PlayAnimation {
            entity_id: ctx.entity_id,
            anim: anim.clone(),
        }),
        ActionKind::GoToScene { scene } => Some(EventCommand::GoToScene {
            scene: scene.clone(),
        }),
        ActionKind::Log { message } => {
            log::info!("[EventSheet] {message}");
            Some(EventCommand::Log {
                message: message.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn dummy_ctx() -> EventContext<'static> {
        EventContext {
            entity_id: 1,
            entity_x: 0.0,
            entity_y: 0.0,
            dt: 1.0 / 60.0,
            keys_down: &|_| false,
            keys_just_pressed: &|k| k == "Space",
            keys_just_released: &|_| false,
            mouse_just_pressed: &|_| false,
            is_colliding_with: &|_| false,
        }
    }

    #[test]
    fn on_create_fires_once() {
        let sheet = EventSheet {
            name: "test".into(),
            events: vec![Event::new(
                vec![Condition::new(ConditionKind::OnCreate)],
                vec![Action::new(ActionKind::Log { message: "created".into() })],
            )],
        };
        let mut state = EventSheetState::default();
        let ctx = dummy_ctx();

        let cmds = evaluate_event_sheet(&sheet, &mut state, &ctx);
        assert_eq!(cmds.len(), 1); // fires first time

        let cmds = evaluate_event_sheet(&sheet, &mut state, &ctx);
        assert_eq!(cmds.len(), 0); // does not fire again
    }

    #[test]
    fn every_tick_always_fires() {
        let sheet = EventSheet {
            name: "test".into(),
            events: vec![Event::new(
                vec![Condition::new(ConditionKind::EveryTick)],
                vec![Action::new(ActionKind::Log { message: "tick".into() })],
            )],
        };
        let mut state = EventSheetState::default();
        let ctx = dummy_ctx();

        for _ in 0..5 {
            let cmds = evaluate_event_sheet(&sheet, &mut state, &ctx);
            assert_eq!(cmds.len(), 1);
        }
    }

    #[test]
    fn key_pressed_condition() {
        let sheet = EventSheet {
            name: "test".into(),
            events: vec![Event::new(
                vec![Condition::new(ConditionKind::OnKeyPressed {
                    key: "Space".into(),
                })],
                vec![Action::new(ActionKind::Log { message: "jump".into() })],
            )],
        };
        let mut state = EventSheetState::default();
        let ctx = dummy_ctx(); // keys_just_pressed returns true for "Space"

        let cmds = evaluate_event_sheet(&sheet, &mut state, &ctx);
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn variable_set_and_compare() {
        let sheet = EventSheet {
            name: "test".into(),
            events: vec![
                // Set health = 100
                Event::new(
                    vec![Condition::new(ConditionKind::OnCreate)],
                    vec![Action::new(ActionKind::SetVariable {
                        name: "health".into(),
                        value: 100.0,
                    })],
                ),
                // If health > 50, log "alive"
                Event::new(
                    vec![Condition::new(ConditionKind::IfVariable {
                        name: "health".into(),
                        op: crate::condition::CompareOp::Greater,
                        value: 50.0,
                    })],
                    vec![Action::new(ActionKind::Log { message: "alive".into() })],
                ),
            ],
        };
        let mut state = EventSheetState::default();
        let ctx = dummy_ctx();

        let cmds = evaluate_event_sheet(&sheet, &mut state, &ctx);
        // OnCreate fires: SetVariable + IfVariable > 50 is true (100 > 50): Log
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn negated_condition() {
        let sheet = EventSheet {
            name: "test".into(),
            events: vec![Event::new(
                vec![Condition::negated(ConditionKind::OnKeyDown {
                    key: "Escape".into(),
                })],
                vec![Action::new(ActionKind::Log {
                    message: "not pressing escape".into(),
                })],
            )],
        };
        let mut state = EventSheetState::default();
        let ctx = dummy_ctx(); // keys_down returns false for everything

        let cmds = evaluate_event_sheet(&sheet, &mut state, &ctx);
        assert_eq!(cmds.len(), 1); // negated false = true
    }

    #[test]
    fn every_n_seconds_timer() {
        let sheet = EventSheet {
            name: "test".into(),
            events: vec![Event::new(
                vec![Condition::new(ConditionKind::EveryNSeconds {
                    interval: 0.5,
                })],
                vec![Action::new(ActionKind::Log { message: "tick".into() })],
            )],
        };
        let mut state = EventSheetState::default();
        let ctx = EventContext {
            dt: 0.1,
            ..dummy_ctx()
        };

        let mut total_fires = 0;
        for _ in 0..10 {
            // 10 * 0.1 = 1.0 second
            let cmds = evaluate_event_sheet(&sheet, &mut state, &ctx);
            total_fires += cmds.len();
        }
        assert_eq!(total_fires, 2); // fires at 0.5s and 1.0s
    }

    #[test]
    fn sub_events() {
        let sheet = EventSheet {
            name: "test".into(),
            events: vec![Event {
                conditions: vec![Condition::new(ConditionKind::EveryTick)],
                actions: vec![Action::new(ActionKind::Log {
                    message: "parent".into(),
                })],
                sub_events: vec![Event::new(
                    vec![Condition::new(ConditionKind::OnKeyPressed {
                        key: "Space".into(),
                    })],
                    vec![Action::new(ActionKind::Log {
                        message: "child".into(),
                    })],
                )],
                enabled: true,
            }],
        };
        let mut state = EventSheetState::default();
        let ctx = dummy_ctx();

        let cmds = evaluate_event_sheet(&sheet, &mut state, &ctx);
        assert_eq!(cmds.len(), 2); // parent + child (Space is pressed)
    }

    #[test]
    fn serialization_roundtrip() {
        let sheet = EventSheet {
            name: "test".into(),
            events: vec![Event::new(
                vec![Condition::new(ConditionKind::OnKeyPressed {
                    key: "Space".into(),
                })],
                vec![
                    Action::new(ActionKind::SetPosition { x: 100.0, y: 200.0 }),
                    Action::new(ActionKind::PlaySound {
                        sound: "jump.wav".into(),
                    }),
                ],
            )],
        };

        let json = serde_json::to_string_pretty(&sheet).unwrap();
        let loaded: EventSheet = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.events.len(), 1);
        assert_eq!(loaded.events[0].conditions.len(), 1);
        assert_eq!(loaded.events[0].actions.len(), 2);
    }
}
