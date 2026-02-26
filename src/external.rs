#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ExternalParams {
    pub animation: Option<String>,
    pub speed: Option<f64>,
    pub intensity: Option<f64>,
    pub color_shift: Option<f64>,
    pub scale: Option<f64>,
    pub render: Option<String>,
    pub color: Option<String>,
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

        // Keep self.params in sync with accumulated state
        self.params.animation = self.animation_pending.clone();
        self.params.scale = self.scale_pending;
        self.params.render = self.render_pending.clone();
        self.params.color = self.color_pending.clone();
        self.params.speed = self.speed;
        self.params.intensity = self.intensity;
        self.params.color_shift = self.color_shift;
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

    pub fn speed(&self) -> f64 {
        self.speed.unwrap_or(1.0)
    }

    pub fn intensity(&self) -> f64 {
        self.intensity.unwrap_or(1.0)
    }

    pub fn color_shift(&self) -> f64 {
        self.color_shift.unwrap_or(0.0)
    }

    pub fn params(&self) -> &ExternalParams {
        &self.params
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
                let mut watcher = notify::recommended_watcher(move |res| {
                    let _ = file_tx.send(res);
                })
                .unwrap();
                notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive)
                    .unwrap();
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
