#!/bin/sh
set -eu

# Restrict secrets to safe XML text; radio init generates hexadecimal passwords.
for password in "${ICECAST_SOURCE_PASSWORD:-}" "${ICECAST_ADMIN_PASSWORD:-}" "${ICECAST_RELAY_PASSWORD:-}"; do
    case "$password" in
        ''|*[!a-zA-Z0-9]*) echo 'Icecast passwords must be nonempty and alphanumeric.' >&2; exit 1 ;;
    esac
    if [ "${#password}" -lt 24 ]; then
        echo 'Icecast passwords must be at least 24 characters.' >&2
        exit 1
    fi
done

umask 077
cat > /tmp/icecast.xml <<EOF
<icecast>
  <location>Independent radio</location>
  <admin>admin@localhost</admin>
  <hostname>localhost</hostname>
  <limits>
    <clients>100</clients>
    <sources>2</sources>
    <queue-size>524288</queue-size>
    <client-timeout>30</client-timeout>
    <header-timeout>15</header-timeout>
    <source-timeout>10</source-timeout>
    <burst-size>65535</burst-size>
  </limits>
  <authentication>
    <source-password>${ICECAST_SOURCE_PASSWORD}</source-password>
    <relay-password>${ICECAST_RELAY_PASSWORD}</relay-password>
    <admin-user>admin</admin-user>
    <admin-password>${ICECAST_ADMIN_PASSWORD}</admin-password>
  </authentication>
  <listen-socket><port>8000</port></listen-socket>
  <fileserve>1</fileserve>
  <paths>
    <basedir>/usr/share/icecast2</basedir>
    <logdir>/tmp</logdir>
    <webroot>/usr/share/icecast2/web</webroot>
    <adminroot>/usr/share/icecast2/admin</adminroot>
    <alias source="/" destination="/status.xsl"/>
  </paths>
  <logging>
    <accesslog>-</accesslog>
    <errorlog>-</errorlog>
    <loglevel>3</loglevel>
  </logging>
  <security><chroot>0</chroot></security>
</icecast>
EOF
exec icecast2 -c /tmp/icecast.xml
