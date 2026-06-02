# Numerical ODE Approximater

Terminal UI for exploring first-order ODEs **y' = F(x, y)** with live graphing, method selection, and text export.

## Run

```bash
cargo run --release
```

## Main screen

- **Equation bar** — Type `F(x, y)` (shown as `y' = …`). Press **Enter** to refresh the graph.
- **Graph** — Solution curve(s); **All three methods** uses three stacked plots.
- **Sidebar**
  - **Method** — Euler, Improved Euler, Runge-Kutta, or all three.
  - **x₀**, **y₀**, **x_end**, **h** — Interval and step size.
  - **y₀ family** — When ON: plot several solutions with **y₀ start**, **y₀ end**, and **y₀ #** (up to 15 curves).
  - **Export** button — Save results to a text file (see below).

## Export

1. Tab/↑/↓ to focus **Export**, press **Enter**.
2. In the popup set **filename**, **h** (step size for this export), and **points** (how many samples).
3. Tab/↑/↓ between popup fields. **Enter** saves, **Esc** cancels.

Files go to **`exported/<name>.txt`**. Numbers are written with **6 significant figures**. Each method is one line; with all three methods you get three lines.

```text
y' = x + y
x0 = 0, y0 = 1, x_end = 2, h = 0.01, points = 100
# coordinates shown to 6 significant figures

Euler: (0, 1), (0.02, 1.0202), ...
Improved Euler: ...
Runge-Kutta: ...
```

Export **h** and **points** are independent of the sidebar values used for the live graph.

## Keys

| Key | Action |
|-----|--------|
| Tab / ↑ / ↓ | Move between fields |
| ← / → | Cursor inside text fields |
| Enter | Update graph (equation & params) / open export / confirm dialogs |
| Space | Open method dropdown (when Method focused) |
| Esc | Cancel export or method menu |
| q | Quit |

## Expression syntax

- Variables: `x`, `y`
- Implicit multiplication: `2y`, `xy`
- Trig: `sin(x)`, `cos(y)`, etc.
- Paste: `y'=x+2y` or type `x+2y` only
