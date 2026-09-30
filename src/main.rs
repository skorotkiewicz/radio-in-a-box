use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    process::{Command, Stdio},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const HELP: &str = "radio: run from the directory containing compose.yaml

  init              Create private passwords and an empty library (no overwrites)
  add FILE...       Copy audio into the library and update the live playlist
  list              List library tracks
  start             Build and start the station
  stop              Stop the station, keeping all music and settings
  restart           Restart services after editing configuration
  status            Show service status
  logs              Follow service logs (Ctrl+C to exit)
  queue NAME        Play a library file next, interrupting the automatic playlist
  skip              Skip the current track
  reload            Reload the playlist without restarting

Listener page: http://localhost:8080
Stream:        http://localhost:8080/radio.mp3
";

fn main() {
    if let Err(error) = run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("radio: {error}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<()> {
    let Some(command) = args.first().map(String::as_str) else {
        print!("{HELP}");
        return Ok(());
    };
    if matches!(command, "help" | "--help" | "-h") {
        print!("{HELP}");
        return Ok(());
    }
    let root = std::env::current_dir()?;
    if !root.join("compose.yaml").is_file() {
        return Err("run from the radio project directory containing compose.yaml".into());
    }
    if command == "init" && args.len() == 1 {
        init(&root)?;
        println!("Station initialized. Existing files kept; passwords are in .env.");
        return Ok(());
    }
    if !root.join(".env").is_file() || !root.join("music/playlist.m3u").is_file() {
        return Err("station is not initialized; run radio init".into());
    }
    match (command, &args[1..]) {
        ("add", files) if !files.is_empty() => {
            for file in files {
                let name = add(&root, Path::new(file))?;
                println!("Added {name}");
            }
            Ok(())
        }
        ("list", []) => {
            for line in fs::read_to_string(root.join("music/playlist.m3u"))?.lines() {
                if let Some(name) = line.strip_prefix("/music/") {
                    println!("{name}");
                }
            }
            Ok(())
        }
        ("start", []) => compose(&["up", "--detach", "--build", "--wait"]),
        ("stop", []) => compose(&["stop"]),
        ("restart", []) => compose(&["up", "--detach", "--build", "--force-recreate", "--wait"]),
        ("status", []) => compose(&["ps"]),
        ("logs", []) => compose(&["logs", "--follow", "--tail", "100"]),
        ("skip", []) => control("radio.skip"),
        ("reload", []) => control("library.reload"),
        ("queue", [name]) => {
            validate_name(name)?;
            if Path::new(name).file_name() != Some(std::ffi::OsStr::new(name)) {
                return Err("queue expects a library filename, not a path".into());
            }
            if !root.join("music").join(name).is_file() {
                return Err(format!("track not in library: {name}").into());
            }
            control(&format!("requests.push /music/{name}"))
        }
        _ => Err(format!("unknown command or invalid arguments\n\n{HELP}").into()),
    }
}

fn secret() -> Result<String> {
    let mut bytes = [0_u8; 24];
    getrandom::fill(&mut bytes).map_err(|error| format!("secure randomness unavailable: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn create_if_missing(path: &Path, content: &str, mode: u32) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(mode);
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(content.as_bytes())?;
            #[cfg(unix)]
            file.set_permissions(fs::Permissions::from_mode(mode))?;
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && path.is_file() => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn init(root: &Path) -> Result<()> {
    let music = root.join("music");
    match fs::create_dir(&music) {
        Ok(()) => {
            #[cfg(unix)]
            fs::set_permissions(&music, fs::Permissions::from_mode(0o755))?;
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && music.is_dir() => {}
        Err(error) => return Err(error.into()),
    }
    create_if_missing(
        &root.join(".env"),
        &format!(
            "STATION_NAME=My Radio\nRADIO_BIND=127.0.0.1\nRADIO_PORT=8080\nICECAST_PORT=8000\nICECAST_SOURCE_PASSWORD={}\nICECAST_ADMIN_PASSWORD={}\nICECAST_RELAY_PASSWORD={}\n",
            secret()?, secret()?, secret()?
        ),
        0o600,
    )?;
    create_if_missing(&music.join("playlist.m3u"), "#EXTM3U\n", 0o644)
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.chars().any(char::is_control) {
        return Err("audio filenames must be nonempty and contain no control characters".into());
    }
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "mp3" | "ogg" | "flac" | "wav" | "m4a" | "aac") {
        return Err("supported audio extensions: mp3, ogg, flac, wav, m4a, aac".into());
    }
    Ok(())
}

fn add(root: &Path, source: &Path) -> Result<String> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("audio filename must be valid UTF-8")?;
    validate_name(name)?;
    let mut input = File::open(source)?;
    if !input.metadata()?.is_file() {
        return Err("audio source must be a regular file".into());
    }
    let destination = root.join("music").join(name);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| format!("cannot import {name} (existing files are never overwritten): {error}"))?;
    io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    #[cfg(unix)]
    output.set_permissions(fs::Permissions::from_mode(0o644))?;
    // Publish only complete copies. A failed import leaves its file for manual recovery.
    // ponytail: one playlist writer; add a file lock if concurrent CLI imports are needed.
    let mut playlist = OpenOptions::new()
        .append(true)
        .open(root.join("music/playlist.m3u"))?;
    playlist.write_all(format!("/music/{name}\n").as_bytes())?;
    playlist.sync_all()?;
    Ok(name.to_owned())
}

fn docker() -> Command {
    let mut command = Command::new("docker");
    command.args(["compose", "-f", "compose.yaml"]);
    command
}

fn compose(args: &[&str]) -> Result<()> {
    let status = docker()
        .args(args)
        .status()
        .map_err(|error| format!("could not run Docker Compose: {error}"))?;
    if !status.success() {
        return Err(format!("Docker Compose failed ({status}); check radio logs").into());
    }
    Ok(())
}

fn control(command: &str) -> Result<()> {
    let mut child = docker()
        .args(["exec", "-T", "liquidsoap", "nc", "-w", "2", "127.0.0.1", "1234"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{command}\nexit")?;
    }
    let output = child.wait_with_output()?;
    let response = String::from_utf8_lossy(&output.stdout);
    if !output.status.success()
        || !response.lines().any(|line| line.trim() == "END")
        || response.to_ascii_lowercase().contains("error")
        || response.to_ascii_lowercase().contains("unknown command")
    {
        return Err(format!("Liquidsoap command failed: {response}; is the station running?").into());
    }
    print!("{response}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_check() -> Result<()> {
        let root = std::env::temp_dir().join(format!("radio-check-{}", secret()?));
        fs::create_dir(&root)?;
        init(&root)?;
        let env = fs::read_to_string(root.join(".env"))?;
        init(&root)?;
        assert_eq!(fs::read_to_string(root.join(".env"))?, env);
        let passwords: Vec<_> = env.lines().filter(|line| line.contains("PASSWORD=")).collect();
        assert_eq!(passwords.len(), 3);
        for password in passwords {
            let value = password.split_once('=').unwrap().1;
            assert_eq!(value.len(), 48);
            assert!(value.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
        #[cfg(unix)]
        assert_eq!(fs::metadata(root.join(".env"))?.permissions().mode() & 0o777, 0o600);
        let track = root.join("a song.MP3");
        fs::write(&track, b"test audio")?;
        assert_eq!(add(&root, &track)?, "a song.MP3");
        assert_eq!(fs::read(root.join("music/a song.MP3"))?, b"test audio");
        assert_eq!(fs::read_to_string(root.join("music/playlist.m3u"))?, "#EXTM3U\n/music/a song.MP3\n");
        assert!(add(&root, &track).is_err());
        assert!(validate_name("track\nradio.skip.mp3").is_err());
        assert!(validate_name("track.txt").is_err());
        assert_eq!(fs::read(&track)?, b"test audio");
        Ok(())
    }
}
