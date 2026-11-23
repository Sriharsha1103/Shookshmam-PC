import React from 'react';
import { createRoot } from 'react-dom/client';
import './style.css';

function App() {
  return <main>
    <header><div><p className="eyebrow">HARDWARE CONTROL CENTRE</p><h1>Shookshmam PC</h1><p className="muted">Phase 1 · Monitoring foundation</p></div><span className="pill">READ-ONLY MODE</span></header>
    <section className="notice"><strong>Hardware adapters are not connected yet.</strong><br/>This scaffold is a UI starting point. Live readings will appear after provider implementation and validation.</section>
    <section className="grid">{['CPU','GPU','Memory','Storage','Motherboard','Cooling'].map(x=><article key={x}><p className="muted">{x.toUpperCase()}</p><h2>—</h2><small>Awaiting sensor provider</small></article>)}</section>
    <footer>Windows · Bazzite Linux · Local telemetry · 30-day history planned</footer>
  </main>;
}

createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
