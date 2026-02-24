import React from 'react';
import { createRoot } from 'react-dom/client';
import { ResponsiveBar } from '@nivo/bar';

const roots = new Map();
const POLICY_DISPLAY_ORDER = [
  'balanced',
  'random',
  'grind',
  'rest',
  'money',
  'luddite',
  'hedge',
  'clout',
  'social',
  'survivor',
  'self-destruct',
  'hermit',
  'accelerationist',
];

function sortPolicies(a, b) {
  const ai = POLICY_DISPLAY_ORDER.indexOf(String(a));
  const bi = POLICY_DISPLAY_ORDER.indexOf(String(b));
  const aRank = ai === -1 ? Number.MAX_SAFE_INTEGER : ai;
  const bRank = bi === -1 ? Number.MAX_SAFE_INTEGER : bi;
  if (aRank !== bRank) return aRank - bRank;
  return String(a).localeCompare(String(b));
}

function cssVar(name, fallback) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function nivoTheme() {
  const text = cssVar('--text-muted', '#71717a');
  const grid = cssVar('--border', '#1a1a24');
  const panel = cssVar('--bg-panel', '#050505');
  const strong = cssVar('--border-strong', '#22222c');
  const mono = cssVar('--font-mono', 'Berkeley Mono Variable, Berkeley Mono, TX-02, monospace');
  return {
    textColor: text,
    axis: {
      domain: { line: { stroke: strong, strokeWidth: 1 } },
      ticks: {
        line: { stroke: strong, strokeWidth: 1 },
        text: { fill: text, fontSize: 10, fontFamily: mono },
      },
      legend: { text: { fill: text, fontSize: 10, fontFamily: mono } },
    },
    grid: { line: { stroke: grid, strokeWidth: 1 } },
    tooltip: {
      container: {
        background: panel,
        color: cssVar('--text-main', '#d4d4d8'),
        border: `1px solid ${strong}`,
        fontSize: '10px',
        fontFamily: mono,
      },
    },
  };
}

function getRoot(id) {
  const el = document.getElementById(id);
  if (!el) return null;
  const cached = roots.get(id);
  if (cached && cached.el === el) {
    return cached.root;
  }
  const root = createRoot(el);
  roots.set(id, { el, root });
  return root;
}

function renderEmpty(id, message) {
  const root = getRoot(id);
  if (!root) return;
  root.render(
    React.createElement(
      'div',
      { className: 'nivo-empty' },
      message
    )
  );
}

function renderBar(id, props) {
  const root = getRoot(id);
  if (!root) return;
  root.render(React.createElement(ResponsiveBar, props));
}

function endingTitle(endingId, endingTitles) {
  if (endingTitles && Object.prototype.hasOwnProperty.call(endingTitles, endingId)) {
    return String(endingTitles[endingId] || endingId);
  }
  return String(endingId || 'Unknown');
}

function endingData(rows, endingTitles) {
  const counts = new Map();
  rows.forEach(r => {
    const endingId = String(r.ending || 'Unknown');
    counts.set(endingId, (counts.get(endingId) || 0) + 1);
  });
  const total = rows.length || 1;
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .slice(0, 12)
    .map(([endingId, runs]) => ({
      endingId,
      ending: endingTitle(endingId, endingTitles),
      share: Math.round((runs / total) * 1000) / 10,
      runs,
    }));
}

function policyData(rows) {
  const byPolicy = new Map();
  rows.forEach(r => {
    const policy = String(r.policy || 'unknown');
    const isDeath = typeof r.is_death === 'boolean'
      ? r.is_death
      : ['Broke', 'BurnedOut'].includes(String(r.ending || ''));
    const agg = byPolicy.get(policy) || { runs: 0, survive: 0 };
    agg.runs += 1;
    if (!isDeath) agg.survive += 1;
    byPolicy.set(policy, agg);
  });
  return [...byPolicy.entries()]
    .sort((a, b) => sortPolicies(a[0], b[0]))
    .map(([policy, agg]) => {
      const survivePct = (agg.survive / agg.runs) * 100;
      const deathPct = 100 - survivePct;
      return {
        policy,
        Survive: Math.round(survivePct * 10) / 10,
        Death: Math.round(deathPct * 10) / 10,
      };
    });
}

function turnData(rows) {
  const counts = new Map();
  rows.forEach(r => {
    const turn = Number(r.turn || 0);
    if (!Number.isFinite(turn) || turn <= 0) return;
    const isDeath = typeof r.is_death === 'boolean'
      ? r.is_death
      : ['Broke', 'BurnedOut'].includes(String(r.ending || ''));
    const agg = counts.get(turn) || { Completed: 0, Death: 0 };
    if (isDeath) agg.Death += 1;
    else agg.Completed += 1;
    counts.set(turn, agg);
  });
  return [...counts.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([turn, agg]) => ({ turn: String(turn), ...agg }));
}

function renderSimCharts(rows, endingTitles) {
  if (!Array.isArray(rows) || rows.length === 0) {
    renderEmpty('sim-ending-chart', 'No Sim Data');
    renderEmpty('sim-policy-chart', 'No Sim Data');
    renderEmpty('sim-turn-chart', 'No Sim Data');
    return;
  }

  const theme = nivoTheme();

  const endings = endingData(rows, endingTitles);
  renderBar('sim-ending-chart', {
    data: endings,
    indexBy: 'ending',
    keys: ['share'],
    layout: 'horizontal',
    margin: { top: 8, right: 12, bottom: 28, left: 170 },
    valueScale: { type: 'linear' },
    indexScale: { type: 'band', round: true },
    colors: [cssVar('--info', '#22d3ee')],
    borderRadius: 1,
    enableLabel: false,
    axisTop: null,
    axisRight: null,
    axisBottom: {
      tickSize: 3,
      tickPadding: 5,
      legend: 'share %',
      legendPosition: 'middle',
      legendOffset: 24,
    },
    axisLeft: {
      tickSize: 0,
      tickPadding: 8,
      tickRotation: 0,
    },
    gridYValues: [],
    theme,
    role: 'application',
    ariaLabel: 'Top ending share chart',
    tooltip: d => `${d.data.ending}: ${d.value}% (${d.data.runs} runs)`,
  });

  const policies = policyData(rows);
  renderBar('sim-policy-chart', {
    data: policies,
    indexBy: 'policy',
    keys: ['Survive', 'Death'],
    groupMode: 'stacked',
    margin: { top: 8, right: 12, bottom: 40, left: 70 },
    padding: 0.25,
    valueScale: { type: 'linear', min: 0, max: 100 },
    indexScale: { type: 'band', round: true },
    colors: [cssVar('--pos', '#4ade80'), cssVar('--neg', '#f87171')],
    borderRadius: 1,
    enableLabel: false,
    axisTop: null,
    axisRight: null,
    axisBottom: { tickSize: 3, tickPadding: 5, tickRotation: -25 },
    axisLeft: {
      tickSize: 3,
      tickPadding: 5,
      legend: 'percent',
      legendPosition: 'middle',
      legendOffset: -52,
    },
    legends: [
      {
        dataFrom: 'keys',
        anchor: 'bottom-right',
        direction: 'row',
        justify: false,
        translateX: 0,
        translateY: 36,
        itemsSpacing: 12,
        itemWidth: 64,
        itemHeight: 12,
        itemTextColor: cssVar('--text-muted', '#71717a'),
        symbolSize: 10,
        symbolShape: 'square',
      },
    ],
    theme,
    role: 'application',
    ariaLabel: 'Policy survival mix chart',
  });

  const turns = turnData(rows);
  renderBar('sim-turn-chart', {
    data: turns,
    indexBy: 'turn',
    keys: ['Completed', 'Death'],
    groupMode: 'stacked',
    margin: { top: 8, right: 12, bottom: 40, left: 62 },
    padding: 0.15,
    valueScale: { type: 'linear' },
    indexScale: { type: 'band', round: true },
    colors: [cssVar('--pos', '#4ade80'), cssVar('--neg', '#f87171')],
    borderRadius: 1,
    enableLabel: false,
    axisTop: null,
    axisRight: null,
    axisBottom: { tickSize: 3, tickPadding: 5, tickRotation: 0, legend: 'turn' },
    axisLeft: {
      tickSize: 3,
      tickPadding: 5,
      legend: 'runs',
      legendPosition: 'middle',
      legendOffset: -45,
    },
    legends: [
      {
        dataFrom: 'keys',
        anchor: 'bottom-right',
        direction: 'row',
        justify: false,
        translateX: 0,
        translateY: 36,
        itemsSpacing: 12,
        itemWidth: 72,
        itemHeight: 12,
        itemTextColor: cssVar('--text-muted', '#71717a'),
        symbolSize: 10,
        symbolShape: 'square',
      },
    ],
    theme,
    role: 'application',
    ariaLabel: 'Turn distribution chart',
  });
}

window.addEventListener('pu:sim-data', event => {
  const rows = event?.detail?.rows || [];
  const endingTitles = event?.detail?.endingTitles || window.__PU_ENDING_TITLES || {};
  renderSimCharts(rows, endingTitles);
});

if (Array.isArray(window.__PU_SIM_ROWS)) {
  renderSimCharts(window.__PU_SIM_ROWS, window.__PU_ENDING_TITLES || {});
}
