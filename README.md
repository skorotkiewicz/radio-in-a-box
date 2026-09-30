# Radio

A self-hosted radio station: Icecast streams MP3, Liquidsoap rotates your music,
a Rust CLI manages it, and a lightweight webpage lets people listen.
The page uses ivory, charcoal, and burnt orange. No frontend build step.

## Start

Install Docker with Compose v2 (supporting `up --wait`) and Rust, then run from
this directory:

```sh
cargo install --path . --locked
radio init
radio add /path/to/music/*.mp3
radio start
```

Open **http://localhost:8080**. The direct MP3 stream is
**http://localhost:8080/radio.mp3**. An empty library broadcasts silence.
Only stream music you have permission to broadcast.

If you prefer not to install the CLI, use `cargo run -- <command>`.
Keep running commands from this project directory.

## Manage live playback

```sh
radio list
radio add '/path/to/Another Track.flac'
radio queue 'Another Track.flac'
radio skip
radio reload
radio status
radio logs
radio stop
```

The `.liq` script is the stable audio engine, not a generated script per song.
Rust copies complete tracks into `music/`, appends their container paths to
`music/playlist.m3u`, and Liquidsoap watches the playlist for live changes.
Queued tracks take priority over the shuffled library. Skip and reload use
Liquidsoap's command socket via `docker compose exec`, without a public
management port. Track changes do not restart the stream.

Imports support MP3, Ogg, FLAC, WAV, M4A, and AAC. Actual decoding depends on
the file's codec. Files are copied, not moved; duplicate names are rejected.
Use one CLI importer at a time. If disk or playlist writing fails, the copied
file is retained for manual recovery rather than deleted. To edit the rotation,
edit `music/playlist.m3u` and run `radio reload`; it contains `/music/filename`
paths, one per line. You can also edit `docker/radio.liq` for advanced audio
behavior, then run `radio restart`.

## Station settings

`radio init` creates `.env` with three independent random passwords. It never
overwrites existing configuration or music. On Unix, `.env` is owner-only;
it and your music are ignored by Git and excluded from Docker build contexts.
Back up `.env` and `music/` privately.

Edit `.env`, then run `radio restart` to apply changes:

```dotenv
STATION_NAME=My Radio
RADIO_BIND=127.0.0.1
RADIO_PORT=8080
ICECAST_PORT=8000
```

Keep the generated password lines. If you replace passwords, use at least
24 alphanumeric characters; XML-special characters are deliberately rejected.
The listener page gets the station name, current track, and listener count
from Icecast. Its stream and status requests use the same origin.

Icecast's admin page is **http://localhost:8000/admin/**, with username `admin`
and `ICECAST_ADMIN_PASSWORD` from `.env`. That port is bound to localhost only.
The webpage proxies only the stream and public status, not admin endpoints.

## Public station

The default binds the website to localhost. To allow other machines to listen,
set `RADIO_BIND=0.0.0.0` and run `radio restart`. For internet access, put an HTTPS
reverse proxy in front of port 8080, configure DNS and firewall rules, and
keep Icecast's admin port and Docker socket private. The included stack does
not provision a domain or TLS certificates. Icecast defaults to 100 listeners;
adjust `docker/icecast.sh` and available bandwidth if you need more.

All services restart automatically and have health checks. The audio services
run as non-root users with read-only filesystems and temporary writable space.
Stopping or recreating containers does not remove your music or settings.

## Checks

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
node --check web/app.js
node --test tests/web.test.cjs
# After radio init:
docker compose config --quiet
# With a running Docker daemon:
radio start
curl -f http://localhost:8080/status-json.xsl
curl -f --max-time 3 http://localhost:8080/radio.mp3 -o /tmp/radio-sample.mp3
```

The stream curl will normally exit with timeout code 28: a live stream does
not end. Confirm that it received audio bytes. Use `radio logs` for decoder
or connection errors. Edit frontend files in `web/` directly; no rebuild is
needed, only a browser refresh.
