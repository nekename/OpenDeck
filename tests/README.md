## Automated testing

This directory contains automated testing infrastructure for OpenDeck. Tests are written in Rust and use the [thirtyfour](https://crates.io/crates/thirtyfour) crate for Rust bindings to Selenium, which is used to drive a [tauri-driver](https://crates.io/crates/tauri-driver) instance.

To run the tests on Linux, you must have `WebKitWebDriver` available on your system. On Arch, I was able to install this using the `webkitgtk-6.0` package. On Windows, you need to have Microsoft Edge Driver installed. See https://v2.tauri.app/develop/tests/webdriver/ for more details on setting up WebDriver for your platform.
