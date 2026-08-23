# Notch

A native Linux notch and control center built with Rust, Wayland, and Slint.

Notch is designed as a lightweight, native Wayland application that provides a persistent top-of-screen
interface for system information, audio visulization, notifications, media controls, and eventually a full control center

---

## Overview

Notch is built around a native Wayland layer-shell surface and a custom software-rendering pipeline.

The project is intentionally modular:

- Wayland handles compositor integration.
- Slint handles UI definition.
- The software renderer produces the UI pixels.

The goal is to keep the core runtime small while allowing the feature system and UI grow independtly.

---
