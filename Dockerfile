FROM debian:bookworm-slim AS icecast
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends icecast2 curl ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home radio
COPY docker/icecast.sh /usr/local/bin/icecast-entrypoint
USER radio
EXPOSE 8000
ENTRYPOINT ["sh", "/usr/local/bin/icecast-entrypoint"]

FROM debian:bookworm-slim AS liquidsoap
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends liquidsoap netcat-openbsd ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home radio
USER radio
CMD ["liquidsoap", "/etc/radio.liq"]
