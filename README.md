# Kestrel Flight Simulator

This is a little flight sim made in Bevy using Rust.

This project is formerly known as `bevy_fs`, but it was renamed due to
likelyhood of confusion with a file system crate.

It is still in early development, but it's getting somewhere...

## Highlights

- Support for both keyboard and gamepad (gamepad is recommended)
- Collisions using `avian3d` and the flight dynamics model crate
  [`avian_fdm`](https://github.com/viccuad/avian_fdm)
- Terrain with real-world elevation data (down to 1m precision), and a
  continuously updating quadtree
- 3D cockpits with clickable buttons and screens displaying flight information
- A settings file - `settings.json`
- 3 aircraft to fly all around the world
- A round earth with variable sun positions

## Preview

![(Kestrel Flight Simulator: Preview 1)](./docs/preview-1.png)
![(Kestrel Flight Simulator: Preview 2)](./docs/preview-2.png)
![(Kestrel Flight Simulator: Preview 3)](./docs/preview-3.png)

## Community

- [Discord](https://discord.gg/epMBz5m2Ad)
- [Fluxer](https://fluxer.gg/hDS7y2y7)

## Run


### Using a precompiled binary

**Download the archive from [here](https://github.com/wesfly/kestrel/releases),
unzip it, and run the precompiled executable.**

### Compiling it yourself

#### Prerequisites

- **Rust toolchain:** [Install Rust](https://rust-lang.org/tools/install)
- [`git-lfs`](https://git-lfs.com/)

#### Running Kestrel Flight Simulator

Compiling the project will take a while and use ~10 GB of memory.

```bash
git clone https://github.com/wesfly/kestrel.git
cd kestrel
cargo run --release
```

## Terrain

Terrain data will be stored in the `.user/cache/` folder after the first fetch
(the game window will be unresponsive while fetching). There is a good amount
of locations you can choose from the menu. If you still want to fly in a custom
location, you can do so by setting the coordinates in `settings.json` and
leaving out the location choice in the menu. The simulator will fall back to
the coordinates stored in the settings.

The terrain zoom goes from 1 to 15, with 15 being zoomed in the furthest. Note
that not everywhere a zoom of 15 is available, but 12 is a safe zoom to use in
any location. You can find more about the zoom levels in different regions here:
<https://mapterhorn.com/coverage>.

## Buildings

Buildings are still an experimental feature, mainly due to the (sometimes
severe) frame rate losses. Some cities like Jakarta, Indonesia may freeze the
simulator. Consider this implementation a temporary solution.

## Controls

### Pause

Press `P` to unpause the sim.

### General

- `F3` to take a screenshot (output: `screenshots/user`)
- `ESC` to return to menu
- `HJKL` to control the sun position

### Camera

- `RMB + drag` to orbit camera
- `C` to switch the camera view
- Mouse wheel to zoom camera
- `R` to reset camera

### Aircraft

- `M` to toggle engine
- `G` to raise or lower landing gear
- `Z` to brake
- `=` to toggle parking brake
- `1` for position lights
- `2` for strobe lights
- `3` for formation lights
- `4` for anti-collision lights

You can change the control device in the settings.

### Some additional bindings for different steering devices

#### Controller

- Left stick to throttle and yaw
- Right stick to pitch and roll

#### HOTAS (Hands On Throttle And Stick)

- Left stick to steer
- `DPadUp` and `DPadDown` to throttle up and down respectively

#### Keyboard

- `WASDQE` to steer
- `PgUp` and `PgDown` to throttle up and down respectively

## Troubleshooting

### Broken Git LFS on Codeberg

```bash
Error downloading object: <some object>: Smudge error: Error downloading <some file path>: [<some commit>] Not Found: [404] Not Found
```

```bash
GIT_LFS_SKIP_SMUDGE=1 git reset --hard HEAD
git clean -fd
git lfs pull # You may need to download git-lfs first
```

## Contributing

PRs welcome.

If you have any problems regarding this repository, please report them as an
issue. Thank you!
