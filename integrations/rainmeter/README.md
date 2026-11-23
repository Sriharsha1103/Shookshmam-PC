# Shookshmam PC — Rainmeter skin

A desktop telemetry widget (circular CPU/GPU gauges + stats) fed by the
Shookshmam PC desktop app. The app writes a small `telemetry.txt` file every
poll; this Rainmeter skin reads it and draws the gauges. Any external widget that
can read a `key=value` text file can consume the same data.

Layout: two circular gauges (CPU and GPU usage) with temperatures underneath,
plus a RAM / VRAM / SSD / CPU-clock stats row.

## How it works

Each telemetry poll, the app writes a flat file to its app-data directory:

```
%APPDATA%\in.shookshmam.pc\telemetry.txt
```

(On this machine that resolves to
`C:\Users\<you>\AppData\Roaming\in.shookshmam.pc\telemetry.txt`.)

Contents are simple `key=value` lines, e.g.:

```
cpu_temp=52.3
cpu_usage=25
cpu_clock=4150
cpu_power=34.2
gpu_temp=41.0
gpu_hotspot=82.0
gpu_usage=19
gpu_clock=30
vram_used=2.3
vram_total=
mem_used=14.6
mem_total=31.8
ssd_temp=33.1
simulated=1
```

A blank value means the sensor isn't exposed (never a fabricated `0`).
`simulated=1` means the values come from the built-in simulation provider, not
real hardware — until a provider is validated, the gauges show simulated data,
and the skin displays a `SIMULATED` tag.

## Install

1. Install [Rainmeter](https://www.rainmeter.net/) (Windows).
2. Copy the `ShookshmamPC` folder from here into your Rainmeter skins folder:
   `Documents\Rainmeter\Skins\ShookshmamPC`.
3. Open `ShookshmamPC\ShookshmamPC.ini` and set `TelemetryPath` (in the
   `[Variables]` section) to your telemetry file, **using forward slashes**:
   ```
   TelemetryPath=C:/Users/<you>/AppData/Roaming/in.shookshmam.pc/telemetry.txt
   ```
4. In Rainmeter: **Refresh all**, then load `ShookshmamPC > ShookshmamPC.ini`.
5. Run the Shookshmam PC desktop app so the file keeps updating. The skin
   refreshes once per second.

## Notes

- The skin is read-only: it only displays telemetry. It issues no hardware
  control and needs no elevation.
- If the gauges stay blank, check that (a) the app is running, (b) `TelemetryPath`
  points at the real file (forward slashes), and (c) the file exists and updates
  (open it in a text editor while the app runs).
- Field order in `telemetry.txt` is stable; the skin's WebParser `RegExp` depends
  on it. If you add fields in the app, append them and extend the RegExp.
