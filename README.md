# mbd

Runs a command when you press the M-Button on Marshall Major V headphones.

## Build

```
make build
```

## Install

```
make install
```

System-wide: `sudo make install-system`.

## Usage

```
mbd run                               run in foreground (testing)
mbd run --command "notify-send hi"    run + save shell command to config
mbd run --script ~/script.sh          run + save script path to config
mbd start                             start background daemon
mbd start --command "notify-send hi"  save to config, then start
mbd stop                              stop daemon
mbd status                            check if running
```

`--command` and `--script` save the value to `~/.config/mbd/config.toml` so it persists.

```
systemctl --user enable --now mbd     autostart on login
```

## Config

`~/.config/mbd/config.toml`:

```toml
command = "notify-send 'M-Button pressed'"
# mac = "C1:CA:AA:D4:9A:F0"
# verbose = true
# mode = "script"
```

## How it works

Subscribes to BLE characteristic `0000000c-1337-1dea-feed-c0ffee70c0de` on the Major V. The device sends `000009` on a single M-Button press.

(reconnects on disconnect)
