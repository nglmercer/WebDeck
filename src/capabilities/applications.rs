use super::*;

fn bounded(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn normalize(mut options: Vec<ApplicationOption>) -> Vec<ApplicationOption> {
    options.retain(|option| !option.value.is_empty() && !option.name.is_empty());
    options.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.value.cmp(&b.value))
    });
    let mut seen = std::collections::BTreeSet::new();
    options.retain(|option| seen.insert(option.value.clone()));
    options.truncate(512);
    options
}

pub fn application_options(audio: bool) -> ApplicationList {
    let found = if audio {
        audio_options()
    } else {
        Ok(process_options())
    };
    ApplicationList {
        api_version: 2,
        available: found.is_ok(),
        applications: normalize(found.unwrap_or_default()),
    }
}

fn process_options() -> Vec<ApplicationOption> {
    let system = sysinfo::System::new_all();
    system
        .processes()
        .values()
        .filter_map(|process| {
            let name = process.name().to_string_lossy();
            if name.is_empty() {
                return None;
            }
            let executable = process
                .exe()
                .map(|path| bounded(&path.to_string_lossy(), 4096))
                .unwrap_or_default();
            Some(ApplicationOption {
                name: bounded(&name, 256),
                value: bounded(&name, 256),
                description: executable.clone(),
                executable,
            })
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn audio_options() -> Result<Vec<ApplicationOption>> {
    let mut command = Process::new("pactl");
    command.args(["-f", "json", "list", "sink-inputs"]);
    let output = ManagedChild::output(command, Duration::from_secs(4))?;
    if !output.status.success() {
        return Err(Error::execution());
    }
    parse_audio_inputs(&serde_json::from_slice(&output.stdout).map_err(|_| Error::execution())?)
}

#[cfg(windows)]
fn audio_options() -> Result<Vec<ApplicationOption>> {
    super::windows::audio_applications()
}

#[cfg(not(any(target_os = "linux", windows)))]
fn audio_options() -> Result<Vec<ApplicationOption>> {
    Err(unsupported())
}

#[cfg(any(target_os = "linux", test))]
fn parse_audio_inputs(value: &Value) -> Result<Vec<ApplicationOption>> {
    Ok(value
        .as_array()
        .ok_or_else(Error::execution)?
        .iter()
        .filter_map(|input| {
            let props = &input["properties"];
            let binary = props["application.process.binary"].as_str().unwrap_or("");
            let name = props["application.name"]
                .as_str()
                .filter(|name| !name.is_empty())
                .unwrap_or(binary);
            let target = if binary.is_empty() { name } else { binary };
            if name.is_empty() || target.chars().count() > 256 {
                return None;
            }
            Some(ApplicationOption {
                name: bounded(name, 256),
                value: target.into(),
                description: bounded(binary, 4096),
                executable: String::new(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_discovery_uses_stable_native_targets_and_deduplicates_streams() {
        let fixtures = json!([
            {"properties":{"application.name":"Spotify","application.process.binary":"spotify"}},
            {"properties":{"application.name":"Spotify","application.process.binary":"spotify"}},
            {"properties":{"application.name":"Browser"}},
            {"properties":{}}
        ]);
        let apps = normalize(parse_audio_inputs(&fixtures).unwrap());
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].value, "Browser");
        assert_eq!(apps[1].value, "spotify");
        assert!(apps.iter().all(|app| app.executable.is_empty()));
        assert!(parse_audio_inputs(&json!({})).is_err());
    }
}
