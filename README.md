# Numerical ODE Approximater

A terminal app for exploring first-order ordinary differential equations of the form **y' = F(x, y)**. You type the right-hand side **F**, pick a numerical method (Euler, improved Euler, or Runge–Kutta), and see the approximate solution plotted live. You can compare several initial values **y₀**, view a slope field, overlay all three methods, and export point data to a text file.

This project assumes **Rust is already installed**. You do not need to know Rust to use the app.

---

## Build and run

Rust’s build tool is **Cargo**. From the project folder:

```bash
cargo run --release
```

Use `--release` for smoother graph updates. Run the same command again whenever you want to reopen the app.

---

## Documentation

| Guide | For |
|-------|-----|
| **[INSTRUCTIONS.md](INSTRUCTIONS.md)** | Screen layout, walkthrough, keyboard shortcuts, equation syntax, export, troubleshooting |
| **[CODE_STRUCTURE.md](CODE_STRUCTURE.md)** | Source layout and module map (contributors) |

```bash
cargo test              # unit tests
cargo bench             # hot-path benchmarks (optional)
```

---

## License

See the repository for license information if provided.
