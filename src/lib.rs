//! Shared library for the Waylight binaries. The cxx-qt QML module (greeter
//! UI, theme engine, translations) is built exactly once, here, and every
//! binary links it: `waylight-greeter` (the greetd greeter),
//! `waylight-config` (the configuration GUI) and `waylight-configd` (the
//! privileged D-Bus configuration service). The daemon never touches Qt
//! objects; it only shares validation and language helpers.

pub mod accounts;
pub mod backend;
pub mod config_backend;
pub mod configd;
pub mod controller;
pub mod i18n;
pub mod power;
pub mod sessions;
pub mod state;
