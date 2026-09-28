#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ExternalParams {
    pub animation: Option<String>,
    pub speed: Option<f64>,
    pub intensity: Option<f64>,
    pub color_shift: Option<f64>,
    pub scale: Option<f64>,
    pub render: Option<String>,
    pub color: Option<String>,
    /// Animation-specific parameters by name (`{"params": {"cohesion": 0.5}}`).
    /// Values are normalized to 0..1 against the animation's declared range.
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Default)]
pub struct CurrentState {
    pub animation_pending: Option<String>,
    pub scale_pending: Option<f64>,
    pub render_pending: Option<String>,
    pub color_pending: Option<String>,
    pub speed: Option<f64>,
    pub intensity: Option<f64>,
    pub color_shift: Option<f64>,
    pub params: ExternalParams,
    named_pending: std::collections::BTreeMap<String, f64>,
    /// Once any named parameter arrives, the deprecated global-field overloads
    /// (speed/intensity/color_shift doubling as per-animation knobs) stop being
    /// forwarded, so the two paths cannot fight over the same field.
    named_ever_seen: bool,
}

impl CurrentState {
    pub fn merge(&mut self, p: ExternalParams) {
        if let Some(v) = p.animation.clone() {
            self.animation_pending = Some(v);
        }
        if let Some(v) = p.scale {
            self.scale_pending = Some(v);
        }
        if let Some(v) = p.render.clone() {
            self.render_pending = Some(v);
        }
        if let Some(v) = p.color.clone() {
            self.color_pending = Some(v);
        }
        if let Some(v) = p.speed {
            self.speed = Some(v);
        }
        if let Some(v) = p.intensity {
            self.intensity = Some(v);
        }
        if let Some(v) = p.color_shift {
            self.color_shift = Some(v);
        }
        if !p.params.is_empty() {
            self.named_ever_seen = true;
            self.named_pending.extend(p.params);
        }

        // Keep self.params in sync with accumulated state
        self.params.animation = self.animation_pending.clone();
        self.params.scale = self.scale_pending;
        self.params.render = self.render_pending.clone();
        self.params.color = self.color_pending.clone();
        self.params.speed = self.speed;
        self.params.intensity = self.intensity;
        self.params.color_shift = self.color_shift;
        self.params.params.clear();
    }

    pub fn take_animation_change(&mut self) -> Option<String> {
        let v = self.animation_pending.take();
        if v.is_some() {
            self.params.animation = None;
        }
        v
    }

    pub fn take_scale_change(&mut self) -> Option<f64> {
        let v = self.scale_pending.take();
        if v.is_some() {
            self.params.scale = None;
        }
        v
    }

    pub fn take_render_change(&mut self) -> Option<String> {
        let v = self.render_pending.take();
        if v.is_some() {
            self.params.render = None;
        }
        v
    }

    pub fn take_color_change(&mut self) -> Option<String> {
        let v = self.color_pending.take();
        if v.is_some() {
            self.params.color = None;
        }
        v
    }

    /// Drain pending named parameters. `run_loop` applies each drained value
    /// once, via the animation's `set_param`.
    pub fn take_named_params(&mut self) -> std::collections::BTreeMap<String, f64> {
        std::mem::take(&mut self.named_pending)
    }

    /// The parameter set to forward to the deprecated global-field overload
    /// path (`set_params`). Once any named parameter has been received, the
    /// globals stop doubling as per-animation knobs, so the two paths cannot
    /// fight over the same internal field.
    pub fn legacy_overloads(&self) -> ExternalParams {
        if self.named_ever_seen {
            ExternalParams::default()
        } else {
            self.params.clone()
        }
    }

    pub fn speed(&self) -> f64 {
        self.speed.unwrap_or(1.0)
    }

    pub fn intensity(&self) -> f64 {
        self.intensity.unwrap_or(1.0)
    }

    pub fn color_shift(&self) -> f64 {
        self.color_shift.unwrap_or(0.0)
    }
}

pub enum ParamsSource {
    Stdin,
    File(std::path::PathBuf),
}

pub fn spawn_reader(source: ParamsSource) -> std::sync::mpsc::Receiver<ExternalParams> {
    let (tx, rx) = std::sync::mpsc::channel::<ExternalParams>();

    match source {
        ParamsSource::Stdin => {
            std::thread::spawn(move || {
                use std::io::BufRead;
                let stdin = std::io::BufReader::new(std::io::stdin());
                for line in stdin.lines() {
                    match line {
                        Ok(l) => {
                            if let Ok(params) = serde_json::from_str::<ExternalParams>(&l)
                                && tx.send(params).is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }
        ParamsSource::File(path) => {
            std::thread::spawn(move || {
                // Read the file once on startup if it already exists
                if let Ok(contents) = std::fs::read_to_string(&path)
                    && let Some(line) = contents.lines().rfind(|l| !l.trim().is_empty())
                    && let Ok(params) = serde_json::from_str::<ExternalParams>(line)
                    && tx.send(params).is_err()
                {
                    return;
                }

                let (file_tx, file_rx) = std::sync::mpsc::channel();
                let mut watcher = match notify::recommended_watcher(move |res| {
                    let _ = file_tx.send(res);
                }) {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("termflix: could not create file watcher: {e}");
                        return;
                    }
                };
                if let Err(e) =
                    notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive)
                {
                    eprintln!("termflix: could not watch {}: {e}", path.display());
                    return;
                }
                while let Ok(Ok(_event)) = file_rx.recv() {
                    if let Ok(contents) = std::fs::read_to_string(&path)
                        && let Some(line) = contents.lines().rfind(|l| !l.trim().is_empty())
                        && let Ok(params) = serde_json::from_str::<ExternalParams>(line)
                        && tx.send(params).is_err()
                    {
                        break;
                    }
                }
            });
        }
    }

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_params_deserializes_partial() {
        let json = r#"{"animation": "matrix", "speed": 2.0}"#;
        let p: ExternalParams = serde_json::from_str(json).unwrap();
        assert_eq!(p.animation.as_deref(), Some("matrix"));
        assert_eq!(p.speed, Some(2.0));
        assert!(p.intensity.is_none());
    }

    #[test]
    fn test_external_params_empty_object() {
        let json = "{}";
        let p: ExternalParams = serde_json::from_str(json).unwrap();
        assert!(p.animation.is_none());
        assert!(p.speed.is_none());
    }

    #[test]
    fn test_external_params_invalid_json_fails() {
        let json = "not json";
        let result = serde_json::from_str::<ExternalParams>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_current_state_merge_accumulates() {
        let mut state = CurrentState::default();
        state.merge(ExternalParams {
            speed: Some(2.0),
            ..Default::default()
        });
        state.merge(ExternalParams {
            intensity: Some(0.5),
            ..Default::default()
        });
        assert_eq!(state.speed(), 2.0);
        assert_eq!(state.intensity(), 0.5);
    }

    #[test]
    fn test_current_state_take_animation_change() {
        let mut state = CurrentState::default();
        state.merge(ExternalParams {
            animation: Some("fire".to_string()),
            ..Default::default()
        });
        let change = state.take_animation_change();
        assert_eq!(change.as_deref(), Some("fire"));
        // Second take returns None
        assert!(state.take_animation_change().is_none());
    }

    #[test]
    fn named_params_merge_and_drain_once() {
        let mut state = CurrentState::default();
        let mut incoming = ExternalParams::default();
        incoming.params.insert("cohesion".to_string(), 0.5);
        state.merge(incoming);
        let drained = state.take_named_params();
        assert_eq!(drained.get("cohesion"), Some(&0.5));
        // Drain is one-shot: the next frame gets nothing.
        assert!(state.take_named_params().is_empty());
    }

    #[test]
    fn named_params_deserialize_from_json() {
        let p: ExternalParams =
            serde_json::from_str(r#"{"params": {"cohesion": 0.5, "drag": 1.0}}"#).unwrap();
        assert_eq!(p.params.get("cohesion"), Some(&0.5));
        assert_eq!(p.params.get("drag"), Some(&1.0));
    }

    #[test]
    fn old_json_without_params_still_valid() {
        let p: ExternalParams = serde_json::from_str(r#"{"intensity": 1.0}"#).unwrap();
        assert_eq!(p.intensity, Some(1.0));
        assert!(p.params.is_empty());
    }

    #[test]
    fn legacy_overloads_suppressed_after_named_params() {
        let mut state = CurrentState::default();
        state.merge(ExternalParams {
            intensity: Some(1.0),
            ..Default::default()
        });
        // Before any named param: globals flow through untouched.
        assert_eq!(state.legacy_overloads().intensity, Some(1.0));

        let mut incoming = ExternalParams::default();
        incoming.params.insert("cohesion".to_string(), 0.2);
        state.merge(incoming);
        // After a named param: the deprecated overload path goes quiet.
        assert_eq!(state.legacy_overloads().intensity, None);
        assert!(state.legacy_overloads().speed.is_none());
    }
}
