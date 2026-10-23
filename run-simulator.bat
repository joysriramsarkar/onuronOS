@echo off
cd /d "%~dp0"
echo [*] Starting OnuronOS Desktop Simulator (nilsim)...
rem Always launch through Cargo so the simulator cannot silently use a stale
rem release binary after a UI fix. Incremental builds make subsequent starts
rem fast while preserving the release profile used by the launcher.
cargo run --release -p nilsim -- %*
