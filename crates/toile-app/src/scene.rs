use crate::GameContext;
use toile_core::scene_stack::Transition;

/// A scene in the scene stack. Implement this trait for each game screen.
pub trait Scene {
    /// Called when this scene is pushed onto the stack or becomes active.
    fn on_enter(&mut self, _ctx: &mut GameContext) {}

    /// Called when this scene is removed from the stack.
    fn on_exit(&mut self, _ctx: &mut GameContext) {}

    /// Called when another scene is pushed on top (this scene freezes).
    fn on_freeze(&mut self) {}

    /// Called when the scene above is popped (this scene resumes).
    fn on_thaw(&mut self) {}

    /// Fixed-timestep update.
    fn update(&mut self, ctx: &mut GameContext, dt: f64);

    /// Draw the scene.
    fn draw(&mut self, ctx: &mut GameContext);

    /// Whether this scene is transparent (scenes below are also drawn).
    fn is_transparent(&self) -> bool {
        false
    }
}

/// Command to modify the scene stack (processed at end of update).
pub enum SceneCommand {
    Push(Box<dyn Scene>, Option<Transition>),
    Pop(Option<Transition>),
    Replace(Box<dyn Scene>, Option<Transition>),
}

enum TransitionState {
    In {
        transition: Transition,
        elapsed: f32,
    },
    Out {
        transition: Transition,
        elapsed: f32,
        next_command: PendingCommand,
    },
}

enum PendingCommand {
    Push(Box<dyn Scene>),
    Pop,
    Replace(Box<dyn Scene>),
}

/// Manages a stack of scenes with transition effects.
pub struct SceneStack {
    stack: Vec<Box<dyn Scene>>,
    commands: Vec<SceneCommand>,
    transition: Option<TransitionState>,
}

impl SceneStack {
    pub fn new(initial: impl Scene + 'static) -> Self {
        Self {
            stack: vec![Box::new(initial)],
            commands: Vec::new(),
            transition: None,
        }
    }

    /// Push a scene on top of the stack.
    pub fn push(&mut self, scene: impl Scene + 'static, transition: Option<Transition>) {
        self.commands
            .push(SceneCommand::Push(Box::new(scene), transition));
    }

    /// Pop the top scene.
    pub fn pop(&mut self, transition: Option<Transition>) {
        self.commands.push(SceneCommand::Pop(transition));
    }

    /// Replace the top scene.
    pub fn replace(&mut self, scene: impl Scene + 'static, transition: Option<Transition>) {
        self.commands
            .push(SceneCommand::Replace(Box::new(scene), transition));
    }

    /// Update the active scene and process transitions.
    pub fn update(&mut self, ctx: &mut GameContext, dt: f64) {
        // Advance transition
        if let Some(ref mut ts) = self.transition {
            match ts {
                TransitionState::In {
                    transition,
                    elapsed,
                } => {
                    *elapsed += dt as f32;
                    if transition.is_done(*elapsed) {
                        self.transition = None;
                    }
                }
                TransitionState::Out {
                    transition,
                    elapsed,
                    ..
                } => {
                    *elapsed += dt as f32;
                    if transition.is_done(*elapsed) {
                        // Execute the pending command
                        let ts = self.transition.take().unwrap();
                        if let TransitionState::Out { next_command, .. } = ts {
                            match next_command {
                                PendingCommand::Push(mut scene) => {
                                    if let Some(top) = self.stack.last_mut() {
                                        top.on_freeze();
                                    }
                                    scene.on_enter(ctx);
                                    self.stack.push(scene);
                                }
                                PendingCommand::Pop => {
                                    if let Some(mut scene) = self.stack.pop() {
                                        scene.on_exit(ctx);
                                    }
                                    if let Some(top) = self.stack.last_mut() {
                                        top.on_thaw();
                                    }
                                }
                                PendingCommand::Replace(mut scene) => {
                                    if let Some(mut old) = self.stack.pop() {
                                        old.on_exit(ctx);
                                    }
                                    scene.on_enter(ctx);
                                    self.stack.push(scene);
                                }
                            }
                        }
                        // Start "in" transition
                        self.transition = Some(TransitionState::In {
                            transition: Transition::fade(0.2),
                            elapsed: 0.0,
                        });
                    }
                }
            }
        }

        // Process queued commands (only if no transition is active)
        if self.transition.is_none() && !self.commands.is_empty() {
            let commands: Vec<_> = self.commands.drain(..).collect();
            for cmd in commands {
                match cmd {
                    SceneCommand::Push(scene, transition) => {
                        if let Some(t) = transition {
                            self.transition = Some(TransitionState::Out {
                                transition: t,
                                elapsed: 0.0,
                                next_command: PendingCommand::Push(scene),
                            });
                        } else {
                            if let Some(top) = self.stack.last_mut() {
                                top.on_freeze();
                            }
                            let mut scene = scene;
                            scene.on_enter(ctx);
                            self.stack.push(scene);
                        }
                    }
                    SceneCommand::Pop(transition) => {
                        if let Some(t) = transition {
                            self.transition = Some(TransitionState::Out {
                                transition: t,
                                elapsed: 0.0,
                                next_command: PendingCommand::Pop,
                            });
                        } else {
                            if let Some(mut scene) = self.stack.pop() {
                                scene.on_exit(ctx);
                            }
                            if let Some(top) = self.stack.last_mut() {
                                top.on_thaw();
                            }
                        }
                    }
                    SceneCommand::Replace(scene, transition) => {
                        if let Some(t) = transition {
                            self.transition = Some(TransitionState::Out {
                                transition: t,
                                elapsed: 0.0,
                                next_command: PendingCommand::Replace(scene),
                            });
                        } else {
                            if let Some(mut old) = self.stack.pop() {
                                old.on_exit(ctx);
                            }
                            let mut scene = scene;
                            scene.on_enter(ctx);
                            self.stack.push(scene);
                        }
                    }
                }
                break; // Process one command at a time
            }
        }

        // Update the top scene
        if let Some(top) = self.stack.last_mut() {
            top.update(ctx, dt);
        }
    }

    /// Draw visible scenes (transparent scenes show those below).
    pub fn draw(&mut self, ctx: &mut GameContext) {
        // Find the lowest visible scene
        let mut draw_from = self.stack.len().saturating_sub(1);
        for i in (0..self.stack.len()).rev() {
            draw_from = i;
            if !self.stack[i].is_transparent() {
                break;
            }
        }

        // Draw from bottom-visible to top
        for i in draw_from..self.stack.len() {
            self.stack[i].draw(ctx);
        }
    }

    /// Get the current transition alpha (0.0 = fully visible, 1.0 = fully black).
    /// Returns None if no transition is active.
    pub fn transition_alpha(&self) -> Option<f32> {
        match &self.transition {
            Some(TransitionState::Out {
                transition,
                elapsed,
                ..
            }) => Some(transition.progress(*elapsed)),
            Some(TransitionState::In {
                transition,
                elapsed,
            }) => Some(1.0 - transition.progress(*elapsed)),
            None => None,
        }
    }

    /// Number of scenes on the stack.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Whether a transition is currently playing.
    pub fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }
}
