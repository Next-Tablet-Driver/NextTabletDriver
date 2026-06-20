# Contributing to NextTabletDriver

Thank you for your interest in contributing to **NextTabletDriver**! We welcome bug fixes, documentation improvements, new tablet profiles, and feature contributions.

To ensure the driver remains fast, clean, and stable, please read our detailed guides before you start:

* 🛠️ **[Developer Guide](file:///c:/Users/iswea/Documents/Developpement/Projects/osu/NextTabletDriver/.dev/docs/development.md):** Learn how to set up the Rust toolchain, compile locally (Windows/Linux), test changes, and how our automated release pipeline works.
* 📏 **[Coding Standards & Best Practices](file:///c:/Users/iswea/Documents/Developpement/Projects/osu/NextTabletDriver/.dev/docs/best_practices.md):** Rules for avoiding panics/crashes, lock safety, documentation structure, and commit message formats.
* 📋 **[Project TODO List](file:///c:/Users/iswea/Documents/Developpement/Projects/osu/NextTabletDriver/.dev/docs/todo.md):** Review our active backlog of bug fixes and upcoming feature enhancements.

---

## Quick Start Contribution Loop

### 1. Set Up the Repository
Fork the repository on GitHub, clone it locally, and create a branch for your work:
```bash
git clone https://github.com/YOUR_USERNAME/NextTabletDriver.git
cd NextTabletDriver
git checkout -b my-contribution-branch
```

### 2. Make Your Changes
Write your code, adding documentation and tests where necessary. Ensure your changes follow our coding standards (no `unwrap`, safe lock handling, etc.).

### 3. Run Pre-Commit Checks
Before pushing, verify that your changes pass all local linting and formatting gates using the validation script:

* **Windows:**
  ```powershell
  .dev/tools/validate.ps1
  ```
* **Linux/macOS:**
  ```bash
  bash .dev/tools/validate.sh
  ```

### 4. Commit and Push
We enforce **Conventional Commit** guidelines. Structure your commit messages as follows:
`type(scope): description` (e.g., `fix(websocket): handle bind error gracefully`).

Push your branch to your fork:
```bash
git push origin my-contribution-branch
```

### 5. Open a Pull Request
Go to the original repository on GitHub, and open a Pull Request. Provide a clear explanation of what your change does, what testing you performed, and reference any issues resolved.
