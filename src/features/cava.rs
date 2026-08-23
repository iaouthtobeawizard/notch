use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub struct Cava {
    process: Child,
    receiver: Receiver<String>,
    current: Vec<f32>,
    target: Vec<f32>,
}

impl Cava {
    pub fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let config = r#"
[general]
bars = 20
framerate = 30
sensitvity = 100

[input]
method = pipewire

[output]
method = raw
raw_target = /dev/stdout
data_format = ascii
ascii_max_range = 7
bar_delimiter = 59
frame_delimiter = 10

[smoothing]
noise_reduction = 50
monstercat = 0
gravity = 25
"#;

        let config_path = format!("/tmp/notch-cava-{}.conf", std::process::id());

        std::fs::write(&config_path, config)?;

        let mut process = Command::new("cava")
            .arg("-p")
            .arg(&config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = process
            .stdout
            .take()
            .ok_or("failed to capture Cava stdout")?;

        let stderr = process
            .stderr
            .take()
            .ok_or("failed to capture Cava stderr")?;

        let (sender, receiver) = mpsc::channel::<String>();

        thread::spawn(move || {
            let reader = BufReader::new(stdout);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        thread::spawn(move || {
            let reader = BufReader::new(stderr);

            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("Cava: {line}");
                }
            }
        });

        Ok(Self {
            process,
            receiver,
            current: vec![0.0; 20],
            target: vec![0.0; 20],
        })
    }

    pub fn try_frame(&mut self) -> Option<Vec<f32>> {
        let mut latest = None;

        while let Ok(frame) = self.receiver.try_recv() {
            latest = Some(frame);
        }

        if let Some(frame) = latest {
            self.target = Self::values(&frame);

            if self.current.len() != self.target.len() {
                self.current = vec![0.0; self.target.len()];
            }
        }

        if self.target.is_empty() {
            return None;
        }

        for i in 0..self.target.len() {
            let target = self.target[i];
            let current = self.current[i];

            let smoothing = if target > current { 0.45 } else { 0.18 };

            self.current[i] += (target - current) * smoothing;
        }

        Some(self.current.clone())
    }

    fn values(frame: &str) -> Vec<f32> {
        frame
            .split(';')
            .filter_map(|value| value.trim().parse::<f32>().ok())
            .map(|value| (value / 7.0).clamp(0.0, 1.0))
            .collect()
    }
}

impl Drop for Cava {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();

        let path = format!("/tmp/notch-cava-{}.conf", std::process::id());

        let _ = std::fs::remove_file(path);
    }
}
