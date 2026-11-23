import React, { useEffect, useMemo, useState } from 'react';
import type {
  ApplyPlan,
  ChannelInfo,
  Effect,
  FanCurve,
  LightingZone,
  Profile,
  Rgb,
} from './types';
import {
  getFanChannels,
  getFanCurves,
  getLightingZones,
  getSupportBundle,
  isTauri,
  listProfiles,
  previewEffect,
  previewProfile,
} from './api';

function GatedNotice({ children }: { children: React.ReactNode }) {
  return (
    <section className="notice notice-warn">
      <strong>Read-only.</strong> {children}
    </section>
  );
}

function useAsync<T>(load: () => Promise<T>, deps: React.DependencyList): [T | null, string | null] {
  const [value, setValue] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!isTauri) {
      setError('Not running inside the Tauri shell.');
      return;
    }
    let active = true;
    load()
      .then((v) => active && setValue(v))
      .catch((e) => active && setError(String(e)));
    return () => {
      active = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return [value, error];
}

// --- Cooling ---

function CurveSvg({ curve }: { curve: FanCurve }) {
  const w = 160;
  const h = 72;
  const tMin = 20;
  const tMax = 100;
  const pts = curve.points
    .map((p) => {
      const x = ((p.temperature_c - tMin) / (tMax - tMin)) * w;
      const y = h - (p.duty_percent / 100) * h;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');
  return (
    <svg width={w} height={h} className="curve-svg" role="img" aria-label={`${curve.name} curve`}>
      <polyline points={pts} fill="none" stroke="#6fd3a0" strokeWidth="2" />
    </svg>
  );
}

export function CoolingPanel() {
  const [channels, chErr] = useAsync<ChannelInfo[]>(getFanChannels, []);
  const [curves, cvErr] = useAsync<FanCurve[]>(getFanCurves, []);
  const error = chErr ?? cvErr;

  return (
    <div>
      <GatedNotice>
        Fan control is disabled — no channel has passed hardware validation, so every duty write is
        refused. Curves and channels below are the configuration model.
      </GatedNotice>
      {error && <section className="notice">{error}</section>}

      <h3 className="group-title">Channels</h3>
      <div className="grid">
        {(channels ?? []).map((c) => (
          <article key={c.id} className="card-nodata">
            <div className="card-head">
              <p className="muted">{c.label.toUpperCase()}</p>
              <span className={`chip ${c.validated ? 'chip-available' : 'chip-unsupported'}`}>
                {c.validated ? 'validated' : 'unvalidated'}
              </span>
            </div>
            <h2>
              {c.min_duty.toFixed(0)}–{c.max_duty.toFixed(0)}%
            </h2>
            <small>{c.allow_stop ? 'fan stop supported' : 'fan stop disabled'}</small>
          </article>
        ))}
      </div>

      <h3 className="group-title">Curve presets</h3>
      <div className="grid">
        {(curves ?? []).map((cv) => (
          <article key={cv.id}>
            <div className="card-head">
              <p className="muted">{cv.name.toUpperCase()}</p>
              <span className="chip">{cv.source_policy}</span>
            </div>
            <CurveSvg curve={cv} />
            <small>
              min {cv.min_duty.toFixed(0)}% · hysteresis {cv.hysteresis_c}°C · ramp ↑
              {cv.ramp_up_pct_per_s}/s ↓{cv.ramp_down_pct_per_s}/s
            </small>
          </article>
        ))}
      </div>
    </div>
  );
}

// --- Lighting ---

const EFFECT_PRESETS: { label: string; effect: Effect }[] = [
  { label: 'Static red', effect: { kind: 'static', color: { r: 220, g: 40, b: 40 } } },
  { label: 'Color cycle', effect: { kind: 'color_cycle', speed: 5, brightness: 100 } },
  { label: 'Breathing', effect: { kind: 'breathing', color: { r: 80, g: 160, b: 255 }, speed: 4 } },
  {
    label: 'Gradient',
    effect: {
      kind: 'gradient',
      colors: [
        { r: 255, g: 0, b: 0 },
        { r: 0, g: 0, b: 255 },
      ],
      speed: 3,
    },
  },
  { label: 'Off', effect: { kind: 'off' } },
];

function rgbCss(c: Rgb): string {
  return `rgb(${c.r}, ${c.g}, ${c.b})`;
}

export function LightingPanel() {
  const [zones, zErr] = useAsync<LightingZone[]>(getLightingZones, []);
  const [selected, setSelected] = useState(0);
  const [swatches, setSwatches] = useState<Rgb[]>([]);
  const [previewErr, setPreviewErr] = useState<string | null>(null);
  const ledCount = 16;

  useEffect(() => {
    if (!isTauri) return;
    previewEffect(EFFECT_PRESETS[selected].effect, ledCount)
      .then(setSwatches)
      .catch((e) => setPreviewErr(String(e)));
  }, [selected]);

  return (
    <div>
      <GatedNotice>
        Lighting writes are disabled. Each zone's electrical type is unverified (shown as
        <em> unknown</em>); writes stay refused until the physical header type is confirmed.
      </GatedNotice>
      {zErr && <section className="notice">{zErr}</section>}

      <h3 className="group-title">Zones</h3>
      <div className="grid">
        {(zones ?? []).map((z) => (
          <article key={z.id} className="card-nodata">
            <div className="card-head">
              <p className="muted">{z.name.toUpperCase()}</p>
              <span className="chip chip-unsupported">{z.electrical_type.replace(/_/g, ' ')}</span>
            </div>
            <h2 style={{ fontSize: 18 }}>{z.connector}</h2>
            <small>
              {z.led_count === null ? 'LED count unknown' : `${z.led_count} LEDs`} ·{' '}
              {z.supported_effects.join(', ')}
            </small>
          </article>
        ))}
      </div>

      <h3 className="group-title">Effect preview</h3>
      <div className="effect-controls">
        {EFFECT_PRESETS.map((p, i) => (
          <button
            key={p.label}
            className={`tab ${i === selected ? 'tab-active' : ''}`}
            onClick={() => setSelected(i)}
          >
            {p.label}
          </button>
        ))}
      </div>
      {previewErr ? (
        <section className="notice">{previewErr}</section>
      ) : (
        <div className="swatches">
          {swatches.map((c, i) => (
            <span key={i} className="swatch" style={{ background: rgbCss(c) }} />
          ))}
        </div>
      )}
      <small className="muted">
        Preview is computed by the effect engine (first frame) — it drives no hardware.
      </small>
    </div>
  );
}

// --- Profiles ---

function PlanView({ plan }: { plan: ApplyPlan }) {
  return (
    <div className="plan">
      {!plan.platform_compatible && (
        <section className="notice notice-warn">Not compatible with this platform.</section>
      )}
      {plan.missing_capabilities.length > 0 && (
        <small className="muted">
          Missing capabilities: {plan.missing_capabilities.join(', ')}
        </small>
      )}
      <ul className="plan-list">
        {plan.changes.map((c, i) => (
          <li key={i}>
            <span className={`chip ${c.supported ? 'chip-available' : 'chip-unsupported'}`}>
              {c.supported ? 'apply' : 'skip'}
            </span>{' '}
            <strong>{c.kind}</strong> {c.target} → {c.value}
            {c.reason ? <small className="muted"> — {c.reason}</small> : null}
          </li>
        ))}
      </ul>
    </div>
  );
}

export function ProfilesPanel() {
  const [profiles, pErr] = useAsync<Profile[]>(listProfiles, []);
  const [plans, setPlans] = useState<Record<string, ApplyPlan>>({});
  const [bundle, setBundle] = useState<string | null>(null);

  const preview = (id: string) => {
    previewProfile(id)
      .then((plan) => setPlans((prev) => ({ ...prev, [id]: plan })))
      .catch(() => {});
  };

  const dumpBundle = () => {
    getSupportBundle()
      .then((b) => setBundle(JSON.stringify(b, null, 2)))
      .catch((e) => setBundle(String(e)));
  };

  const anyUnsupported = useMemo(
    () => Object.values(plans).some((p) => p.changes.some((c) => !c.supported)),
    [plans],
  );

  return (
    <div>
      <GatedNotice>
        Previewing a profile is safe: it compares the profile with the current hardware and shows
        what <em>would</em> change. Nothing is applied — hardware writes require validated
        capabilities that don't exist yet.
      </GatedNotice>
      {pErr && <section className="notice">{pErr}</section>}

      <div className="grid">
        {(profiles ?? []).map((p) => (
          <article key={p.id}>
            <div className="card-head">
              <p className="muted">{p.name.toUpperCase()}</p>
              <span className="chip">{p.preferred_control_mode}</span>
            </div>
            <small>{p.description}</small>
            <div style={{ marginTop: 10 }}>
              <button className="tab" onClick={() => preview(p.id)}>
                Preview changes
              </button>
            </div>
            {plans[p.id] && <PlanView plan={plans[p.id]} />}
          </article>
        ))}
      </div>

      {anyUnsupported && (
        <small className="muted">
          Components marked “skip” are unsupported on current hardware and would be left unchanged.
        </small>
      )}

      <h3 className="group-title">Diagnostics</h3>
      <button className="tab" onClick={dumpBundle}>
        Generate support bundle
      </button>
      {bundle && <pre className="bundle">{bundle}</pre>}
    </div>
  );
}
