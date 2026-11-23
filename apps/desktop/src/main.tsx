import React, { useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import './style.css';
import type {
  AppConfig,
  Diagnostics,
  SensorCategory,
  SensorReading,
  SensorStatus,
  Snapshot,
} from './types';
import { getConfig, getDiagnostics, getSnapshot, isTauri, onSnapshot } from './api';

const CATEGORY_ORDER: SensorCategory[] = [
  'temperature',
  'utilization',
  'frequency',
  'power',
  'fan_speed',
  'voltage',
  'memory',
  'storage_health',
];

const CATEGORY_LABEL: Record<SensorCategory, string> = {
  temperature: 'Temperatures',
  utilization: 'Utilization',
  frequency: 'Clocks',
  power: 'Power',
  fan_speed: 'Fans',
  voltage: 'Voltages',
  memory: 'Memory',
  storage_health: 'Storage',
};

const STATUS_LABEL: Record<SensorStatus, string> = {
  available: 'live',
  stale: 'stale',
  unavailable: 'unavailable',
  unsupported: 'unsupported',
  permission_denied: 'no permission',
  error: 'error',
};

function formatValue(reading: SensorReading): string {
  if (reading.value === null || !reading.status) return '—';
  if (!reading.status.match(/available|stale/)) return '—';
  const v = reading.value;
  switch (reading.unit) {
    case 'bytes':
      return `${(v / 1e9).toFixed(1)} GB`;
    case '%':
      return `${v.toFixed(0)}%`;
    case 'MHz':
      return v >= 1000 ? `${(v / 1000).toFixed(2)} GHz` : `${v.toFixed(0)} MHz`;
    case 'C':
      return `${v.toFixed(1)} °C`;
    case 'W':
      return `${v.toFixed(1)} W`;
    case 'V':
      return `${v.toFixed(3)} V`;
    case 'RPM':
      return `${v.toFixed(0)} RPM`;
    default:
      return reading.unit ? `${v} ${reading.unit}` : `${v}`;
  }
}

function Chip({ status }: { status: SensorStatus }) {
  return <span className={`chip chip-${status}`}>{STATUS_LABEL[status]}</span>;
}

function ReadingCard({ reading }: { reading: SensorReading }) {
  const usable = reading.status === 'available' || reading.status === 'stale';
  return (
    <article className={usable ? '' : 'card-nodata'}>
      <div className="card-head">
        <p className="muted">{reading.name.toUpperCase()}</p>
        <Chip status={reading.status} />
      </div>
      <h2>{formatValue(reading)}</h2>
      <small>
        {reading.source}
        {reading.quality_note ? ` · ${reading.quality_note}` : ''}
      </small>
    </article>
  );
}

function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [diag, setDiag] = useState<Diagnostics | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri) {
      setError('Not running inside the Tauri shell — start the app with `npm run tauri dev`.');
      return;
    }
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        setConfig(await getConfig());
        setSnapshot(await getSnapshot());
        setDiag(await getDiagnostics());
        unlisten = await onSnapshot((snap) => {
          setSnapshot(snap);
          getDiagnostics().then(setDiag).catch(() => {});
        });
      } catch (e) {
        setError(String(e));
      }
    })();
    return () => unlisten?.();
  }, []);

  const grouped = useMemo(() => {
    const map = new Map<SensorCategory, SensorReading[]>();
    for (const r of snapshot?.readings ?? []) {
      const list = map.get(r.category) ?? [];
      list.push(r);
      map.set(r.category, list);
    }
    return map;
  }, [snapshot]);

  const okProviders = diag?.providers.filter((p) => p.ok).length ?? 0;

  return (
    <main>
      <header>
        <div>
          <p className="eyebrow">HARDWARE CONTROL CENTRE</p>
          <h1>Shookshmam PC</h1>
          <p className="muted">Phase 1 · Monitoring foundation</p>
        </div>
        <span className="pill">{config?.read_only === false ? 'CONTROL' : 'READ-ONLY MODE'}</span>
      </header>

      {config?.simulated && (
        <section className="notice notice-warn">
          <strong>Simulated data.</strong> No validated hardware provider is active, so the values
          below come from the built-in simulation (<code>source: simulation</code>). They are not
          real sensor readings.
        </section>
      )}

      {error && (
        <section className="notice">
          <strong>Telemetry unavailable.</strong>
          <br />
          {error}
        </section>
      )}

      {CATEGORY_ORDER.filter((c) => grouped.has(c)).map((category) => (
        <section key={category} className="group">
          <h3 className="group-title">{CATEGORY_LABEL[category]}</h3>
          <div className="grid">
            {grouped
              .get(category)!
              .sort((a, b) => a.name.localeCompare(b.name))
              .map((r) => (
                <ReadingCard key={r.id} reading={r} />
              ))}
          </div>
        </section>
      ))}

      {!error && (snapshot?.readings.length ?? 0) === 0 && (
        <section className="notice">Awaiting first telemetry snapshot…</section>
      )}

      <footer>
        {okProviders} active provider{okProviders === 1 ? '' : 's'} ·{' '}
        {config ? `${config.sample_interval_ms} ms sampling` : 'sampling…'} ·{' '}
        {config ? `${config.history_retention_days}-day history` : 'history'} · Local only
        {snapshot ? ` · updated ${new Date(snapshot.timestamp_unix_ms).toLocaleTimeString()}` : ''}
      </footer>
    </main>
  );
}

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
