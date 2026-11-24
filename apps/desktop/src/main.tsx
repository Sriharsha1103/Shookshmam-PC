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
import { getConfig, getDiagnostics, getDevices, getSnapshot, isTauri, onSnapshot } from './api';
import { CoolingPanel, LightingPanel, ProfilesPanel } from './panels';

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

type Tab = 'monitoring' | 'cooling' | 'lighting' | 'profiles';
const TABS: { id: Tab; label: string }[] = [
  { id: 'monitoring', label: 'Monitoring' },
  { id: 'cooling', label: 'Cooling' },
  { id: 'lighting', label: 'Lighting' },
  { id: 'profiles', label: 'Profiles' },
];

function formatValue(reading: SensorReading): string {
  if (reading.value === null) return '—';
  if (reading.status !== 'available' && reading.status !== 'stale') return '—';
  const v = reading.value;
  switch (reading.unit) {
    case 'bytes':
      return `${(v / (1024 ** 3)).toFixed(1)} GiB`;
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

function usable(r: SensorReading | null | undefined): r is SensorReading {
  return !!r && (r.status === 'available' || r.status === 'stale') && r.value !== null;
}

function Gauge({
  frac,
  accent,
  valueText,
  subText,
  size = 128,
}: {
  frac: number;
  accent: string;
  valueText: string;
  subText: string;
  size?: number;
}) {
  const stroke = 11;
  const cx = size / 2;
  const r = cx - stroke;
  const c = 2 * Math.PI * r;
  const f = Math.max(0, Math.min(1, Number.isFinite(frac) ? frac : 0));
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="gauge">
      <circle cx={cx} cy={cx} r={r} fill="none" stroke="#2a3340" strokeWidth={stroke} />
      <circle
        cx={cx}
        cy={cx}
        r={r}
        fill="none"
        stroke={accent}
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={`${f * c} ${c}`}
        transform={`rotate(-90 ${cx} ${cx})`}
      />
      <text x={cx} y={cx} textAnchor="middle" dominantBaseline="middle" className="gauge-value">
        {valueText}
      </text>
      <text x={cx} y={cx + 20} textAnchor="middle" className="gauge-sub">
        {subText}
      </text>
    </svg>
  );
}

function StatRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat">
      <span className="stat-label">{label}</span>
      <span className="stat-val">{value}</span>
    </div>
  );
}

function clockStr(r: SensorReading | null): string {
  if (!usable(r)) return 'n/a';
  const v = r.value as number;
  return v >= 1000 ? `${(v / 1000).toFixed(2)} GHz` : `${v.toFixed(0)} MHz`;
}
function tempStr(r: SensorReading | null): string {
  return usable(r) ? `${(r.value as number).toFixed(1)} °C` : 'n/a';
}

function getFriendlyTempLabel(sensor: SensorReading): string {
  const id = (sensor.id + sensor.name).toLowerCase();

  // CPU temperatures
  if (/tctl|package|core/.test(id)) return 'CPU Package';
  if (/tdie/.test(id)) return 'CPU Die';

  // GPU temperatures
  if (/edge/.test(id)) return 'GPU Edge';
  if (/junction|hotspot/.test(id)) return 'GPU Hotspot';
  if (/mem.*temp|temperature.*mem|mem.*gpu/.test(id)) return 'GPU Memory';

  // Fallback: use original name if it's readable, otherwise create one
  if (sensor.name && sensor.name.length < 50) {
    return sensor.name;
  }

  return `Sensor ${sensor.id}`;
}

function categorizeTempSensor(temp: SensorReading): 'cpu' | 'gpu' | 'motherboard' | 'storage' | 'other' {
  const id = (temp.id + temp.name).toLowerCase();
  if (/cpu|core|package|tctl|k10/.test(id)) return 'cpu';
  if (/gpu|edge|hotspot|junction|memory|junction.*gpu|mem.*gpu|amdgpu/.test(id)) return 'gpu';
  if (/motherboard|board|chipset|vrm|phase|ambient/.test(id)) return 'motherboard';
  if (/nvme|ssd|drive|storage|disk|temp/.test(id)) return 'storage';
  return 'other';
}

function findEssentialTemps(temps: SensorReading[]): {
  cpu: SensorReading | null;
  gpuEdge: SensorReading | null;
  gpuJunction: SensorReading | null;
  gpuMem: SensorReading | null;
} {
  const essential: {
    cpu: SensorReading | null;
    gpuEdge: SensorReading | null;
    gpuJunction: SensorReading | null;
    gpuMem: SensorReading | null;
  } = { cpu: null, gpuEdge: null, gpuJunction: null, gpuMem: null };

  // CPU: prefer Package/Tctl, then any CPU-like sensor
  const cpuSensors = temps.filter((t) => /package|tctl|cpu/.test((t.id + t.name).toLowerCase()));
  if (cpuSensors.length > 0) {
    essential.cpu = cpuSensors[0];
  }

  // GPU Edge
  const edgeSensors = temps.filter((t) => /edge/.test((t.id + t.name).toLowerCase()));
  if (edgeSensors.length > 0) {
    essential.gpuEdge = edgeSensors[0];
  }

  // GPU Junction
  const juncSensors = temps.filter((t) => /junction|hotspot/.test((t.id + t.name).toLowerCase()));
  if (juncSensors.length > 0) {
    essential.gpuJunction = juncSensors[0];
  }

  // GPU Memory
  const memSensors = temps.filter((t) => /mem.*temp|temperature.*mem|mem.*gpu/.test((t.id + t.name).toLowerCase()));
  if (memSensors.length > 0) {
    essential.gpuMem = memSensors[0];
  }

  return essential;
}

function TemperatureAccordion({ additionalTemps }: { additionalTemps: SensorReading[] }) {
  const [expanded, setExpanded] = React.useState(false);

  if (additionalTemps.length === 0) return null;

  const grouped: Record<string, SensorReading[]> = { cpu: [], gpu: [], motherboard: [], storage: [], other: [] };
  for (const temp of additionalTemps) {
    const cat = categorizeTempSensor(temp);
    grouped[cat].push(temp);
  }

  return (
    <div style={{ marginTop: '12px' }}>
      <button
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
        style={{
          background: 'none',
          border: 'none',
          color: '#8292a5',
          fontSize: '13px',
          fontWeight: 600,
          cursor: 'pointer',
          padding: '0',
          textAlign: 'left',
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          letterSpacing: '0.16em',
        }}
      >
        <span style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)', display: 'inline-block', transition: 'transform 0.2s' }}>
          ▶
        </span>
        ADDITIONAL SENSORS ({additionalTemps.length})
      </button>

      {expanded && (
        <div style={{ marginTop: '12px', paddingLeft: '8px', borderLeft: '2px solid #2a3340' }}>
          {Object.entries(grouped).map(([category, sensors]) =>
            sensors.length > 0 ? (
              <div key={category} style={{ marginBottom: '10px' }}>
                <div style={{ color: '#7f8fa3', fontSize: '11px', fontWeight: 600, marginBottom: '6px', textTransform: 'uppercase', letterSpacing: '0.12em' }}>
                  {category}
                </div>
                {sensors.map((temp) => (
                  <div
                    key={temp.id}
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      fontSize: '12px',
                      padding: '4px 0',
                      borderBottom: '1px solid #1a2332',
                    }}
                  >
                    <span style={{ color: '#8292a5' }}>{getFriendlyTempLabel(temp)}</span>
                    <span style={{ color: '#eef3f8', fontWeight: 500 }}>{formatValue(temp)}</span>
                  </div>
                ))}
              </div>
            ) : null,
          )}
        </div>
      )}
    </div>
  );
}

function MonitoringView({
  snapshot,
  error,
  devices = {},
}: {
  snapshot: Snapshot | null;
  error: string | null;
  devices: Record<string, string>;
}) {
  const rs = snapshot?.readings ?? [];
  const byId = (id: string) => rs.find((x) => x.id === id) ?? null;
  const temps = rs.filter((x) => x.category === 'temperature');
  const fans = rs.filter((x) => x.category === 'fan_speed');

  const cpuUsage = byId('cpu.utilization.total');
  const cpuClock = byId('cpu.frequency.core');
  const cpuCores = byId('cpu.cores');
  const cpuTemp =
    temps.find((t) => usable(t) && /cpu|tctl|k10|package|core/i.test(t.name + t.id)) ?? null;

  const gpuUsage = byId('gpu.utilization.core') ?? byId('gpu.utilization.total');
  const gpuClock = byId('gpu.frequency.core');
  const gpuMemClock = byId('gpu.frequency.memory');
  const gpuTempEdge = byId('gpu.temp.edge');
  const gpuTempJunction = byId('gpu.temp.junction');
  const gpuTempMemory = byId('gpu.temp.memory');
  const gpuMemUsed = byId('gpu.memory.used');
  const gpuMemTotal = byId('gpu.memory.total');
  const gpuTemp =
    temps.find((t) => usable(t) && /gpu|hotspot|edge|amdgpu/i.test(t.name + t.id)) ?? null;

  const memUsed = byId('mem.used');
  const memTotal = byId('mem.total');
  const memAvail = byId('mem.available');

  const GiB = 1024 ** 3;
  const gpuMemUsedGib = usable(gpuMemUsed) ? (gpuMemUsed.value as number) / GiB : null;
  const gpuMemTotalGib = usable(gpuMemTotal) ? (gpuMemTotal.value as number) / GiB : null;
  const gpuMemFrac = gpuMemUsedGib !== null && gpuMemTotalGib ? gpuMemUsedGib / gpuMemTotalGib : 0;

  const cpuPct = usable(cpuUsage) ? (cpuUsage.value as number) : null;
  const gpuPct = usable(gpuUsage) ? (gpuUsage.value as number) : null;
  const memUsedGib = usable(memUsed) ? (memUsed.value as number) / GiB : null;
  const memTotalGib = usable(memTotal) ? (memTotal.value as number) / GiB : null;
  const memFrac = memUsedGib !== null && memTotalGib ? memUsedGib / memTotalGib : 0;

  if (error) {
    return (
      <section className="notice">
        <strong>Telemetry unavailable.</strong>
        <br />
        {error}
      </section>
    );
  }
  if (rs.length === 0) {
    return <section className="notice">Awaiting first telemetry snapshot…</section>;
  }

  return (
    <>
      <div className="panels">
        <article className="panel">
          <div className="panel-head">
            <span className="panel-title">CPU</span>
            <span className="panel-subtitle" title={cpuUsage ? devices[cpuUsage.device_id] || '' : ''}>{cpuUsage ? devices[cpuUsage.device_id] || '' : ''}</span>
          </div>
          <div className="panel-body">
            <Gauge
              frac={(cpuPct ?? 0) / 100}
              accent="#6fd3a0"
              valueText={cpuPct !== null ? `${cpuPct.toFixed(0)}%` : 'n/a'}
              subText="USAGE"
            />
            <div className="stats">
              <StatRow label="Temp" value={tempStr(cpuTemp)} />
              <StatRow label="Clock" value={clockStr(cpuClock)} />
              <StatRow label="Cores" value={usable(cpuCores) ? `${cpuCores.value}` : 'n/a'} />
            </div>
          </div>
        </article>

        <article className="panel">
          <div className="panel-head">
            <span className="panel-title">GPU</span>
            <span className="panel-subtitle" title={gpuUsage ? devices[gpuUsage.device_id] || '' : ''}>{gpuUsage ? devices[gpuUsage.device_id] || '' : ''}</span>
          </div>
          <div className="panel-body">
            <Gauge
              frac={(gpuPct ?? 0) / 100}
              accent="#8c78ff"
              valueText={gpuPct !== null ? `${gpuPct.toFixed(0)}%` : 'n/a'}
              subText="USAGE"
            />
            <div className="stats">
              <StatRow label="Core Clock" value={clockStr(gpuClock)} />
              <StatRow label="Mem Clock" value={clockStr(gpuMemClock)} />
              <StatRow label="Temp (Edge)" value={tempStr(gpuTempEdge)} />
              {usable(gpuTempJunction) && <StatRow label="Temp (Junct.)" value={tempStr(gpuTempJunction)} />}
              {usable(gpuTempMemory) && <StatRow label="Temp (Mem)" value={tempStr(gpuTempMemory)} />}
              {gpuPct === null && <small className="muted">Not exposed on this platform yet.</small>}
            </div>
          </div>
        </article>

        <article className="panel">
          <div className="panel-head">
            <span className="panel-title">GPU VRAM</span>
            <span className="panel-subtitle" title={gpuMemUsed ? devices[gpuMemUsed.device_id] || '' : ''}>{gpuMemUsed ? devices[gpuMemUsed.device_id] || '' : ''}</span>
          </div>
          <div className="panel-body">
            <Gauge
              frac={gpuMemFrac}
              accent="#a65dd9"
              valueText={gpuMemUsedGib !== null ? `${gpuMemUsedGib.toFixed(1)}` : 'n/a'}
              subText="GiB USED"
            />
            <div className="stats">
              <StatRow label="Total" value={gpuMemTotalGib !== null ? `${gpuMemTotalGib.toFixed(1)} GiB` : 'n/a'} />
              <StatRow
                label="In use"
                value={gpuMemFrac ? `${(gpuMemFrac * 100).toFixed(0)}%` : 'n/a'}
              />
              {gpuMemUsedGib === null && <small className="muted">Not exposed on this platform yet.</small>}
            </div>
          </div>
        </article>

        <article className="panel">
          <div className="panel-head">
            <span className="panel-title">MEMORY</span>
            <small className="muted">{memUsed?.source ?? ''}</small>
          </div>
          <div className="panel-body">
            <Gauge
              frac={memFrac}
              accent="#e8b64b"
              valueText={memUsedGib !== null ? `${memUsedGib.toFixed(1)}` : 'n/a'}
              subText="GiB USED"
            />
            <div className="stats">
              <StatRow label="Total" value={memTotalGib !== null ? `${memTotalGib.toFixed(1)} GiB` : 'n/a'} />
              <StatRow
                label="Available"
                value={usable(memAvail) ? `${((memAvail.value as number) / GiB).toFixed(1)} GiB` : 'n/a'}
              />
              <StatRow
                label="In use"
                value={memFrac ? `${(memFrac * 100).toFixed(0)}%` : 'n/a'}
              />
            </div>
          </div>
        </article>
      </div>

      <div className="panels">
        <article className="panel">
          <div className="panel-head">
            <span className="panel-title">TEMPERATURES</span>
          </div>
          <div className="tele-list">
            {temps.length === 0 && <small className="muted">No temperature sensors exposed.</small>}
            {temps.length > 0 && (() => {
              const essential = findEssentialTemps(temps);
              const essentialIds = new Set(
                Object.values(essential)
                  .filter((t) => t !== null)
                  .map((t) => t!.id),
              );
              const additionalTemps = temps.filter((t) => !essentialIds.has(t.id));

              return (
                <>
                  {essential.cpu && (
                    <div className="stat">
                      <span className="stat-label">{getFriendlyTempLabel(essential.cpu)}</span>
                      <span className="stat-val">{formatValue(essential.cpu)}</span>
                    </div>
                  )}
                  {essential.gpuEdge && (
                    <div className="stat">
                      <span className="stat-label">{getFriendlyTempLabel(essential.gpuEdge)}</span>
                      <span className="stat-val">{formatValue(essential.gpuEdge)}</span>
                    </div>
                  )}
                  {essential.gpuJunction && (
                    <div className="stat">
                      <span className="stat-label">{getFriendlyTempLabel(essential.gpuJunction)}</span>
                      <span className="stat-val">{formatValue(essential.gpuJunction)}</span>
                    </div>
                  )}
                  {essential.gpuMem && (
                    <div className="stat">
                      <span className="stat-label">{getFriendlyTempLabel(essential.gpuMem)}</span>
                      <span className="stat-val">{formatValue(essential.gpuMem)}</span>
                    </div>
                  )}
                  <TemperatureAccordion additionalTemps={additionalTemps} />
                </>
              );
            })()}
          </div>
        </article>

        <article className="panel">
          <div className="panel-head">
            <span className="panel-title">FANS</span>
          </div>
          <div className="tele-list">
            {fans.length === 0 && (
              <small className="muted">No fan tachometers exposed on this platform.</small>
            )}
            {fans.map((fn) => (
              <div key={fn.id} className="stat">
                <span className="stat-label">{fn.name}</span>
                <span className="stat-val">{formatValue(fn)}</span>
              </div>
            ))}
          </div>
        </article>
      </div>
    </>
  );
}

function App() {
  const [tab, setTab] = useState<Tab>('monitoring');
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [diag, setDiag] = useState<Diagnostics | null>(null);
  const [devices, setDevices] = useState<Record<string, string>>({});
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

        // Fetch device metadata and build a map of device_id -> device_name
        const deviceList = await getDevices();
        const deviceMap = Object.fromEntries(deviceList.map((d) => [d.id, d.name]));
        setDevices(deviceMap);

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

  const okProviders = diag?.providers.filter((p) => p.ok).length ?? 0;

  return (
    <main>
      <header>
        <div>
          <p className="eyebrow">HARDWARE CONTROL CENTRE</p>
          <h1>Shookshmam PC</h1>
          <p className="muted">Phases 1–5 · monitoring + gated control</p>
        </div>
        <span className="pill">{config?.read_only === false ? 'CONTROL' : 'READ-ONLY MODE'}</span>
      </header>

      <nav className="tabs">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`tab ${tab === t.id ? 'tab-active' : ''}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      {config?.simulated && (
        <section className="notice notice-warn">
          <strong>Simulated data.</strong> No validated hardware provider is active, so monitoring
          values come from the built-in simulation (<code>source: simulation</code>). They are not
          real sensor readings.
        </section>
      )}

      {tab === 'monitoring' && <MonitoringView snapshot={snapshot} error={error} devices={devices} />}
      {tab === 'cooling' && <CoolingPanel />}
      {tab === 'lighting' && <LightingPanel />}
      {tab === 'profiles' && <ProfilesPanel />}

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
