# discord-rpc-helper

Helps set Steam game activity as a Discord activity.

## Installation

### Compile with cargo

Clone the repo and run `cargo install --path .` from the repo's root directory.

You will also need to provide your own Discord client id. Make sure `XDG_CONFIG_HOME` is set and create `$XDG_CONFIG_HOME\discord-rpc-helper\config.json`.

### Example config.json

```json
{
  "discord_client_id": "11111111111111111111" 
}
```

### Cargo binstall

We have added binstall support in version 1.1.7. If you have binstall installed (`cargo install cargo-binstall`) then you can install the service with `cargo binstall discord-rpc-helper`. After that, you can either run the helper from the terminal or follow the [systemd setup](#running-the-service-automatically-on-login-systemd-service) bellow.

## Caching directory

We scan for `XDG_RUNTIME_DIR` and create a folder in there. This means the cache does not persist between reboots. In the future there will be an option to configure the cache path.

## Running the service automatically on login (systemd service)

Create `$HOME/.config/systemd/user/discord-rpc-helper.service` and paste the following:

```systemd
[Service]
Environment=XDG_CONFIG_HOME=/home/YOURUSERNAME/.config
ExecStart=/home/YOURUSERNAME/.cargo/bin/discord-rpc-helper

[Install]
WantedBy=default.target

[Unit]
Description=Discord RPC helper for Steam, written in Rust
After=network.target
```

Make sure to change YOURUSERNAME to the username that you used to run `cargo install` with.

After that run `systemctl --user daemon-reload` and `systemctl --user enable --now discord-rpc-helper.service`.

Check if everything is running by running `systemctl --user status discord-rpc-helper.service`.

## Quirks and features

### Steam Age Gate

When a game requires an age gate to get to the Steam Store page, we handle the age gate by submitting an age of 1/1/1990.