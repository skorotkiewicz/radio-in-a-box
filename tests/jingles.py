"""Run after building the Liquidsoap image: python3 tests/jingles.py [engine image]."""
import pathlib
import re
import subprocess
import sys
import tempfile
import wave

engine, image = sys.argv[1:] or ["docker", "radio-liquidsoap"]
root = pathlib.Path(tempfile.mkdtemp(prefix="radio-jingles-check-"))
music = root / "music"
jingles = music / "jingles"
jingles.mkdir(parents=True)
music.chmod(0o755)
jingles.chmod(0o755)

# Tiny valid WAVs need no ffmpeg or test framework.
for name in ["song1", "song2", "queued", "jingle1", "jingle2"]:
    path = (jingles if name.startswith("jingle") else music) / f"{name}.wav"
    with wave.open(str(path), "wb") as audio:
        audio.setnchannels(2)
        audio.setsampwidth(2)
        audio.setframerate(44100)
        audio.writeframes(bytes(44100 * 4))
    path.chmod(0o644)

(music / "playlist.m3u").write_text(
    '#EXTM3U\nannotate:title="Song 1":/music/song1.wav\n'
    'annotate:title="Song 2":/music/song2.wav\n'
)
jingle_playlist = (
    '#EXTM3U\nannotate:title="Jingle 1":/music/jingles/jingle1.wav\n'
    'annotate:title="Jingle 2":/music/jingles/jingle2.wav\n'
)
station = pathlib.Path(__file__).resolve().parents[1] / "docker/radio.liq"
script = root / "check.liq"
# Exercise the actual station pipeline, replacing only its network output.
script.write_text(station.read_text().split("output.icecast(", 1)[0] + '''
ignore(requests.push(request.create("annotate:title=\\"Song queued\\":/music/queued.wav")))
played = ref(0)
def check_track(metadata) =
  title = list.assoc(default="", "title", metadata)
  if title != "" then
    print("CHECK " ^ title)
    played := !played + 1
    # Test-only exit: do not join the streaming thread from its own callback.
    if !played >= 6 then exit(0) end
  end
end
radio.on_track(check_track)
output.dummy(radio)
''')
script.chmod(0o644)

for every, available, expected in [
    (2, True, [False, False, True] * 2),
    (1, True, [False, True] * 3),
    (0, True, [False] * 6),
    (2, False, [False] * 6),
]:
    (jingles / "every.txt").write_text(f"{every}\n")
    (jingles / "playlist.m3u").write_text(jingle_playlist if available else "#EXTM3U\n")
    for path in [music / "playlist.m3u", jingles / "playlist.m3u", jingles / "every.txt"]:
        path.chmod(0o644)
    result = subprocess.run([
        engine, "run", "--rm", "--network", "none", "--read-only", "--tmpfs", "/tmp",
        "--volume", f"{music}:/music:ro", "--volume", f"{script}:/etc/check.liq:ro",
        "--entrypoint", "liquidsoap", image, "/etc/check.liq",
    ], capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, result.stdout + result.stderr
    tracks = re.findall(r"^CHECK (.+)$", result.stdout, re.MULTILINE)
    assert len(tracks) == 6, result.stdout + result.stderr
    assert [track.startswith("Jingle") for track in tracks] == expected, tracks
    assert "Song queued" in tracks, tracks
    print(f"PASS: every={every}, jingles available={available}: {tracks}")

print(f"Test audio retained at {root}")
