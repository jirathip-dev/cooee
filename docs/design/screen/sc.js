const ST = new URLSearchParams(location.search).get('style') || 'modern';
const WHO = { res: ['Researcher', '--p'], chief: ['Chief', '--l'] };
const LOCK = '<svg class=lck viewBox="0 0 12 14" width="10" height="12"><rect x="1" y="6" width="10" height="7.5" rx="1.6" fill="currentColor"/><path d="M3.3 6V4.2a2.7 2.7 0 0 1 5.4 0V6" fill="none" stroke="currentColor" stroke-width="1.6"/></svg>';
const EYE = '<svg viewBox="0 0 24 24"><path d="M2 12s3.6-6.5 10-6.5S22 12 22 12s-3.6 6.5-10 6.5S2 12 2 12z"/><circle cx="12" cy="12" r="3"/></svg>';
const ARROW = '<svg viewBox="0 0 16 16"><path d="M2 1.5l11 6.2-4.7 1.2L6.4 13.6z"/></svg>';

const PEOPLE = [['Dana Ortiz', 'Chief Executive Officer'], ['Marcus Lee', 'Chief Financial Officer'], ['Priya Nair', 'VP, Sales'],
  ['Tom Weber', 'Chief Technology Officer'], ['Ines Kahl', 'General Counsel'], ['Ravi Shah', 'VP, Product'], ['Ana Lima', 'Head of Data'], ['Ken Ito', 'Controller']];
const company = tab => `<div class=s-card><div class=s-ban></div><div class=s-co><i></i><div><b>Kestrel Systems</b><span>Software · Austin, TX · 212 employees</span></div></div>
  <div class=s-tabs><span>Home</span><span>About</span><span class=${tab === 'people' ? 'on' : ''}>People</span><span>Jobs</span></div></div>`;
const ppl = blur => `<div class=s-ppl>${PEOPLE.map(([n, t]) => `<div class="s-p${blur ? ' blur' : ''}"><i></i><b>${n}</b><span>${t}</span></div>`).join('')}</div>`;
const TOP = '<div class=s-top><b class=s-logo>in</b><span class=s-q>Search</span><span class=s-nav>Home Network Jobs Messaging</span></div>';
const PAGES = {
  'li-people': () => `${TOP}<div class=s-body>${company('people')}${ppl(false)}</div>`,
  'li-wall': () => `${TOP}<div class=s-body>${company('people')}<div class=s-wall>${ppl(true)}<div class=s-ov><b>Sign in to see who works at Kestrel</b><span class=s-btn>Sign in</span></div></div></div>`,
  'li-login': () => `<div class=s-top><b class=s-logo>in</b></div><div class=s-login><b>Sign in</b><span>Stay up to date on your professional world</span>
    <label>Email or phone</label><div class=s-in>guy@example.com</div><label>Password</label><div class="s-in on">•••••••••••</div><span class=s-btn>Sign in</span></div>`,
  sec: () => `<div class=s-top><b class=s-logo style="background:#1b3a63">SEC</b><span class=s-q>Company search</span><span class=s-nav>Filings Rules Enforcement</span></div>
    <div class=s-sec><h1>Kestrel Systems, Inc. (KSTL)</h1><span>CIK 0001874420 · State of Inc.: DE · Fiscal year end: 12/31</span>
    <div class=s-tb><span class=h>Form</span><span class=h>Description</span><span class=h>Filed</span><span class=h>Period</span>
    <span class=k>8-K</span><span>Current report, items 2.02 and 9.01</span><span>2026-11-06</span><span>—</span>
    <span class="k hl">10-Q</span><span class=hl>Quarterly report, Q3 2026</span><span class=hl>2026-11-06</span><span class=hl>2026-09-30</span>
    <span class=k>10-Q</span><span>Quarterly report, Q2 2026</span><span>2026-08-07</span><span>2026-06-30</span>
    <span class=k>DEF 14A</span><span>Proxy statement</span><span>2026-04-18</span><span>—</span>
    <span class=k>10-K</span><span>Annual report</span><span>2026-02-27</span><span>2025-12-31</span></div></div>`,
  cb: () => `<div class=s-top><b class=s-logo style="background:#146aff">cb</b><span class=s-q>Search Crunchbase</span></div>
    <div class=s-cb><div class=s-card><span>Total funding</span><b>$148.5M</b><span>4 rounds · last: Series C, Mar 2026</span></div>
    <div class=s-card><span>Lead investors</span><b>Birch &amp; Co</b><span>Northwind Ventures · Alder Partners</span></div>
    <div class=s-card><span>Employees</span><b>201–500</b><span>Austin, Texas</span></div><div class=s-card><span>Acquisitions</span><b>1</b><span>Tally Labs, 2025</span></div></div>`,
};

function site(e) {
  const d = e.dataset, w = +d.w, z = +(d.z || 1), h = +(d.h || Math.round(w * 0.625));
  const [cx, cy] = (d.crop || '0,0').split(',').map(Number);
  const pin = (xy, cls, who) => {
    const [x, y] = xy.split(',');
    const [name, t] = who ? WHO[who] : ['You', '--a'];
    return `<i class=tap style="left:${x}%;top:${y}%;--t:var(${t})"></i><span class="xcur${who ? '' : ' you'}" style="left:${x}%;top:${y}%;--t:var(${t})">${ARROW}<b>${ST === 'terminal' ? `[${name}]` : name}</b></span>`;
  };
  e.innerHTML = (d.bare ? '' : `<div class=ubar>${LOCK}<span><b>${d.url.split('/')[0]}</b>${d.url.slice(d.url.indexOf('/'))}</span><i class=xst>${d.st}</i></div>`)
    + `<div class=xvw style="height:${h}px"><div class=site style="zoom:${(w / 1280) * z};margin:${-cy}px 0 0 ${-cx}px">${PAGES[d.pg]()}</div>`
    + (d.cur ? pin(d.cur, 'cur', d.who) : '') + (d.you ? pin(d.you, 'cur') : '') + (d.rpl ? `<span class=rpl>${d.rpl}</span>` : '') + '</div>';
}

const FS = {
  res: [['09:40', 'sec.gov', 'sec'], ['09:41', 'the 10-Q', 'sec', 'snt'], ['09:43', 'Crunchbase', 'cb'], ['09:44', 'sign-in wall', 'li-wall', 'snt'],
    ['09:51', 'you, signing in', 'li-login', 'you'], ['09:53', 'People', 'li-people', 'live']],
  chief: [['10:02', 'Crunchbase', 'cb'], ['10:04–10:11', 'kasikornbank.com', null, 'gap'], ['10:12', 'sec.gov', 'sec']],
};
function fs(e) {
  const tw = e.classList.contains('xbig') ? 104 : 80, sel = +(e.dataset.sel ?? -1), from = +(e.dataset.from || 0);
  return FS[e.dataset.set].map(([t, a, k, m], i) => {
    if (i < from || i >= from + +(e.dataset.n || 9)) return '';
    const cls = ['xfi', m === 'live' ? 'live' : '', i === sel ? 'sel' : '', m === 'gap' ? 'gap' : ''].join(' ').trim();
    const th = m === 'gap' ? '<span class=xth>no frames</span>'
      : `<span class=xth><span class=site style="zoom:${tw / 1280}">${PAGES[k]()}</span>${m === 'snt' ? `<i class=snt title="sent to the persona">${EYE}</i>` : ''}</span>`;
    const lab = m === 'live' ? 'live' : m === 'you' ? `<span class=xwt>${t}</span>` : t;
    return `<a class="${cls}" href="#" data-k=replying>${th}<span><b>${lab}</b></span><span>${a}</span></a>`;
  }).join('');
}
for (const e of document.querySelectorAll('[data-y=site]')) site(e);
for (const e of document.querySelectorAll('[data-y=fs]')) e.innerHTML = fs(e);
