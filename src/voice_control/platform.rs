//! Only explicit local actions; never interpolate recognized speech into a script.
use super::commands::Action;
use color_eyre::eyre::{Result, eyre};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run(program: &str, args: &[&str]) -> Result<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let start = Instant::now();
    while child.try_wait()?.is_none() {
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(eyre!(
                "The local command did not respond within five seconds"
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!(
            "{}",
            error.trim().chars().take(300).collect::<String>()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}

fn script(source: &str) -> Result<String> {
    run("/usr/bin/osascript", &["-e", source])
}

fn helper(name: &str) -> Result<String> {
    for folder in ["/opt/homebrew/bin", "/usr/local/bin"] {
        let path = format!("{folder}/{name}");
        if std::path::Path::new(&path).is_file() {
            return Ok(path);
        }
    }
    Err(eyre!(
        "Install {name}, which is also used by your existing Mac controls"
    ))
}

fn volume() -> Result<(u8, bool)> {
    if let Ok(value) = script("output volume of (get volume settings)")
        && let Ok(level) = value.parse::<u8>()
    {
        return Ok((level, false));
    }
    let value = run(&helper("m1ddc")?, &["get", "volume"])?;
    Ok((value.parse()?, true))
}

fn set_volume(level: u8, ddc: bool) -> Result<()> {
    if ddc {
        run(&helper("m1ddc")?, &["set", "volume", &level.to_string()])?;
    } else {
        script(&format!("set volume output volume {level}"))?;
    }
    Ok(())
}

fn brightness(level: Option<u8>, step: i8) -> Result<String> {
    let tool = helper("m1ddc")?;
    let level = match level {
        Some(n) => n,
        None => {
            let current: i16 = run(&tool, &["get", "brightness"])?.parse()?;
            (current + i16::from(step)).clamp(0, 100) as u8
        }
    };
    run(&tool, &["set", "brightness", &level.to_string()])?;
    Ok(format!("Brightness: {level}%"))
}

fn light_key(key: u8) -> Result<()> {
    script(&format!(
        "tell application \"System Events\"\nif not (exists process \"Digital Light\") then error \"Digital Light is not running\"\nkey code {key} using {{command down, option down, control down, shift down}}\nend tell"
    ))?;
    Ok(())
}

pub(super) fn execute(action: Action) -> Result<String> {
    match action {
        Action::Volume(step) => {
            let (current, ddc) = volume()?;
            let level = (i16::from(current) + i16::from(step)).clamp(0, 100) as u8;
            set_volume(level, ddc)?;
            Ok(format!("Volume: {level}%"))
        }
        Action::SetVolume(level) => {
            let (_, ddc) = volume()?;
            set_volume(level, ddc)?;
            Ok(format!("Volume: {level}%"))
        }
        Action::Mute(muted) => {
            let (_, ddc) = volume()?;
            if ddc {
                run(
                    &helper("m1ddc")?,
                    &["set", "mute", if muted { "on" } else { "off" }],
                )?;
            } else {
                script(if muted {
                    "set volume with output muted"
                } else {
                    "set volume without output muted"
                })?;
            }
            Ok(if muted { "Muted" } else { "Sound on" }.into())
        }
        Action::Brightness(step) => brightness(None, step),
        Action::SetBrightness(n) => brightness(Some(n), 0),
        Action::Play | Action::Pause | Action::PlayPause | Action::Next | Action::Previous => {
            let command = match action {
                Action::Play => "play",
                Action::Pause => "pause",
                Action::Next => "next",
                Action::Previous => "previous",
                _ => "togglePlayPause",
            };
            run(&helper("nowplaying-cli")?, &[command])?;
            Ok(format!("Media: {command}"))
        }
        Action::LightToggle => {
            light_key(18)?;
            Ok("Digital Light toggled".into())
        }
        Action::LightPower(on) => {
            script(
                "tell application \"System Events\"\nif not (exists process \"Digital Light\") then error \"Digital Light is not running\"\nend tell",
            )?;
            let current = run(
                "/usr/bin/defaults",
                &["read", "mezenin.Digital-Light", "saved_isOn"],
            )?;
            let current = match current.as_str() {
                "1" | "true" => true,
                "0" | "false" => false,
                _ => return Err(eyre!("Could not read Digital Light's power state")),
            };
            if on != current {
                light_key(18)?;
            }
            Ok(if on {
                "Digital Light on"
            } else {
                "Digital Light off"
            }
            .into())
        }
        Action::LightMode => {
            light_key(19)?;
            Ok("Digital Light mode changed".into())
        }
        Action::Sleep => {
            run("/usr/bin/pmset", &["sleepnow"])?;
            Ok("Sleeping".into())
        }
        Action::Lock => {
            script(
                "tell application \"System Events\" to keystroke \"q\" using {control down, command down}",
            )?;
            Ok("Locked".into())
        }
        Action::Cancel => Ok("Cancelled".into()),
    }
}
