# Security policy

We take the security of NextTabletDriver seriously. This page explains which versions get fixes and how to report a problem.

## Supported versions

Only the **latest release** receives security fixes. Please update before you report a problem, to check that it still exists.

## Report a vulnerability

**Do not open a public issue for a security problem.** A public issue lets everyone see the problem before it is fixed.

Use GitHub's private report form:

1. Go to the [Security tab](https://github.com/Next-Tablet-Driver/NextTabletDriver/security) of the repository.
2. Choose **Report a vulnerability**.

Only the maintainers can read your report.

If you do not see that button, open a public issue with the title "Security contact request" and **no technical details**. A maintainer will then get in touch with you privately.

### What to include

- A clear description of the problem and its impact.
- Steps to reproduce it, or a small example.
- The version of NextTabletDriver, your operating system and its version.
- Anything you already know about a fix.

### What happens next

1. We read your report and confirm that we received it.
2. We check the problem and tell you what we found.
3. We work on a fix and publish it in a new release.
4. We tell you when it is public. If you want, we credit you in the release notes.

Please give us time to fix the problem before you share it with others.

## What is in scope

- The desktop app and its installers.
- The plugin loader, in particular anything that lets a plugin load **without** your approval (see [Plugins](../docs/plugins.md#how-plugins-are-trusted)).
- The WebSocket server (see [Integrations](../docs/integrations.md#websocket)).
- The update process.
- The handling of theme and settings files, and of files you import.
- The SDK.

## What is not in scope

- A plugin that you approved doing harm. A plugin is native code that runs with your rights, and the app warns you about this before it runs one.
- Problems in a third-party dependency. Report them to that project. Tell us too if they affect us.
- Problems in the tablet definitions. Those belong to [OpenTabletDriver](https://github.com/OpenTabletDriver/OpenTabletDriver).
- Attacks that need full control of your computer first.

## How the app protects you

These are design choices that you can rely on. If you find a way around one, that is a vulnerability.

- **Plugins** are loaded only after you approve the exact file (its SHA-256 hash).
- The **WebSocket server** is off by default. When it is on, it listens on `127.0.0.1` only, accepts at most 10 clients, and refuses browser pages that do not come from your own computer.
- **Theme files** contain data only. They are checked against strict rules and size limits, and they cannot run code.
- The **interface** never chooses a file path. File pickers for plugins and themes are opened by the Rust part of the app.
- The interface can only connect to the app itself and to the GitHub API (it has a content security policy).
