# Rust edits the playlist and sends commands, not executable Liquidsoap code.
settings.log.file.set(false)
settings.log.stdout.set(true)
settings.server.telnet.set(true)
settings.server.telnet.bind_addr.set("127.0.0.1")
settings.server.telnet.port.set(1234)

library = playlist(id="library", mode="randomize", reload_mode="watch", "/music/playlist.m3u")
requests = request.queue(id="requests")
# Queued tracks interrupt the library; an empty library broadcasts silence.
radio = fallback(track_sensitive=false, [requests, library, blank()])
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
