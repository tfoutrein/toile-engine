//! Animation state machine — the single source of truth shared by the runtime and
//! the editor (ADR-038 / ADR-039 Phase 0.5).
//!
//! Both the runtime (which *selects* states from live motion) and the editor (which
//! *describes* each state's trigger condition to the user) derive from the functions
//! here, so the displayed conditions can never drift from the actual behaviour. The
//! drift is pinned by [`tests`] that assert `select_states` and `condition_for` agree.

use toile_behaviors::BehaviorConfig;

use crate::{AnimState, AnimationData, AnimationStateMap, EntityData};

/// Run kicks in above this multiple of the walk threshold. Used by BOTH
/// [`select_states`] (the predicate) and [`condition_for`] (the description), so the
/// "Run" tier can never be described with a different number than it actually uses.
pub const RUN_THRESHOLD_MULTIPLIER: f32 = 24.0;

/// Default horizontal speed (px/s) above which a platformer entity is "walking".
/// Mirrors `AnimationStateMap::default().move_threshold`.
pub const DEFAULT_MOVE_THRESHOLD: f32 = 5.0;

/// Movement behavior that can drive automatic idle/walk/jump animations (ADR-038).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MotionKind {
    Platform,
    TopDown,
}

/// The movement behavior (if any) that drives this entity's automatic animations.
pub fn motion_kind(data: &EntityData) -> Option<MotionKind> {
    for b in &data.behaviors {
        match b {
            BehaviorConfig::Platform(_) => return Some(MotionKind::Platform),
            BehaviorConfig::TopDown(_) => return Some(MotionKind::TopDown),
            _ => {}
        }
    }
    None
}

/// Snapshot of an entity's motion, used to drive the animation state machine (ADR-038).
#[derive(Clone, Copy, Debug)]
pub struct MotionSnapshot {
    pub on_ground: bool,
    pub was_on_ground: bool,
    pub vx: f32,
    pub vy: f32,
}

/// States to try, in priority order, for the current motion (first that resolves wins).
/// World +y is up: `vy >= 0` means rising (jump), `< 0` means falling (fall).
pub fn select_states(kind: MotionKind, snap: &MotionSnapshot, move_threshold: f32) -> Vec<AnimState> {
    use AnimState as S;
    match kind {
        MotionKind::Platform => {
            if !snap.on_ground && snap.vy >= 0.0 {
                vec![S::Jump]
            } else if !snap.on_ground {
                vec![S::Fall, S::Jump]
            } else if snap.vx.abs() > move_threshold * RUN_THRESHOLD_MULTIPLIER {
                vec![S::Run, S::Walk]
            } else if snap.vx.abs() > move_threshold {
                vec![S::Walk]
            } else {
                vec![S::Idle]
            }
        }
        MotionKind::TopDown => {
            if snap.vx != 0.0 || snap.vy != 0.0 {
                vec![S::Walk]
            } else {
                vec![S::Idle]
            }
        }
    }
}

/// Language-neutral description of when a canonical state triggers, derived from the
/// same branches/constants as [`select_states`]. The editor turns this into UI text,
/// so it never hard-codes thresholds that could drift from the runtime.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ConditionDescription {
    /// On the ground, horizontal speed below the walk threshold.
    GroundedStill,
    /// On the ground, `|vx|` above `speed` (covers both the Walk and Run tiers).
    GroundedFasterThan { speed: f32 },
    /// Airborne, moving up.
    AirborneRising,
    /// Airborne, moving down.
    AirborneFalling,
    /// Any non-zero velocity (top-down).
    Moving,
    /// Zero velocity (top-down).
    Still,
    /// Driven by event sheets, not motion (Custom states).
    Scripted,
    /// This state isn't reachable for this motion kind.
    NotApplicable,
}

/// The trigger condition for `state` under `kind`, given the entity's `move_threshold`.
/// Kept in lock-step with [`select_states`] by the consistency tests below.
pub fn condition_for(state: &AnimState, kind: MotionKind, move_threshold: f32) -> ConditionDescription {
    use AnimState as S;
    use ConditionDescription as C;
    match kind {
        MotionKind::Platform => match state {
            S::Idle => C::GroundedStill,
            S::Walk => C::GroundedFasterThan { speed: move_threshold },
            S::Run => C::GroundedFasterThan { speed: move_threshold * RUN_THRESHOLD_MULTIPLIER },
            S::Jump => C::AirborneRising,
            S::Fall => C::AirborneFalling,
            S::Custom(_) => C::Scripted,
        },
        MotionKind::TopDown => match state {
            S::Idle => C::Still,
            S::Walk | S::Run => C::Moving,
            S::Jump | S::Fall => C::NotApplicable,
            S::Custom(_) => C::Scripted,
        },
    }
}

/// Synonyms for a canonical animation state — matched case-insensitively, so an anim
/// named "Idle", "marche" or "course" still maps to its state (ADR-038). Custom states
/// carry no synonyms (they resolve only via an explicit binding).
pub fn state_synonyms(state: &AnimState) -> &'static [&'static str] {
    use AnimState::*;
    match state {
        Idle => &["idle", "repos", "stand", "default", "wait"],
        Walk => &["walk", "marche", "move", "moving"],
        Run => &["run", "course", "sprint"],
        Jump => &["jump", "saut", "sauter", "rise"],
        Fall => &["fall", "chute", "tomber"],
        Custom(_) => &[],
    }
}

/// How an entity's animations get their pixels (ADR-039 Phase 3). A *grid* clip indexes
/// the entity's shared `sprite_sheet` (and renders from the entity's base texture); a
/// *strip* clip carries its own `sprite_file` and is autonomous. This drives the
/// "Replace base sprite" guard: replacing the base sprite/sheet can misalign GRID clips
/// (their frame indices point into the old sheet) but never touches STRIP clips.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SourcingModel {
    /// No animations.
    None,
    /// All clips index the entity's shared sprite sheet.
    Grid,
    /// All clips carry their own `sprite_file`.
    Strip,
    /// A mix of grid and strip clips.
    Mixed,
}

/// Classify how an entity's animations are sourced (see [`SourcingModel`]).
pub fn detect_sourcing_model(anims: &[AnimationData]) -> SourcingModel {
    let mut has_grid = false;
    let mut has_strip = false;
    for a in anims {
        if a.sprite_file.is_some() {
            has_strip = true;
        } else {
            has_grid = true;
        }
    }
    match (has_grid, has_strip) {
        (false, false) => SourcingModel::None,
        (true, false) => SourcingModel::Grid,
        (false, true) => SourcingModel::Strip,
        (true, true) => SourcingModel::Mixed,
    }
}

/// Resolve the first state that maps to an existing animation: explicit binding
/// (`animation_states`) first, then the case-insensitive name-synonym fallback.
pub fn resolve_state_to_anim(
    states: &[AnimState],
    anims: &[AnimationData],
    map: Option<&AnimationStateMap>,
) -> Option<String> {
    for st in states {
        if let Some(m) = map {
            if let Some(bound) = m.anim_for(st) {
                if anims.iter().any(|a| a.name == bound) {
                    return Some(bound.to_string());
                }
            }
        }
        let syns = state_synonyms(st);
        if let Some(a) = anims.iter().find(|a| syns.iter().any(|s| a.name.eq_ignore_ascii_case(s))) {
            return Some(a.name.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anim(name: &str) -> AnimationData {
        AnimationData {
            name: name.into(),
            frames: vec![0, 1],
            fps: 8.0,
            looping: true,
            sprite_file: None,
            strip_frames: None,
        }
    }

    fn snap(on_ground: bool, vx: f32, vy: f32) -> MotionSnapshot {
        MotionSnapshot { on_ground, was_on_ground: on_ground, vx, vy }
    }

    #[test]
    fn select_states_reproduces_legacy_platform_behaviour() {
        use AnimState as S;
        assert_eq!(select_states(MotionKind::Platform, &snap(false, 0.0, 10.0), 5.0), vec![S::Jump]);
        assert_eq!(select_states(MotionKind::Platform, &snap(false, 0.0, -10.0), 5.0), vec![S::Fall, S::Jump]);
        assert_eq!(select_states(MotionKind::Platform, &snap(true, 10.0, 0.0), 5.0), vec![S::Walk]);
        assert_eq!(select_states(MotionKind::Platform, &snap(true, 0.0, 0.0), 5.0), vec![S::Idle]);
        assert_eq!(select_states(MotionKind::Platform, &snap(true, 300.0, 0.0), 5.0), vec![S::Run, S::Walk]);
    }

    #[test]
    fn select_states_topdown() {
        use AnimState as S;
        assert_eq!(select_states(MotionKind::TopDown, &snap(true, 0.0, 0.0), 5.0), vec![S::Idle]);
        assert_eq!(select_states(MotionKind::TopDown, &snap(true, 1.0, 0.0), 5.0), vec![S::Walk]);
        assert_eq!(select_states(MotionKind::TopDown, &snap(true, 0.0, -3.0), 5.0), vec![S::Walk]);
    }

    /// Drift guard: the described Walk/Run thresholds must be the exact boundaries at
    /// which `select_states` flips. If someone changes the predicate without updating the
    /// shared constants (or vice-versa), this fails.
    #[test]
    fn condition_for_matches_select_states_thresholds() {
        use AnimState as S;
        let thr = 5.0_f32;

        // Walk boundary.
        let walk_speed = match condition_for(&S::Walk, MotionKind::Platform, thr) {
            ConditionDescription::GroundedFasterThan { speed } => speed,
            other => panic!("Walk should be GroundedFasterThan, got {other:?}"),
        };
        assert_eq!(walk_speed, thr);
        assert_eq!(select_states(MotionKind::Platform, &snap(true, walk_speed + 0.1, 0.0), thr), vec![S::Walk]);
        assert_eq!(select_states(MotionKind::Platform, &snap(true, walk_speed - 0.1, 0.0), thr), vec![S::Idle]);
        // Boundary-exact: the threshold is STRICT (`>`), so exactly at walk_speed → Idle.
        // (Pins `>` vs `>=`, which the ±0.1 probes above miss.)
        assert_eq!(select_states(MotionKind::Platform, &snap(true, walk_speed, 0.0), thr), vec![S::Idle]);

        // Run boundary.
        let run_speed = match condition_for(&S::Run, MotionKind::Platform, thr) {
            ConditionDescription::GroundedFasterThan { speed } => speed,
            other => panic!("Run should be GroundedFasterThan, got {other:?}"),
        };
        assert_eq!(run_speed, thr * RUN_THRESHOLD_MULTIPLIER);
        assert_eq!(select_states(MotionKind::Platform, &snap(true, run_speed + 0.1, 0.0), thr)[0], S::Run);
        // Just below the run tier → Walk (not Run).
        assert_eq!(select_states(MotionKind::Platform, &snap(true, run_speed - 0.1, 0.0), thr), vec![S::Walk]);
        // Boundary-exact: strict `>`, so exactly at run_speed → Walk (not yet Run).
        assert_eq!(select_states(MotionKind::Platform, &snap(true, run_speed, 0.0), thr), vec![S::Walk]);

        // Air states — tie the descriptions to select_states so the vy boundary is pinned too.
        assert_eq!(condition_for(&S::Jump, MotionKind::Platform, thr), ConditionDescription::AirborneRising);
        assert_eq!(condition_for(&S::Fall, MotionKind::Platform, thr), ConditionDescription::AirborneFalling);
        assert_eq!(condition_for(&S::Idle, MotionKind::Platform, thr), ConditionDescription::GroundedStill);
        // AirborneRising covers the vy == 0 apex (the split is `vy >= 0`), so it must yield Jump.
        assert_eq!(select_states(MotionKind::Platform, &snap(false, 0.0, 0.0), thr), vec![S::Jump]);
        // AirborneFalling: any downward vy → Fall (then Jump fallback).
        assert_eq!(select_states(MotionKind::Platform, &snap(false, 0.0, -0.1), thr), vec![S::Fall, S::Jump]);

        // Top-down.
        assert_eq!(condition_for(&S::Walk, MotionKind::TopDown, thr), ConditionDescription::Moving);
        assert_eq!(condition_for(&S::Idle, MotionKind::TopDown, thr), ConditionDescription::Still);

        // Scripted.
        assert_eq!(condition_for(&S::Custom("attack".into()), MotionKind::Platform, thr), ConditionDescription::Scripted);
    }

    #[test]
    fn resolve_via_synonyms_case_insensitively() {
        let anims = vec![anim("Idle"), anim("Marche"), anim("JUMP")];
        assert_eq!(resolve_state_to_anim(&[AnimState::Idle], &anims, None).as_deref(), Some("Idle"));
        // "Marche" is a Walk synonym, matched case-insensitively.
        assert_eq!(resolve_state_to_anim(&[AnimState::Walk], &anims, None).as_deref(), Some("Marche"));
        assert_eq!(resolve_state_to_anim(&[AnimState::Jump], &anims, None).as_deref(), Some("JUMP"));
        // No Run synonym present.
        assert_eq!(resolve_state_to_anim(&[AnimState::Run], &anims, None), None);
    }

    #[test]
    fn resolve_legacy_exact_names_still_match() {
        // Non-regression: the classic lowercase idle/walk/jump names still resolve.
        let anims = vec![anim("idle"), anim("walk"), anim("jump")];
        assert_eq!(resolve_state_to_anim(&[AnimState::Idle], &anims, None).as_deref(), Some("idle"));
        assert_eq!(resolve_state_to_anim(&[AnimState::Walk], &anims, None).as_deref(), Some("walk"));
        assert_eq!(resolve_state_to_anim(&[AnimState::Jump], &anims, None).as_deref(), Some("jump"));
    }

    #[test]
    fn resolve_falls_back_through_priority_list() {
        let anims = vec![anim("jump"), anim("walk")];
        // No "fall" anim → falls back to "jump".
        assert_eq!(resolve_state_to_anim(&[AnimState::Fall, AnimState::Jump], &anims, None).as_deref(), Some("jump"));
        // No "run" anim → falls back to "walk".
        assert_eq!(resolve_state_to_anim(&[AnimState::Run, AnimState::Walk], &anims, None).as_deref(), Some("walk"));
    }

    #[test]
    fn resolve_prefers_explicit_binding_then_name_fallback() {
        let anims = vec![anim("course"), anim("idle")];
        let mut map = AnimationStateMap::default();
        map.set_binding(AnimState::Walk, "course".into());
        // Walk → explicit binding "course" (even though "course" is a Run synonym).
        assert_eq!(resolve_state_to_anim(&[AnimState::Walk], &anims, Some(&map)).as_deref(), Some("course"));
        // Idle → no binding, resolves by name fallback.
        assert_eq!(resolve_state_to_anim(&[AnimState::Idle], &anims, Some(&map)).as_deref(), Some("idle"));
        // Without a map, Walk has no name/synonym match here → None.
        assert_eq!(resolve_state_to_anim(&[AnimState::Walk], &anims, None), None);
    }

    #[test]
    fn custom_states_have_no_synonyms() {
        assert!(state_synonyms(&AnimState::Custom("attack".into())).is_empty());
        assert_eq!(state_synonyms(&AnimState::Walk), &["walk", "marche", "move", "moving"]);
    }

    #[test]
    fn detect_sourcing_model_classifies() {
        let grid = AnimationData { name: "g".into(), frames: vec![0], fps: 8.0, looping: true, sprite_file: None, strip_frames: None };
        let strip = AnimationData { name: "s".into(), frames: vec![0], fps: 8.0, looping: true, sprite_file: Some("s.png".into()), strip_frames: Some(1) };
        assert_eq!(detect_sourcing_model(&[]), SourcingModel::None);
        assert_eq!(detect_sourcing_model(&[grid.clone()]), SourcingModel::Grid);
        assert_eq!(detect_sourcing_model(&[strip.clone()]), SourcingModel::Strip);
        assert_eq!(detect_sourcing_model(&[grid, strip]), SourcingModel::Mixed);
    }
}
