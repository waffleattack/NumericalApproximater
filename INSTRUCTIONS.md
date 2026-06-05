# Instructions — Numerical ODE Approximater

This guide explains how to use the terminal app. It assumes **Rust is already installed**; you do not need to know Rust to run the program.

To build and launch the app, see [README.md](README.md#build-and-run).

---

## 1. What you see on screen

| Area | Purpose |
|------|---------|
| **Top bar** | Equation `y' = …` — edit **F(x, y)** here |
| **Graph** | Approximate solution curve(s) |
| **Sidebar** | Method, **graph** mode (solution / slope / both), **y₀ family**, interval (**x₀**, **x_end**), step size **h**, initial value(s), **Save**, **Quit** |
| **Footer** | Errors (red) or brief confirmations (green) |

The default equation is **`y - y^3`** (i.e. **y' = y − y³**). That model has stable equilibria at **y = ±1** and an unstable equilibrium at **y = 0**; different starting **y₀** can converge, stay near zero, or blow up — useful for trying the **y₀ family** feature.

---

## 2. Quick start walkthrough

1. **Run** the app (`cargo run --release`).
2. The graph should already show a solution for the defaults (**x₀ = 0**, **y₀ = 0.3**, **x_end = 3**, **h = 0.01**).
3. **Change the equation** — Tab to the equation bar, edit (e.g. `x - y`), **Enter** to refresh.
4. **Change the method** — Tab to **Method**, **Enter** to open the list, **↑** / **↓** to pick, **Enter** to confirm (**Esc** to cancel).
5. **Slope field** — Tab to **graph**, **Enter** to cycle **solution** → **slope** → **both** (not available with **All three methods**). In **slope** mode, edit **x min** / **x max** / **y min** / **y max** (defaults match the solution window); **Enter** to apply.
6. **Several initial values** — Tab to **y₀ family**, **Enter** to turn it **ON**, set **y₀ start**, **y₀ end**, and **y₀ #** (max 15), **Enter** to update.
7. **Export** — Tab to **Save** (or **`s`**), fill in filename / **h** / **points**, **Enter** to save under `exported/`.
8. **Quit** — **Quit** + **Enter**, **`q`**, or **Ctrl+C**.

Check the footer if something fails.

---

## 3. Keyboard reference

| Key | Action |
|-----|--------|
| **Tab**, **↑**, **↓** | Move focus |
| **Enter** | Apply / confirm |
| **Space** | Same as **Enter**, except in text fields (types a space) |
| **Esc** | Cancel dialog or menu |
| **s**, **q** | Export / quit (ignored while typing in a field) |
| **Ctrl+C** | Quit |

In the export dialog, **Enter** saves and **Esc** cancels. In the method menu, **↑** / **↓** choose, **Enter** confirms.

---

## 4. Equation syntax

You only type the right-hand side **F**; the app shows `y' = F`.

| You can write | Meaning |
|---------------|---------|
| `x`, `y` | Variables |
| `e`, `e^x` | Euler’s number and exponentials |
| `2y`, `xy`, `2(x+y)` | Implicit multiplication |
| `sin(x)`, `cos(y)`, `exp(x)`, `ln(x)`, `sqrt(x)` | Standard functions |
| `y^3`, `x^2`, `sin(x)^2` | Powers (use parentheses on function results: `sin(x)^2`, not `sin^2(x)`) |
| `y - y^3` | Example: cubic ODE |

You may paste a full equation such as `y'=y-y^3`; the app strips the `y' =` prefix.

Supported functions include **sin**, **cos**, **tan**, **exp**, **ln**, **log**, **sqrt**, **abs**, and the usual hyperbolic and inverse trig names. Function names must be followed by parentheses, e.g. `sin(x)` not `sin x`.

---

## 5. Parameters (sidebar)

| Field | Meaning |
|-------|---------|
| **x₀** | Start of the x-interval |
| **y₀** / **y₀ start** | Initial value (single **y₀** when family is OFF; start of the range when ON) |
| **y₀ end**, **y₀ #** | Shown when **y₀ family** is ON: equally spaced **y₀** from start to end (**y₀ #** values, max 15) |
| **x_end** | End of the x-interval (**> x₀** integrates forward, **< x₀** backward) |
| **h** | Fixed step size for integration (must be **> 0**) |

Smaller **h** means more accurate curves but slower updates. The graph subsamples to at most about 800 points per curve for drawing.

---

## 6. Methods

- **Euler** — simplest; fastest per step, least accurate.
- **Improved Euler** — predictor–corrector (Heun-type).
- **Runge–Kutta (RK4)** — usually the most accurate for a given **h**.
- **All three methods** — three stacked plots, one per method.

---

## 7. Export format

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

## 8. Running again later

From the project folder:

```bash
cargo run --release
```

No need to pass `--release` every time, but it is recommended for smooth graph updates.

---

## 9. Troubleshooting

| Problem | What to try |
|---------|-------------|
| `cargo: command not found` | Install Rust via [rustup.rs](https://rustup.rs), restart the terminal |
| Compile errors after updating the repo | `cargo build --release` and read the error line; ensure you are in the project root (folder with `Cargo.toml`) |
| Graph does not update | Press **Enter** after editing the equation or a number field; check the red error text in the footer |
| `function 'sin' must be called with parentheses` | Write `sin(x)` instead of `sin x` or `sin^2(x)` |
| Blank or garbled UI | Use a larger terminal window; avoid resizing while the app runs |
| Export failed | Check that the filename is not empty; ensure `x_end ≠ x₀` and `h > 0` |
