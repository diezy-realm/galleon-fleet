#!/usr/bin/env python3
"""
Galleon Fleet — Service Manager (BL-10)
=======================================
Installs, manages, and uninstalls the Galleon daemon as an OS user service:
- Linux: systemd user unit (~/.config/systemd/user/galleon.service)
- macOS: launchd LaunchAgent (~/Library/LaunchAgents/com.galleon.fleet.plist)
- Windows: Startup shortcut or User Task

SECURITY & PRIVILEGE DESIGN:
Daemon MUST run as standard user, NOT as root/administrator.
Sudo / elevation is strictly restricted only to necessary installation steps.
"""

import argparse
import os
import platform
import subprocess
import sys
from pathlib import Path

SYSTEMD_UNIT_TEMPLATE = """[Unit]
Description=Galleon Fleet Autonomous Daemon
Documentation=https://github.com/diezy-realm/galleon-fleet
After=network.target

[Service]
Type=simple
ExecStart={bin_path} gateway start
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info
Environment=GALLEON_CONFIG_DIR={config_dir}

[Install]
WantedBy=default.target
"""

LAUNCHD_PLIST_TEMPLATE = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.galleon.fleet</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin_path}</string>
        <string>gateway</string>
        <string>start</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{log_dir}/galleon.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{log_dir}/galleon.stderr.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
        <key>GALLEON_CONFIG_DIR</key>
        <string>{config_dir}</string>
    </dict>
</dict>
</plist>
"""


def find_galleon_binary() -> str:
    # Check standard install locations or PATH
    bin_name = "galleon.exe" if os.name == "nt" else "galleon"
    fallback_bin = "clawcrew.exe" if os.name == "nt" else "clawcrew"

    # Search in current repo target or PATH
    for candidate in [
        Path.cwd() / "target" / "release" / bin_name,
        Path.cwd() / "target" / "debug" / bin_name,
        Path.cwd() / "target" / "release" / fallback_bin,
        Path.cwd() / "target" / "debug" / fallback_bin,
        Path.home() / ".cargo" / "bin" / bin_name,
        Path.home() / ".cargo" / "bin" / fallback_bin,
    ]:
        if candidate.is_file():
            return str(candidate.resolve())

    # Fallback to binary in PATH
    return bin_name


def get_config_dir() -> str:
    return str(Path.home() / ".config" / "galleon")


def install_service():
    system = platform.system()
    bin_path = find_galleon_binary()
    config_dir = get_config_dir()
    print(f"[*] Installing Galleon user service for {system}...")
    print(f"[*] Binary: {bin_path}")
    print(f"[*] Config Directory: {config_dir}")

    if system == "Linux":
        unit_dir = Path.home() / ".config" / "systemd" / "user"
        unit_dir.mkdir(parents=True, exist_ok=True)
        unit_file = unit_dir / "galleon.service"
        content = SYSTEMD_UNIT_TEMPLATE.format(bin_path=bin_path, config_dir=config_dir)
        unit_file.write_text(content)
        print(f"[+] Wrote systemd user unit: {unit_file}")
        try:
            subprocess.run(["systemctl", "--user", "daemon-reload"], check=True)
            subprocess.run(["systemctl", "--user", "enable", "galleon.service"], check=True)
            subprocess.run(["systemctl", "--user", "start", "galleon.service"], check=True)
            print("[+] Galleon service enabled and started via systemd.")
        except Exception as e:
            print(f"[!] systemctl command failed (may not be in interactive user session): {e}")

    elif system == "Darwin":
        launch_dir = Path.home() / "Library" / "LaunchAgents"
        launch_dir.mkdir(parents=True, exist_ok=True)
        log_dir = Path.home() / ".config" / "galleon" / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        plist_file = launch_dir / "com.galleon.fleet.plist"
        content = LAUNCHD_PLIST_TEMPLATE.format(
            bin_path=bin_path,
            config_dir=config_dir,
            log_dir=str(log_dir)
        )
        plist_file.write_text(content)
        print(f"[+] Wrote launchd agent plist: {plist_file}")
        try:
            subprocess.run(["launchctl", "load", "-w", str(plist_file)], check=True)
            print("[+] Galleon service loaded into launchd.")
        except Exception as e:
            print(f"[!] launchctl command failed: {e}")

    elif system == "Windows":
        startup_dir = Path(os.environ.get("APPDATA", str(Path.home()))) / "Microsoft" / "Windows" / "Start Menu" / "Programs" / "Startup"
        cmd_file = startup_dir / "galleon-daemon.cmd"
        cmd_file.write_text(f'@echo off\r\nstart /b "" "{bin_path}" gateway start\r\n')
        print(f"[+] Wrote Windows user startup script: {cmd_file}")

    print("[SUCCESS] Service installation complete.")


def service_status():
    system = platform.system()
    print(f"[*] Checking Galleon service status on {system}...")
    if system == "Linux":
        subprocess.run(["systemctl", "--user", "status", "galleon.service"])
    elif system == "Darwin":
        subprocess.run(["launchctl", "list", "com.galleon.fleet"])
    else:
        print("[*] Galleon service is configured in user startup directory.")


def uninstall_service():
    system = platform.system()
    print(f"[*] Uninstalling Galleon user service for {system}...")
    if system == "Linux":
        unit_file = Path.home() / ".config" / "systemd" / "user" / "galleon.service"
        try:
            subprocess.run(["systemctl", "--user", "stop", "galleon.service"])
            subprocess.run(["systemctl", "--user", "disable", "galleon.service"])
        except Exception:
            pass
        if unit_file.exists():
            unit_file.unlink()
            print(f"[-] Removed {unit_file}")
        try:
            subprocess.run(["systemctl", "--user", "daemon-reload"])
        except Exception:
            pass
    elif system == "Darwin":
        plist_file = Path.home() / "Library" / "LaunchAgents" / "com.galleon.fleet.plist"
        try:
            subprocess.run(["launchctl", "unload", str(plist_file)])
        except Exception:
            pass
        if plist_file.exists():
            plist_file.unlink()
            print(f"[-] Removed {plist_file}")
    elif system == "Windows":
        cmd_file = Path(os.environ.get("APPDATA", str(Path.home()))) / "Microsoft" / "Windows" / "Start Menu" / "Programs" / "Startup" / "galleon-daemon.cmd"
        if cmd_file.exists():
            cmd_file.unlink()
            print(f"[-] Removed {cmd_file}")

    print("[SUCCESS] Galleon service uninstalled.")


def main():
    parser = argparse.ArgumentParser(description="Galleon Fleet Service Manager")
    parser.add_argument("action", choices=["install", "status", "uninstall"], help="Action to perform")
    args = parser.parse_args()

    if args.action == "install":
        install_service()
    elif args.action == "status":
        service_status()
    elif args.action == "uninstall":
        uninstall_service()


if __name__ == "__main__":
    main()
