//! Saídas e volume por app pelo `pactl`, que fala tanto com o PulseAudio quanto com o PipeWire
//! (pipewire-pulse). As saídas são os sinks; cada app tocando áudio tem um ou mais sink inputs.

use std::collections::HashMap;

use super::{AppStream, OutputDevice};
use crate::util::run;

/// Um item do `pactl list`: `Sink #47` ou `Sink Input #85`, com os campos (`Mute: no`) e as
/// propriedades (`application.name = "Firefox"`).
#[derive(Debug, Default)]
struct Block {
    index: u32,
    fields: HashMap<String, String>,
    props: HashMap<String, String>,
}

fn parse_blocks(output: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    for line in output.lines() {
        if !line.starts_with(char::is_whitespace) {
            if let Some((_, index)) = line.rsplit_once('#') {
                if let Ok(index) = index.trim().parse() {
                    blocks.push(Block { index, ..Default::default() });
                }
            }
            continue;
        }
        let Some(block) = blocks.last_mut() else { continue };
        let text = line.trim();
        if line.starts_with("\t\t") || line.starts_with("        ") {
            if let Some((key, value)) = text.split_once(" = ") {
                block.props.insert(key.to_string(), value.trim_matches('"').to_string());
                continue;
            }
        }
        if let Some((key, value)) = text.split_once(": ") {
            block.fields.entry(key.to_string()).or_insert_with(|| value.to_string());
        } else if let Some(key) = text.strip_suffix(':') {
            block.fields.entry(key.to_string()).or_default();
        }
    }
    blocks
}

/// `front-left: 32768 /  50% / -18.06 dB,   front-right: …`: o primeiro canal basta.
fn parse_level(volume: &str) -> Option<f32> {
    volume.split('/').map(str::trim).find_map(|part| part.strip_suffix('%')?.trim().parse::<f32>().ok()).map(|p| p / 100.0)
}

fn default_sink() -> Option<String> {
    run("pactl", &["get-default-sink"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| run("pactl", &["info"])?.lines().find_map(|l| l.strip_prefix("Default Sink: ").map(|s| s.trim().to_string())))
}

fn outputs_from(sinks: &[Block], default: Option<&str>) -> Vec<OutputDevice> {
    sinks
        .iter()
        .filter_map(|sink| {
            let id = sink.fields.get("Name")?.clone();
            let name = sink.fields.get("Description").cloned().unwrap_or_else(|| id.clone());
            Some(OutputDevice { default: default == Some(id.as_str()), id, name })
        })
        .collect()
}

pub fn outputs() -> Result<Vec<OutputDevice>, String> {
    let sinks = run("pactl", &["list", "sinks"]).ok_or("não foi possível listar as saídas (pactl)")?;
    Ok(outputs_from(&parse_blocks(&sinks), default_sink().as_deref()))
}

pub fn set_output(id: &str) -> Result<(), String> {
    run("pactl", &["set-default-sink", id]).ok_or("falha ao trocar a saída de áudio")?;
    // O PipeWire já leva os streams para a saída nova; o PulseAudio só usa a padrão nos próximos.
    if let Some(inputs) = run("pactl", &["list", "short", "sink-inputs"]) {
        for index in inputs.lines().filter_map(|l| l.split_whitespace().next()) {
            let _ = run("pactl", &["move-sink-input", index, id]);
        }
    }
    Ok(())
}

/// Os sink inputs agrupados por app: o Firefox, por exemplo, abre um stream por aba.
fn apps_from(inputs: &[Block], exe: impl Fn(&str) -> Option<String>) -> Vec<AppStream> {
    let mut apps: Vec<AppStream> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    for input in inputs {
        let props = &input.props;
        let Some(name) = props.get("application.name").or_else(|| props.get("application.process.binary")).or_else(|| props.get("media.name")) else {
            continue;
        };
        let key = props.get("application.process.binary").unwrap_or(name).clone();
        let level = input.fields.get("Volume").and_then(|v| parse_level(v)).unwrap_or(1.0);
        let muted = input.fields.get("Mute").is_some_and(|m| m == "yes");
        if let Some(i) = keys.iter().position(|k| *k == key) {
            let app = &mut apps[i];
            app.id = format!("{},{}", app.id, input.index);
            app.level = app.level.max(level);
            app.muted &= muted;
            continue;
        }
        let icon = props.get("application.process.id").and_then(|pid| exe(pid));
        keys.push(key);
        apps.push(AppStream { id: input.index.to_string(), name: name.clone(), icon, level, muted });
    }
    apps
}

pub fn apps() -> Option<Vec<AppStream>> {
    let inputs = run("pactl", &["list", "sink-inputs"])?;
    let exe = |pid: &str| std::fs::read_link(format!("/proc/{pid}/exe")).ok().map(|p| p.display().to_string());
    Some(apps_from(&parse_blocks(&inputs), exe))
}

pub fn set_app_volume(id: &str, level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    for index in id.split(',') {
        index.parse::<u32>().map_err(|_| format!("stream inválido: {index}"))?;
        if let Some(level) = level {
            let percent = format!("{}%", (level * 100.0).round() as u32);
            run("pactl", &["set-sink-input-volume", index, &percent]).ok_or("falha ao ajustar o volume do app")?;
        }
        if let Some(muted) = muted {
            run("pactl", &["set-sink-input-mute", index, if muted { "1" } else { "0" }]).ok_or("falha ao silenciar o app")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINKS: &str = "Sink #47
\tState: RUNNING
\tName: alsa_output.pci-0000_00_1f.3.analog-stereo
\tDescription: Built-in Audio Analog Stereo
\tMute: no
\tVolume: front-left: 32768 /  50% / -18.06 dB,   front-right: 32768 /  50% / -18.06 dB
\t        balance 0.00
\tProperties:
\t\talsa.card = \"0\"
\t\tdevice.description = \"Built-in Audio Analog Stereo\"

Sink #52
\tState: SUSPENDED
\tName: bluez_output.AA_BB_CC_DD_EE_FF.1
\tDescription: WH-1000XM4
\tMute: no
";

    const INPUTS: &str = "Sink Input #85
\tDriver: PipeWire
\tSink: 47
\tMute: no
\tVolume: front-left: 65536 / 100% / 0.00 dB,   front-right: 65536 / 100% / 0.00 dB
\t        balance 0.00
\tProperties:
\t\tapplication.name = \"Firefox\"
\t\tapplication.process.id = \"1234\"
\t\tapplication.process.binary = \"firefox\"
\t\tmedia.name = \"AudioStream\"

Sink Input #86
\tSink: 47
\tMute: yes
\tVolume: front-left: 32768 /  50% / -18.06 dB,   front-right: 32768 /  50% / -18.06 dB
\tProperties:
\t\tapplication.name = \"Firefox\"
\t\tapplication.process.id = \"1234\"
\t\tapplication.process.binary = \"firefox\"

Sink Input #90
\tSink: 47
\tMute: yes
\tVolume: mono: 19661 /  30% / -31.37 dB
\tProperties:
\t\tapplication.name = \"Spotify\"
\t\tapplication.process.binary = \"spotify\"
";

    #[test]
    fn parses_sinks_and_default() {
        let outputs = outputs_from(&parse_blocks(SINKS), Some("bluez_output.AA_BB_CC_DD_EE_FF.1"));
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].name, "Built-in Audio Analog Stereo");
        assert!(!outputs[0].default);
        assert_eq!(outputs[1], OutputDevice { id: "bluez_output.AA_BB_CC_DD_EE_FF.1".into(), name: "WH-1000XM4".into(), default: true });
    }

    #[test]
    fn groups_sink_inputs_by_app() {
        let apps = apps_from(&parse_blocks(INPUTS), |pid| Some(format!("/proc/{pid}/exe")));
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0], AppStream { id: "85,86".into(), name: "Firefox".into(), icon: Some("/proc/1234/exe".into()), level: 1.0, muted: false });
        assert_eq!(apps[1], AppStream { id: "90".into(), name: "Spotify".into(), icon: None, level: 0.3, muted: true });
    }
}
