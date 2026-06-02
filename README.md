# Numerical ODE Approximater

A terminal app for exploring first-order ordinary differential equations of the form **y' = F(x, y)**. You type the right-hand side **F**, pick a numerical method (Euler, improved Euler, or Runge–Kutta), and see the approximate solution plotted live. You can compare several initial values **y₀**, overlay all three methods, and export point data to a text file.

This guide assumes **Rust is already installed** on your computer. You do not need to know Rust to run the program.


---

## 1. Build and run (first time)

Rust’s build tool is **Cargo**. From the project folder, run:

```bash
cargo run --release
```


---

## 2. What you see on screen

| Area | Purpose |
|------|---------|
| **Top bar** | Equation `y' = …` — edit **F(x, y)** here |
| **Graph** | Approximate solution curve(s) |
| **Sidebar** | Method, interval (**x₀**, **x_end**), step size **h**, initial value **y₀**, optional **y₀ family**, **Export** |

The default equation is **`y - y^3`** (i.e. **y' = y − y³**). That model has stable equilibria at **y = ±1** and an unstable equilibrium at **y = 0**; different starting **y₀** can converge, stay near zero, or blow up — useful for trying the **y₀ family** feature.

---

## 3. Quick start walkthrough

1. **Run** the app (`cargo run --release`).
2. The graph should already show a solution for the defaults (**x₀ = 0**, **y₀ = 0.3**, **x_end = 3**, **h = 0.01**).
3. **Change the equation** — Tab to the equation bar, edit (e.g. `x - y`), press **Enter** to refresh the graph.
4. **Change the method** — Tab to **Method**, press **Enter** or **Space** to open the list, use **↑** / **↓**, then **Enter** or **Space** to confirm (**Esc** to cancel without changing).
5. **Several initial values** — Tab to **y₀ family**, press **Enter** or **Space** to turn it **ON**, set **y₀ start**, **y₀ end**, and **y₀ #** (max 15 curves), then **Enter** on a parameter field to update.
6. **Export** — Tab to **Export**, **Enter**, fill in filename / export **h** / **points**, **Enter** to save under `exported/<name>.txt`.

Errors (bad formula, invalid numbers) appear in a **red message** at the bottom of the sidebar.

---

## 4. Keyboard reference

| Key | Action |
|-----|--------|
| **Tab** / **↑** / **↓** | Move focus between sidebar fields and the equation bar |
| **←** / **→** | Move cursor inside a text field |
| **Home** / **End** | Jump to start/end of text in a field |
| **Enter** | Recompute graph (equation or numeric fields); open export; open **y₀ family** toggle; confirm method menu |
| **Space** | Open method menu (when **Method** is focused); confirm method choice in the menu; toggle **y₀ family** when that row is focused |
| **Esc** | Cancel export dialog or method menu (no change) |
| **q** | Quit the app |

In the **export** popup: **Tab** / **↑** / **↓** move between filename, **h**, and **points**; **Enter** saves; **Esc** cancels.

---

## 5. Equation syntax

You only type the right-hand side **F**; the app shows `y' = F`.

| You can write | Meaning |
|---------------|---------|
| `x`, `y` | Variables |
| `2y`, `xy`, `2(x+y)` | Implicit multiplication |
| `sin(x)`, `cos(y)`, `exp(x)` | Standard functions |
| `y^3`, `x^2` | Powers |
| `y - y^3` | Example: cubic ODE |

You may paste a full equation such as `y'=y-y^3`; the app strips the `y' =` prefix.

---

## 6. Parameters (sidebar)

| Field | Meaning |
|-------|---------|
| **x₀** | Start of the x-interval |
| **y₀** | Initial value (used when **y₀ family** is OFF) |
| **x_end** | End of the x-interval (must be **> x₀**) |
| **h** | Fixed step size for integration (must be **> 0**) |
| **y₀ family** | When ON: integrate for equally spaced **y₀** from **y₀ start** to **y₀ end** (**y₀ #** values, max 15) |

Smaller **h** means more accurate curves but slower updates. The graph subsamples to at most about 800 points per curve for drawing.

---

## 7. Methods

- **Euler** — simplest; fastest per step, least accurate.
- **Improved Euler** — predictor–corrector (Heun-type).
- **Runge–Kutta (RK4)** — usually the most accurate for a given **h**.
- **All three methods** — three stacked plots, one per method.

---

## 8. Export format

Exported files live in the **`exported/`** folder (created automatically). Each file lists the equation, parameters, and one line of `(x, y)` pairs per method (and per **y₀** when the family is enabled). Coordinates use **6 significant figures**.

Export **h** and **points** in the dialog are **only for the file**; they do not change the live graph’s sidebar **h**.

Example:

```text
y' = y - y^3
x0 = 0, y0 = 0.3, x_end = 3, h = 0.01, points = 100
# coordinates shown to 6 significant figures

Runge-Kutta: (0, 0.3), (0.03, 0.30891), ...
```

---

## 9. Running again later

From the project folder:

```bash
cargo run --release
```

No need to pass `--release` every time, but it is recommended for smooth graph updates.

To run tests (optional, for developers):

```bash
cargo test
```

---

## 10. Troubleshooting

| Problem | What to try |
|---------|-------------|
| `cargo: command not found` | Install Rust via [rustup.rs](https://rustup.rs), restart the terminal |
| Compile errors after updating the repo | `cargo build --release` and read the error line; ensure you are in the project root (folder with `Cargo.toml`) |
| Graph does not update | Press **Enter** after editing the equation or a number field; check the red error text in the sidebar |
| Blank or garbled UI | Use a larger terminal window; avoid resizing while the app runs |
| Export failed | Check that the filename is not empty; ensure `x_end > x0` and `h > 0` |

---

## License

See the repository for license information if provided.
