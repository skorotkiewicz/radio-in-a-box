# Rust edits the playlist and sends commands, not executable Liquidsoap code.
settings.log.file.set(false)
settings.log.stdout.set(true)
settings.server.telnet.set(true)
settings.server.telnet.bind_addr.set("127.0.0.1")
settings.server.telnet.port.set(1234)

library = playlist(id="library", mode="randomize", reload_mode="watch", "/music/playlist.m3u")
requests = request.queue(id="requests")
jingles = playlist(id="jingles", mode="random", reload_mode="watch", "/music/jingles/playlist.m3u")

# Read the atomic setting at track boundaries; zero disables jingles.
def jingle_every() =
  if file.exists("/music/jingles/every.txt") then
    max(0, int_of_string(default=0, string.trim(file.contents("/music/jingles/every.txt"))))
  else
    0
  end
end

# Queued and automatic songs both count. A playing jingle finishes before requests.
songs = fallback(track_sensitive=false, [requests, library])
rotation = rotate(weights=[
  {max(1, jingle_every())},
  {if jingle_every() > 0 and songs.is_ready() then 1 else 0 end}
], [songs, jingles])
# Missing jingles do not block songs; an empty song library broadcasts silence.
radio = fallback(track_sensitive=false, [rotation, blank()])
server.register(namespace="radio", description="Skip the current track.", "skip",
  fun (_) -> begin radio.skip(); "OK" end)

output.icecast(
  %mp3(bitrate=128),
  host="icecast", port=8000,
  password=getenv("ICECAST_SOURCE_PASSWORD"),
  mount="/radio.mp3",
  name=getenv("STATION_NAME"),
  description="Independent radio, broadcasting around the clock.",
  public=false,
  radio
)
